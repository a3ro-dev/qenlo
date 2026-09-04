//! Observable embedded filtered vector search.
//!
//! Default features contain only the portable exact CPU backend. Applications
//! opt into C++ (`usearch`) and GPU (`gpu-wgpu`) build requirements explicitly.
//!
//! ```no_run
//! # async fn example() -> Result<(), qenlo::Error> {
//! use qenlo::{Collection, CollectionConfig, Filter, NewRecord};
//! let collection = Collection::create("vectors.qenlo", CollectionConfig::cpu_exact(2)).await?;
//! collection.add_batch(&[NewRecord {
//!     id: 1, user_id: 7, timestamp: -1, vector: vec![1.0, 0.0],
//! }])?;
//! let found = collection.search(&[1.0, 0.0], &Filter::ALL, 10).await?;
//! assert_eq!(found.results[0].id, 1);
//! collection.close()?;
//! let reopened = Collection::open("vectors.qenlo", CollectionConfig::cpu_exact(2)).await?;
//! assert_eq!(reopened.stats().live_rows, 1);
//! reopened.close()?;
//! # Ok(())
//! # }
//! ```

use async_lock::{RwLock, RwLockWriteGuard};
#[cfg(feature = "usearch")]
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
#[cfg(feature = "gpu-wgpu")]
use std::sync::{Arc, Mutex, RwLock as SyncRwLock};

#[cfg(feature = "usearch")]
use qenlo_core::Predicate;
use qenlo_core::{CoreStore, Error as CoreError, Mutation as CoreMutation, SearchHit};
use thiserror::Error;
use tracing::{Instrument, info_span};
use web_time::{Duration, Instant};

#[cfg(feature = "gpu-wgpu")]
mod gpu;
mod index_state;
mod storage;
#[cfg(feature = "usearch")]
mod usearch_backend;

#[cfg(feature = "gpu-wgpu")]
pub use gpu::GpuCapabilities;
pub use qenlo_core::CpuDistancePath;
pub use qenlo_core::Record;
pub use qenlo_core::{Predicate as Filter, TimestampRange};

/// Maximum supported result count for this prototype.
pub const MAX_K: usize = 64;
/// Default cap for all Qenlo-owned GPU allocations, including scratch buffers.
pub const DEFAULT_GPU_BUDGET_BYTES: u64 = 512 * 1024 * 1024;
#[cfg(feature = "gpu-wgpu")]
const AUTOMATIC_GPU_MIN_ELIGIBLE_ROWS: usize = 4_096;

/// Requested execution policy. Required backends error; automatic GPU mode reports CPU fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendSelection {
    CpuExact,
    #[cfg(feature = "usearch")]
    Usearch,
    #[cfg(feature = "gpu-wgpu")]
    WgpuRequired(GpuFilterMode),
    /// Try GPU and disclose a CPU fallback if initialization or allocation fails.
    #[cfg(feature = "gpu-wgpu")]
    Automatic(GpuFilterMode),
}

/// Backend that actually completed a search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Cpu,
    Usearch,
    Wgpu,
}

/// Search algorithm used, independent of hardware selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    Exact,
    Hnsw,
    IvfFlat,
    IvfSq8,
}

/// Exact GPU eligibility strategy; all modes obey the same canonical predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuFilterMode {
    CpuMask,
    CpuEligibleRows,
    GpuPredicate,
}

/// Host-side eligible-row preparation used by exact WGPU row and mask filters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum GpuRowPreparation {
    /// Preserve the original count-then-materialize behavior for controlled ablations.
    LegacyTwoPass,
    /// Materialize eligible rows once and reuse them for routing and execution.
    #[default]
    OnePass,
    /// Reuse one bounded eligible-row list for the current generation and predicate.
    Cached,
}

/// One conservative, hardware-bound automatic-routing decision boundary.
///
/// Profiles are produced from tuning data, never from held-out evaluation data.
/// A mismatch falls back to Qenlo's documented static rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterProfile {
    pub adapter_name: String,
    pub dimension: usize,
    pub batch_size: usize,
    pub filter_mode: GpuFilterMode,
    pub cached_rows: bool,
    pub gpu_min_eligible_rows: usize,
}

/// Canonical predicate shape compiled into an eligibility plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EligibilityPredicateKind {
    All,
    UserEquality,
    TimestampRange,
    UserAndTimestamp,
    Empty,
}

/// Physical eligibility representation selected before backend execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EligibilityRepresentation {
    Empty,
    TinyRows,
    SortedRows,
    DenseMask,
    ShaderPredicate,
}

/// Where row eligibility was evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterExecution {
    OrderedMetadataIndexes,
    GraphPredicate,
    Gpu(GpuFilterMode),
}

/// Portable construction settings. Default features require neither GPU nor C++.
#[derive(Debug, Clone)]
pub struct CollectionConfig {
    pub dimension: usize,
    pub backend: BackendSelection,
    pub gpu_allocation_budget_bytes: u64,
}

/// Admission budget for canonical loading and durable writes (not measured RSS).
/// Allows vector payload plus 512 bytes per row for indexes and bookkeeping.
#[derive(Debug, Clone, Copy)]
pub struct StorageOptions {
    pub max_load_bytes: u64,
}

/// When a stale derived index may be rebuilt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RebuildPolicy {
    #[default]
    OnSearch,
    Explicit,
}

/// Why preparation is needed; contains no row or predicate data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationReason {
    Initial,
    Mutation,
    Restart,
    MissingIndex,
    CorruptIndex,
    StaleIndex,
    DeviceRecovery,
}

/// Tracing detail. Base reports remain available with instrumentation disabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Diagnostics {
    Disabled,
    Basic,
    Detailed,
}

static NEXT_OPERATION_ID: AtomicU64 = AtomicU64::new(1);

impl Default for StorageOptions {
    fn default() -> Self {
        Self {
            max_load_bytes: storage::MAX_LOAD_BYTES,
        }
    }
}

impl CollectionConfig {
    pub fn cpu_exact(dimension: usize) -> Self {
        Self {
            dimension,
            backend: BackendSelection::CpuExact,
            gpu_allocation_budget_bytes: DEFAULT_GPU_BUDGET_BYTES,
        }
    }
}

/// A public ID and computed cosine distance, ordered by distance then ID.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub id: u64,
    pub distance: f32,
}

/// Why a measurement was not collected; absence is never a fabricated zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unavailable {
    pub reason: String,
}

/// A measured value or an explicit reason it is unavailable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Measurement<T> {
    Available(T),
    Unavailable(Unavailable),
}

impl<T> Measurement<T> {
    fn unavailable(reason: impl Into<String>) -> Self {
        Self::Unavailable(Unavailable {
            reason: reason.into(),
        })
    }
}

/// Host-side phase durations. GPU device timestamps are not currently collected.
#[derive(Debug, Clone)]
pub struct PhaseTimings {
    pub preparation: Measurement<Duration>,
    pub filtering: Measurement<Duration>,
    pub upload: Measurement<Duration>,
    pub execution: Measurement<Duration>,
    pub readback: Measurement<Duration>,
    pub selection: Measurement<Duration>,
}

impl PhaseTimings {
    fn cpu(preparation: Measurement<Duration>, execution: Duration) -> Self {
        Self {
            preparation,
            filtering: Measurement::unavailable("included in exact CPU execution"),
            upload: Measurement::unavailable("no device transfer"),
            execution: Measurement::Available(execution),
            readback: Measurement::unavailable("no device readback"),
            selection: Measurement::unavailable("included in exact CPU execution"),
        }
    }
}

/// Privacy-safe execution facts for one completed search; total includes preparation and locks.
#[derive(Debug, Clone)]
pub struct ExecutionReport {
    /// Process-local correlation ID attached to the search span.
    pub operation_id: u64,
    pub cpu_distance_path: Option<CpuDistancePath>,
    pub eligible_rows: Measurement<u64>,
    pub last_commit: Option<CommitReport>,
    pub preparation_reason: Option<PreparationReason>,
    /// Time waiting for collection locks, included in `total_duration`.
    pub lock_wait: Duration,
    /// Effective `(connectivity, expansion_add, expansion_search)` for ANN.
    pub ann_parameters: Option<(usize, usize, usize)>,
    pub requested_backend: BackendSelection,
    pub actual_backend: BackendKind,
    pub algorithm: Algorithm,
    pub filter_execution: FilterExecution,
    pub index_generation: u64,
    pub rebuilt: bool,
    /// Why automatic routing selected the backend; absent for required backends.
    pub routing_reason: Option<String>,
    pub fallback_reason: Option<String>,
    pub total_duration: Duration,
    pub phases: PhaseTimings,
    pub upload_bytes: Measurement<u64>,
    pub readback_bytes: Measurement<u64>,
    pub dispatch_count: Measurement<u32>,
    pub qenlo_allocation_bytes: Measurement<u64>,
    pub candidates: Measurement<u64>,
    /// Eligible-row preparation mode for WGPU row/mask execution.
    pub gpu_row_preparation: Option<GpuRowPreparation>,
    /// Complete predicate traversals performed before or during this search.
    pub predicate_traversals: u32,
    /// Time spent materializing the eligible-row list on the host.
    pub row_materialization: Measurement<Duration>,
    /// Rows in the materialized list, distinct from algorithm candidates.
    pub materialized_rows: Measurement<u64>,
    /// `Some(true)` for a cache hit, `Some(false)` for a miss, otherwise absent.
    pub row_cache_hit: Option<bool>,
    /// Predicate shape and physical representation chosen by `EligibilityPlan`.
    pub eligibility_predicate_kind: Option<EligibilityPredicateKind>,
    pub eligibility_representation: Option<EligibilityRepresentation>,
    /// Generation and corpus size against which eligibility was compiled.
    pub eligibility_generation: Option<u64>,
    pub corpus_rows: Measurement<u64>,
    pub eligible_selectivity: Measurement<f64>,
    /// Estimated eligibility bytes uploaded by the selected representation.
    pub eligibility_transfer_bytes: Measurement<u64>,
    /// Number of contiguous runs in the sorted eligible-row sequence.
    pub eligible_contiguous_runs: Measurement<u64>,
    pub eligibility_cacheable: Option<bool>,
    pub eligibility_resident: Option<bool>,
    pub results: usize,
    /// Queries sharing this execution. Transfer/dispatch metrics are batch totals.
    pub batch_size: usize,
}

/// Ordered results and the work report for a single committed generation.
#[derive(Debug, Clone)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub report: ExecutionReport,
}

/// An owned input row. Its vector is validated and normalized on commit.
#[derive(Clone, Debug)]
pub struct NewRecord {
    pub id: u64,
    pub user_id: u64,
    pub timestamp: i64,
    pub vector: Vec<f32>,
}

/// One ordered operation in an atomic batch. Deleted IDs cannot be reused.
#[derive(Clone, Debug)]
pub enum Mutation {
    Add(NewRecord),
    Delete(u64),
}

/// A committed mutation batch. In-memory collections have no durable generation.
#[derive(Clone, Debug)]
pub struct CommitReport {
    pub operation_id: u64,
    pub lock_wait: Duration,
    pub generation: u64,
    pub durable_generation: Option<u64>,
    pub mutations: usize,
    pub total_duration: Duration,
    pub persistence: Measurement<Duration>,
}

/// Canonical, durable, and derived readiness state; contains no row payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionStats {
    /// Disposable on-disk readiness generation, not a loaded index.
    pub persisted_index_generation: Option<u64>,
    pub preparation_reason: PreparationReason,
    pub index_persistence: Measurement<Duration>,
    pub dimension: usize,
    pub rows: usize,
    pub live_rows: usize,
    pub generation: u64,
    pub prepared_generation: Option<u64>,
    pub durable_generation: Option<u64>,
    pub recovered_interrupted_write: bool,
    pub closed: bool,
}

/// Explicit validation, lifecycle, storage, and backend failures.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("k must be in 1..={MAX_K}, got {0}")]
    InvalidK(usize),
    #[error("requested backend is not enabled: {0}")]
    BackendNotEnabled(&'static str),
    #[error("backend preparation failed: {0}")]
    Preparation(String),
    #[error("backend search failed: {0}")]
    Search(String),
    #[error("collection storage failed: {0}")]
    Storage(String),
    #[error("collection is closed")]
    Closed,
    #[error("commit outcome is uncertain; reopen the collection: {0}")]
    CommitUncertain(String),
    #[error("derived index is not prepared; call prepare() or use RebuildPolicy::OnSearch")]
    IndexNotPrepared,
    #[error("invalid IVF configuration: lists and nprobe must satisfy 1 <= nprobe <= lists <= 64")]
    InvalidIvfConfig,
}

enum Backend {
    Cpu,
    #[cfg(feature = "usearch")]
    Usearch(usearch_backend::UsearchBackend),
    #[cfg(feature = "gpu-wgpu")]
    Wgpu(Box<gpu::GpuBackend>),
}

/// Embedded collection shared with `Arc<Collection>` across threads.
///
/// Ready CPU and USearch searches share a read lock. Mutations and rebuilds
/// take a write lock. GPU queries additionally serialize their scratch buffers.
/// Synchronous mutation/inspection methods block; use a blocking thread when
/// contending with async GPU work on a single-thread executor. No workers run.
pub struct Collection {
    diagnostics: AtomicU8,
    inner: RwLock<CollectionState>,
    #[cfg(feature = "gpu-wgpu")]
    gpu_gate: async_lock::Mutex<()>,
    #[cfg(feature = "gpu-wgpu")]
    gpu_row_preparation: AtomicU8,
    #[cfg(feature = "gpu-wgpu")]
    gpu_row_cache: Mutex<Option<CachedGpuRows>>,
    #[cfg(feature = "gpu-wgpu")]
    router_profile: SyncRwLock<Option<RouterProfile>>,
}

#[cfg(feature = "gpu-wgpu")]
struct CachedGpuRows {
    generation: u64,
    filter: Filter,
    rows: Arc<[u32]>,
}

#[cfg(feature = "gpu-wgpu")]
struct EligibilityPlan {
    generation: u64,
    eligible_count: usize,
    corpus_size: usize,
    predicate_kind: EligibilityPredicateKind,
    representation: EligibilityRepresentation,
    rows: Option<Arc<[u32]>>,
    transfer_bytes: u64,
    contiguous_runs: usize,
    materialization: Measurement<Duration>,
    predicate_traversals: u32,
    cache_hit: Option<bool>,
}

struct SearchBatchContext<'a> {
    rebuilt: bool,
    preparation: Measurement<Duration>,
    #[cfg(feature = "gpu-wgpu")]
    row_preparation: GpuRowPreparation,
    #[cfg(feature = "gpu-wgpu")]
    row_cache: &'a Mutex<Option<CachedGpuRows>>,
    #[cfg(feature = "gpu-wgpu")]
    router_profile: Option<RouterProfile>,
    #[cfg(not(feature = "gpu-wgpu"))]
    marker: std::marker::PhantomData<&'a ()>,
}

#[cfg(feature = "gpu-wgpu")]
impl EligibilityPlan {
    fn compile(
        store: &CoreStore,
        filter: &Filter,
        mode: GpuFilterMode,
        preparation: GpuRowPreparation,
        cache: &Mutex<Option<CachedGpuRows>>,
    ) -> Self {
        let generation = store.generation();
        let corpus_size = store.len();
        let predicate_kind = eligibility_predicate_kind(filter);
        let uses_host_rows = matches!(
            mode,
            GpuFilterMode::CpuMask | GpuFilterMode::CpuEligibleRows
        );
        let materialization;
        let mut cache_hit = None;
        let mut predicate_traversals = 1;
        let rows = if uses_host_rows && preparation == GpuRowPreparation::Cached {
            let cached = cache
                .lock()
                .expect("GPU row cache poisoned")
                .as_ref()
                .filter(|entry| entry.generation == generation && entry.filter == *filter)
                .map(|entry| Arc::clone(&entry.rows));
            if let Some(rows) = cached {
                predicate_traversals = 0;
                cache_hit = Some(true);
                materialization = Measurement::Available(Duration::ZERO);
                rows
            } else {
                let started = Instant::now();
                let rows: Arc<[u32]> = store.filter(filter).into();
                materialization = Measurement::Available(started.elapsed());
                cache_hit = Some(false);
                *cache.lock().expect("GPU row cache poisoned") = Some(CachedGpuRows {
                    generation,
                    filter: *filter,
                    rows: Arc::clone(&rows),
                });
                rows
            }
        } else {
            if uses_host_rows && preparation == GpuRowPreparation::LegacyTwoPass {
                // Deliberately preserve the shipped count pass for controlled ablations only.
                let _ = store.filter(filter).len();
                predicate_traversals = 2;
            }
            let started = Instant::now();
            let rows: Arc<[u32]> = store.filter(filter).into();
            materialization = Measurement::Available(started.elapsed());
            rows
        };
        let eligible_count = rows.len();
        let contiguous_runs = contiguous_run_count(&rows);
        let representation = if eligible_count == 0 {
            EligibilityRepresentation::Empty
        } else {
            match mode {
                GpuFilterMode::CpuMask => EligibilityRepresentation::DenseMask,
                GpuFilterMode::CpuEligibleRows if eligible_count <= 64 => {
                    EligibilityRepresentation::TinyRows
                }
                GpuFilterMode::CpuEligibleRows => EligibilityRepresentation::SortedRows,
                GpuFilterMode::GpuPredicate => EligibilityRepresentation::ShaderPredicate,
            }
        };
        let transfer_bytes = match representation {
            EligibilityRepresentation::Empty => 0,
            EligibilityRepresentation::TinyRows | EligibilityRepresentation::SortedRows => {
                eligible_count as u64 * 4
            }
            EligibilityRepresentation::DenseMask => corpus_size as u64 * 4,
            EligibilityRepresentation::ShaderPredicate => 0,
        };
        Self {
            generation,
            eligible_count,
            corpus_size,
            predicate_kind,
            representation,
            // Retain canonical rows even for shader execution so an automatic CPU decision or
            // GPU fallback can consume the same traversal without recompiling eligibility.
            rows: Some(rows),
            transfer_bytes,
            contiguous_runs,
            materialization,
            predicate_traversals,
            cache_hit,
        }
    }

    fn annotate(
        &self,
        output: &mut BackendOutput,
        row_preparation: GpuRowPreparation,
        uses_host_rows: bool,
    ) {
        output.gpu_row_preparation = uses_host_rows.then_some(row_preparation);
        output.predicate_traversals = self.predicate_traversals;
        output.row_materialization = self.materialization.clone();
        output.materialized_rows = Measurement::Available(self.eligible_count as u64);
        output.row_cache_hit = uses_host_rows.then_some(self.cache_hit).flatten();
        output.eligibility_predicate_kind = Some(self.predicate_kind);
        output.eligibility_representation = Some(self.representation);
        output.eligibility_generation = Some(self.generation);
        output.corpus_rows = Measurement::Available(self.corpus_size as u64);
        output.eligible_selectivity = Measurement::Available(if self.corpus_size == 0 {
            0.0
        } else {
            self.eligible_count as f64 / self.corpus_size as f64
        });
        output.eligibility_transfer_bytes = Measurement::Available(self.transfer_bytes);
        output.eligible_contiguous_runs = Measurement::Available(self.contiguous_runs as u64);
        output.eligibility_cacheable = Some(uses_host_rows);
        output.eligibility_resident = Some(false);
    }
}

#[cfg(feature = "gpu-wgpu")]
fn eligibility_predicate_kind(filter: &Filter) -> EligibilityPredicateKind {
    if filter.timestamp.is_empty() {
        EligibilityPredicateKind::Empty
    } else {
        match (
            filter.user_id.is_some(),
            filter.timestamp == TimestampRange::ALL,
        ) {
            (false, true) => EligibilityPredicateKind::All,
            (true, true) => EligibilityPredicateKind::UserEquality,
            (false, false) => EligibilityPredicateKind::TimestampRange,
            (true, false) => EligibilityPredicateKind::UserAndTimestamp,
        }
    }
}

#[cfg(feature = "gpu-wgpu")]
fn contiguous_run_count(rows: &[u32]) -> usize {
    rows.first().map_or(0, |_| {
        1 + rows
            .windows(2)
            .filter(|pair| pair[1] != pair[0] + 1)
            .count()
    })
}

impl Collection {
    /// Construct an in-memory collection; use `create` for durable commits.
    pub async fn new(config: CollectionConfig) -> Result<Self, Error> {
        Ok(Self::from_state(CollectionState::new(config).await?))
    }

    /// Create a collection in an empty directory, held under an exclusive OS lock.
    pub async fn create(path: impl AsRef<Path>, config: CollectionConfig) -> Result<Self, Error> {
        Ok(Self::from_state(
            CollectionState::create(path, config).await?,
        ))
    }

    /// Recover a durable collection. A second open handle is rejected.
    pub async fn open(path: impl AsRef<Path>, config: CollectionConfig) -> Result<Self, Error> {
        Ok(Self::from_state(CollectionState::open(path, config).await?))
    }

    /// Import a checksummed portable `.qn` file into a mutable in-memory collection.
    ///
    /// The imported file is not modified. Call [`Self::export_qn`] to persist later changes.
    pub async fn import_qn(
        path: impl AsRef<Path>,
        config: CollectionConfig,
    ) -> Result<Self, Error> {
        Ok(Self::from_state(
            CollectionState::import_qn_with_options(path, config, StorageOptions::default())
                .await?,
        ))
    }

    /// Create with an explicit canonical-memory admission budget.
    pub async fn create_with_options(
        path: impl AsRef<Path>,
        config: CollectionConfig,
        options: StorageOptions,
    ) -> Result<Self, Error> {
        Ok(Self::from_state(
            CollectionState::create_with_options(path, config, options).await?,
        ))
    }

    /// Open with an explicit budget, for collections above the 512 MiB default.
    pub async fn open_with_options(
        path: impl AsRef<Path>,
        config: CollectionConfig,
        options: StorageOptions,
    ) -> Result<Self, Error> {
        Ok(Self::from_state(
            CollectionState::open_with_options(path, config, options).await?,
        ))
    }

    /// Import a portable `.qn` file with an explicit canonical-memory admission budget.
    pub async fn import_qn_with_options(
        path: impl AsRef<Path>,
        config: CollectionConfig,
        options: StorageOptions,
    ) -> Result<Self, Error> {
        Ok(Self::from_state(
            CollectionState::import_qn_with_options(path, config, options).await?,
        ))
    }

    fn from_state(state: CollectionState) -> Self {
        Self {
            diagnostics: AtomicU8::new(Diagnostics::Basic as u8),
            inner: RwLock::new(state),
            #[cfg(feature = "gpu-wgpu")]
            gpu_gate: async_lock::Mutex::new(()),
            #[cfg(feature = "gpu-wgpu")]
            gpu_row_preparation: AtomicU8::new(GpuRowPreparation::OnePass as u8),
            #[cfg(feature = "gpu-wgpu")]
            gpu_row_cache: Mutex::new(None),
            #[cfg(feature = "gpu-wgpu")]
            router_profile: SyncRwLock::new(None),
        }
    }

    /// Validate and add one row; durable collections sync before returning.
    pub fn add(&self, id: u64, user_id: u64, timestamp: i64, vector: &[f32]) -> Result<(), Error> {
        let operation_id = NEXT_OPERATION_ID.fetch_add(1, Ordering::Relaxed);
        let _span = self.operation_span("add", operation_id).entered();
        let started = Instant::now();
        let mut state = self.inner.write_blocking();
        let lock_wait = started.elapsed();
        state.add(id, user_id, timestamp, vector)?;
        state.record_single_commit(operation_id, lock_wait, started.elapsed());
        #[cfg(feature = "gpu-wgpu")]
        self.invalidate_gpu_row_cache();
        Ok(())
    }

    /// Delete one row immediately and durably; IDs are never reused.
    pub fn delete(&self, id: u64) -> Result<(), Error> {
        let operation_id = NEXT_OPERATION_ID.fetch_add(1, Ordering::Relaxed);
        let _span = self.operation_span("delete", operation_id).entered();
        let started = Instant::now();
        let mut state = self.inner.write_blocking();
        let lock_wait = started.elapsed();
        state.delete(id)?;
        state.record_single_commit(operation_id, lock_wait, started.elapsed());
        #[cfg(feature = "gpu-wgpu")]
        self.invalidate_gpu_row_cache();
        Ok(())
    }

    /// Commit ordered add/delete operations atomically. Errors roll back before publication.
    pub fn commit(&self, mutations: &[Mutation]) -> Result<CommitReport, Error> {
        let operation_id = NEXT_OPERATION_ID.fetch_add(1, Ordering::Relaxed);
        let _span = self.operation_span("commit", operation_id).entered();
        let started = Instant::now();
        let mut state = self.inner.write_blocking();
        let lock_wait = started.elapsed();
        let mut report = state.commit(mutations)?;
        report.operation_id = operation_id;
        report.lock_wait = lock_wait;
        report.total_duration = started.elapsed();
        state.last_commit = Some(report.clone());
        #[cfg(feature = "gpu-wgpu")]
        self.invalidate_gpu_row_cache();
        Ok(report)
    }

    /// Add all rows atomically, rejecting duplicate IDs and invalid vectors.
    pub fn add_batch(&self, rows: &[NewRecord]) -> Result<CommitReport, Error> {
        self.commit(&rows.iter().cloned().map(Mutation::Add).collect::<Vec<_>>())
    }

    /// Delete all IDs atomically; any invalid deletion rolls back the batch.
    pub fn delete_batch(&self, ids: &[u64]) -> Result<CommitReport, Error> {
        self.commit(
            &ids.iter()
                .copied()
                .map(Mutation::Delete)
                .collect::<Vec<_>>(),
        )
    }

    /// Return live eligible IDs. Retains the original empty-on-closed behavior.
    pub fn filter(&self, filter: &Filter) -> Vec<u64> {
        self.inner.read_blocking().filter(filter)
    }

    /// Return the dimension configured for this collection.
    pub fn dimension(&self) -> usize {
        self.inner.read_blocking().store.dimension()
    }

    /// Retrieve a single canonical record by ID, including tombstones.
    pub fn get_record(&self, id: u64) -> Option<Record> {
        self.inner.read_blocking().store.get(id).cloned()
    }

    /// Return a paginated slice of records matching an optional filter, along with total matching count.
    pub fn scan_records(
        &self,
        offset: usize,
        limit: usize,
        filter: Option<&Filter>,
    ) -> (Vec<Record>, usize) {
        let state = self.inner.read_blocking();
        match filter {
            Some(f) => {
                let slots = state.store.filter(f);
                let total = slots.len();
                let records = slots
                    .into_iter()
                    .skip(offset)
                    .take(limit)
                    .filter_map(|slot| state.store.record(slot).cloned())
                    .collect();
                (records, total)
            }
            None => {
                let total = state.store.len();
                let records = state
                    .store
                    .records()
                    .skip(offset)
                    .take(limit)
                    .map(|(_, r)| r.clone())
                    .collect();
                (records, total)
            }
        }
    }

    /// Explicitly prepare the current generation, serialized with all mutations.
    pub async fn prepare(&self) -> Result<bool, Error> {
        self.inner.write().await.prepare().await
    }

    /// Search one committed generation. A concurrent commit is visible entirely or not at all.
    pub async fn search(
        &self,
        query: &[f32],
        filter: &Filter,
        k: usize,
    ) -> Result<SearchResponse, Error> {
        let operation_id = NEXT_OPERATION_ID.fetch_add(1, Ordering::Relaxed);
        let mut response = self
            .search_locked(query, filter, k)
            .instrument(self.operation_span("search", operation_id))
            .await?;
        response.report.operation_id = operation_id;
        Ok(response)
    }

    async fn search_locked(
        &self,
        query: &[f32],
        filter: &Filter,
        k: usize,
    ) -> Result<SearchResponse, Error> {
        let started = Instant::now();
        let mut state = self.inner.read().await;
        let mut lock_wait = started.elapsed();
        state.ensure_open()?;
        if !(1..=MAX_K).contains(&k) {
            return Err(Error::InvalidK(k));
        }
        qenlo_core::normalize_vector(query, state.store.dimension())?;
        let mut rebuilt = false;
        let mut preparation_reason = None;
        let mut preparation = Measurement::unavailable("index already current");
        if state.prepared_generation != Some(state.store.generation()) {
            if state.rebuild_policy == RebuildPolicy::Explicit {
                return Err(Error::IndexNotPrepared);
            }
            drop(state);
            let waiting = Instant::now();
            let mut writer = self.inner.write().await;
            lock_wait += waiting.elapsed();
            if writer.prepared_generation != Some(writer.store.generation())
                && writer.rebuild_policy == RebuildPolicy::Explicit
            {
                return Err(Error::IndexNotPrepared);
            }
            let preparing = Instant::now();
            preparation_reason = Some(writer.preparation_reason);
            rebuilt = writer.prepare().await?;
            if rebuilt {
                preparation = Measurement::Available(preparing.elapsed());
            }
            state = RwLockWriteGuard::downgrade(writer);
        }
        #[cfg(feature = "gpu-wgpu")]
        let _gpu_guard = if matches!(state.backend, Backend::Wgpu(_)) {
            let waiting = Instant::now();
            let guard = self.gpu_gate.lock().await;
            lock_wait += waiting.elapsed();
            Some(guard)
        } else {
            None
        };
        #[cfg(feature = "gpu-wgpu")]
        let router_profile = self
            .router_profile
            .read()
            .expect("router profile poisoned")
            .clone();
        let mut response = state
            .search_batch_inner(
                &[query],
                filter,
                k,
                SearchBatchContext {
                    rebuilt,
                    preparation,
                    #[cfg(feature = "gpu-wgpu")]
                    row_preparation: self.gpu_row_preparation(),
                    #[cfg(feature = "gpu-wgpu")]
                    row_cache: &self.gpu_row_cache,
                    #[cfg(feature = "gpu-wgpu")]
                    router_profile,
                    #[cfg(not(feature = "gpu-wgpu"))]
                    marker: std::marker::PhantomData,
                },
            )
            .await?
            .remove(0);
        response.report.lock_wait = lock_wait;
        response.report.preparation_reason = if rebuilt { preparation_reason } else { None };
        #[cfg(feature = "gpu-wgpu")]
        if rebuilt {
            if response.report.actual_backend == BackendKind::Wgpu {
                // Preparation uploads every stored vector and three u64 metadata columns.
                let resident_upload =
                    state.store.len() as u64 * (state.store.dimension() as u64 * 4 + 24);
                if let Measurement::Available(bytes) = &mut response.report.upload_bytes {
                    *bytes += resident_upload;
                }
            } else if matches!(state.config.backend, BackendSelection::Automatic(_))
                && response.report.fallback_reason.is_some()
            {
                response.report.upload_bytes =
                    Measurement::unavailable("GPU preparation failed; partial uploads unavailable");
                response.report.qenlo_allocation_bytes = Measurement::unavailable(
                    "GPU preparation failed; partial allocations unavailable",
                );
            }
        }
        response.report.total_duration = started.elapsed();
        if self.diagnostics.load(Ordering::Relaxed) == Diagnostics::Detailed as u8 {
            if matches!(response.report.eligible_rows, Measurement::Unavailable(_)) {
                response.report.eligible_rows =
                    Measurement::Available(state.store.filter(filter).len() as u64);
            }
            response.report.total_duration = started.elapsed();
        }
        Ok(response)
    }

    /// Search one committed generation. GPU-capable backends execute one native batch.
    pub async fn search_batch(
        &self,
        queries: &[&[f32]],
        filter: &Filter,
        k: usize,
    ) -> Result<Vec<SearchResponse>, Error> {
        if queries.is_empty() {
            return Ok(Vec::new());
        }
        let started = Instant::now();
        let mut state = self.inner.read().await;
        let mut lock_wait = started.elapsed();
        state.ensure_open()?;
        if !(1..=MAX_K).contains(&k) {
            return Err(Error::InvalidK(k));
        }
        for query in queries {
            qenlo_core::normalize_vector(query, state.store.dimension())?;
        }
        let mut rebuilt = false;
        let mut preparation = Measurement::unavailable("index already current");
        let mut preparation_reason = None;
        if state.prepared_generation != Some(state.store.generation()) {
            if state.rebuild_policy == RebuildPolicy::Explicit {
                return Err(Error::IndexNotPrepared);
            }
            drop(state);
            let waiting = Instant::now();
            let mut writer = self.inner.write().await;
            lock_wait += waiting.elapsed();
            let preparing = Instant::now();
            preparation_reason = Some(writer.preparation_reason);
            rebuilt = writer.prepare().await?;
            if rebuilt {
                preparation = Measurement::Available(preparing.elapsed());
            }
            state = RwLockWriteGuard::downgrade(writer);
        }
        #[cfg(feature = "gpu-wgpu")]
        let _gpu_guard = if matches!(state.backend, Backend::Wgpu(_)) {
            let waiting = Instant::now();
            let guard = self.gpu_gate.lock().await;
            lock_wait += waiting.elapsed();
            Some(guard)
        } else {
            None
        };
        #[cfg(feature = "gpu-wgpu")]
        let router_profile = self
            .router_profile
            .read()
            .expect("router profile poisoned")
            .clone();
        let mut responses = state
            .search_batch_inner(
                queries,
                filter,
                k,
                SearchBatchContext {
                    rebuilt,
                    preparation,
                    #[cfg(feature = "gpu-wgpu")]
                    row_preparation: self.gpu_row_preparation(),
                    #[cfg(feature = "gpu-wgpu")]
                    row_cache: &self.gpu_row_cache,
                    #[cfg(feature = "gpu-wgpu")]
                    router_profile,
                    #[cfg(not(feature = "gpu-wgpu"))]
                    marker: std::marker::PhantomData,
                },
            )
            .await?;
        for response in &mut responses {
            response.report.operation_id = NEXT_OPERATION_ID.fetch_add(1, Ordering::Relaxed);
            response.report.lock_wait = lock_wait;
            response.report.preparation_reason = if rebuilt { preparation_reason } else { None };
            response.report.total_duration = started.elapsed();
            response.report.batch_size = queries.len();
        }
        Ok(responses)
    }

    /// Inspect canonical and durable generations without scanning vectors.
    pub fn stats(&self) -> CollectionStats {
        self.inner.read_blocking().stats()
    }

    /// Change rebuild policy; explicit preparation is always allowed.
    pub fn set_rebuild_policy(&self, policy: RebuildPolicy) -> Result<(), Error> {
        let mut state = self.inner.write_blocking();
        state.ensure_open()?;
        state.rebuild_policy = policy;
        Ok(())
    }

    /// Select tracing detail; only `Detailed` adds an eligibility-count scan.
    pub fn set_diagnostics(&self, diagnostics: Diagnostics) {
        self.diagnostics.store(diagnostics as u8, Ordering::Relaxed);
    }

    /// Select eligible-row preparation for exact WGPU row and mask filters.
    #[cfg(feature = "gpu-wgpu")]
    pub fn set_gpu_row_preparation(&self, mode: GpuRowPreparation) {
        self.gpu_row_preparation
            .store(mode as u8, Ordering::Relaxed);
        *self.gpu_row_cache.lock().expect("GPU row cache poisoned") = None;
    }

    /// Install a tuning-only automatic router profile. It is ignored on mismatch.
    #[cfg(feature = "gpu-wgpu")]
    pub fn set_router_profile(&self, profile: Option<RouterProfile>) {
        *self
            .router_profile
            .write()
            .expect("router profile poisoned") = profile;
    }

    #[cfg(feature = "gpu-wgpu")]
    fn gpu_row_preparation(&self) -> GpuRowPreparation {
        match self.gpu_row_preparation.load(Ordering::Relaxed) {
            value if value == GpuRowPreparation::LegacyTwoPass as u8 => {
                GpuRowPreparation::LegacyTwoPass
            }
            value if value == GpuRowPreparation::Cached as u8 => GpuRowPreparation::Cached,
            _ => GpuRowPreparation::OnePass,
        }
    }

    #[cfg(feature = "gpu-wgpu")]
    fn invalidate_gpu_row_cache(&self) {
        *self.gpu_row_cache.lock().expect("GPU row cache poisoned") = None;
    }

    /// Enable portable IVF candidate generation with exact FP32 GPU re-ranking.
    /// The derived index is rebuilt on the next prepare/search and is never canonical.
    #[cfg(feature = "gpu-wgpu")]
    pub fn set_gpu_ivf(&self, lists: usize, nprobe: usize) -> Result<(), Error> {
        if lists == 0 || lists > 64 || nprobe == 0 || nprobe > lists {
            return Err(Error::InvalidIvfConfig);
        }
        let mut state = self.inner.write_blocking();
        state.ensure_open()?;
        let Backend::Wgpu(gpu) = &mut state.backend else {
            return Err(Error::BackendNotEnabled("gpu-wgpu"));
        };
        gpu.configure_ivf(lists, nprobe);
        state.prepared_generation = None;
        state.preparation_reason = PreparationReason::Mutation;
        Ok(())
    }

    /// Enable SQ8 coarse-centroid scoring plus exact FP32 GPU re-ranking.
    #[cfg(feature = "gpu-wgpu")]
    pub fn set_gpu_ivf_sq8(&self, lists: usize, nprobe: usize) -> Result<(), Error> {
        if lists == 0 || lists > 64 || nprobe == 0 || nprobe > lists {
            return Err(Error::InvalidIvfConfig);
        }
        let mut state = self.inner.write_blocking();
        state.ensure_open()?;
        let Backend::Wgpu(gpu) = &mut state.backend else {
            return Err(Error::BackendNotEnabled("gpu-wgpu"));
        };
        gpu.configure_ivf_sq8(lists, nprobe);
        state.prepared_generation = None;
        state.preparation_reason = PreparationReason::Mutation;
        Ok(())
    }

    fn operation_span(&self, operation: &'static str, operation_id: u64) -> tracing::Span {
        if self.diagnostics.load(Ordering::Relaxed) == Diagnostics::Disabled as u8 {
            tracing::Span::none()
        } else {
            info_span!("qenlo.operation", operation, operation_id)
        }
    }

    /// Sync pending canonical data. Normal durable mutations are already synced.
    pub fn flush(&self) -> Result<(), Error> {
        self.inner.write_blocking().flush()
    }

    /// Atomically export the current canonical generation as one portable `.qn` file.
    ///
    /// Existing targets are never overwritten. The portable file includes tombstones,
    /// generation, normalized vectors, fixed-width metadata, and a CRC32 checksum.
    pub fn export_qn(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let state = self.inner.read_blocking();
        state.ensure_open()?;
        storage::write_portable_with_limit(
            path.as_ref(),
            &state.store,
            state.storage_options.max_load_bytes,
        )
        .map_err(|error| Error::Storage(error.to_string()))
    }

    /// Flush, mark closed, and release the exclusive filesystem lock. Idempotent.
    pub fn close(&self) -> Result<(), Error> {
        self.inner.write_blocking().close()
    }

    /// Negotiated adapter capabilities without exposing backend-specific types.
    #[cfg(feature = "gpu-wgpu")]
    pub fn gpu_capabilities(&self) -> Option<GpuCapabilities> {
        match &self.inner.read_blocking().backend {
            Backend::Wgpu(gpu) => Some(gpu.capabilities().clone()),
            _ => None,
        }
    }

    /// Configure USearch's search expansion for this handle, including future rebuilds.
    #[cfg(feature = "usearch")]
    pub fn set_ann_search_expansion(&self, expansion: usize) -> Result<(), Error> {
        let mut state = self.inner.write_blocking();
        state.ensure_open()?;
        match &mut state.backend {
            Backend::Usearch(index) => index.set_search_expansion(expansion),
            _ => Err(Error::Preparation("collection does not use USearch".into())),
        }
    }
}

struct CollectionState {
    last_commit: Option<CommitReport>,
    rebuild_policy: RebuildPolicy,
    preparation_reason: PreparationReason,
    persisted_index_generation: Option<u64>,
    index_persistence: Measurement<Duration>,
    storage_options: StorageOptions,
    config: CollectionConfig,
    store: CoreStore,
    backend: Backend,
    prepared_generation: Option<u64>,
    fallback_reason: Option<String>,
    path: Option<PathBuf>,
    durable_generation: Option<u64>,
    recovered_interrupted_write: bool,
    closed: bool,
    storage_lock: Option<std::fs::File>,
}

impl CollectionState {
    #[tracing::instrument(name = "qenlo.initialize", skip_all, fields(backend = ?config.backend))]
    pub async fn new(config: CollectionConfig) -> Result<Self, Error> {
        let store = CoreStore::new(config.dimension)?;
        let (backend, fallback_reason) = Self::initialize_backend(&config).await?;
        Ok(Self {
            storage_options: StorageOptions::default(),
            last_commit: None,
            rebuild_policy: RebuildPolicy::OnSearch,
            preparation_reason: PreparationReason::Initial,
            persisted_index_generation: None,
            index_persistence: Measurement::unavailable("index not persisted"),
            config,
            store,
            backend,
            prepared_generation: None,
            fallback_reason,
            path: None,
            durable_generation: None,
            recovered_interrupted_write: false,
            closed: false,
            storage_lock: None,
        })
    }

    #[tracing::instrument(name = "qenlo.import_qn", skip_all, fields(backend = ?config.backend))]
    async fn import_qn_with_options(
        path: impl AsRef<Path>,
        config: CollectionConfig,
        options: StorageOptions,
    ) -> Result<Self, Error> {
        let opened = storage::read_portable_with_limit(path.as_ref(), options.max_load_bytes)
            .map_err(|error| Error::Storage(error.to_string()))?;
        let store = opened.store;
        if store.dimension() != config.dimension {
            return Err(Error::Storage(format!(
                "portable dimension mismatch: file has {}, config requests {}",
                store.dimension(),
                config.dimension
            )));
        }
        let (backend, fallback_reason) = Self::initialize_backend(&config).await?;
        Ok(Self {
            storage_options: options,
            last_commit: None,
            rebuild_policy: RebuildPolicy::OnSearch,
            preparation_reason: PreparationReason::Initial,
            persisted_index_generation: None,
            index_persistence: Measurement::unavailable("portable indexes are rebuilt locally"),
            config,
            store,
            backend,
            prepared_generation: None,
            fallback_reason,
            path: None,
            durable_generation: None,
            recovered_interrupted_write: opened.recovered_interrupted_write,
            closed: false,
            storage_lock: None,
        })
    }

    /// Create a durable collection directory and its initial generation.
    #[tracing::instrument(name = "qenlo.create", skip_all, fields(backend = ?config.backend))]
    pub async fn create(path: impl AsRef<Path>, config: CollectionConfig) -> Result<Self, Error> {
        let path = path.as_ref().to_path_buf();
        let mut collection = Self::new(config).await?;
        let lock = storage::create(&path, &collection.store)
            .map_err(|error| Error::Storage(error.to_string()))?;
        collection.storage_lock = Some(lock);
        collection.path = Some(path);
        collection.durable_generation = Some(collection.store.generation());
        Ok(collection)
    }

    async fn create_with_options(
        path: impl AsRef<Path>,
        config: CollectionConfig,
        options: StorageOptions,
    ) -> Result<Self, Error> {
        let path = path.as_ref().to_path_buf();
        let mut collection = Self::new(config).await?;
        collection.storage_options = options;
        let lock = storage::create_with_limit(&path, &collection.store, options.max_load_bytes)
            .map_err(|error| Error::Storage(error.to_string()))?;
        collection.storage_lock = Some(lock);
        collection.path = Some(path);
        collection.durable_generation = Some(0);
        Ok(collection)
    }

    /// Open the newest committed generation of a durable collection.
    ///
    /// The configured dimension must exactly match the stored dimension. Format
    /// versions are never inferred or migrated implicitly.
    #[tracing::instrument(name = "qenlo.open", skip_all, fields(backend = ?config.backend))]
    pub async fn open(path: impl AsRef<Path>, config: CollectionConfig) -> Result<Self, Error> {
        let path = path.as_ref().to_path_buf();
        let opened = storage::open(&path).map_err(|error| Error::Storage(error.to_string()))?;
        Self::from_opened(path, config, StorageOptions::default(), opened).await
    }

    async fn open_with_options(
        path: impl AsRef<Path>,
        config: CollectionConfig,
        options: StorageOptions,
    ) -> Result<Self, Error> {
        let path = path.as_ref().to_path_buf();
        let opened = storage::open_with_limit(&path, options.max_load_bytes)
            .map_err(|error| Error::Storage(error.to_string()))?;
        Self::from_opened(path, config, options, opened).await
    }

    async fn from_opened(
        path: PathBuf,
        config: CollectionConfig,
        options: StorageOptions,
        opened: storage::OpenedStore,
    ) -> Result<Self, Error> {
        if opened.store.dimension() != config.dimension {
            return Err(CoreError::DimensionMismatch {
                expected: opened.store.dimension(),
                actual: config.dimension,
            }
            .into());
        }
        let durable_generation = opened.store.generation();
        let (backend, fallback_reason) = Self::initialize_backend(&config).await?;
        let (persisted_index_generation, preparation_reason) = index_state::inspect(
            &path,
            config.dimension,
            durable_generation,
            backend_tag(&backend),
        );
        Ok(Self {
            storage_options: options,
            last_commit: None,
            rebuild_policy: RebuildPolicy::OnSearch,
            preparation_reason,
            persisted_index_generation,
            index_persistence: Measurement::unavailable(
                "index marker read on open; resident index must rebuild",
            ),
            config,
            store: opened.store,
            backend,
            prepared_generation: None,
            fallback_reason,
            path: Some(path),
            durable_generation: Some(durable_generation),
            recovered_interrupted_write: opened.recovered_interrupted_write,
            closed: false,
            storage_lock: Some(opened.lock),
        })
    }

    async fn initialize_backend(
        config: &CollectionConfig,
    ) -> Result<(Backend, Option<String>), Error> {
        match config.backend {
            BackendSelection::CpuExact => Ok((Backend::Cpu, None)),
            #[cfg(feature = "usearch")]
            BackendSelection::Usearch => Ok((
                Backend::Usearch(usearch_backend::UsearchBackend::new(config.dimension)?),
                None,
            )),
            #[cfg(feature = "gpu-wgpu")]
            BackendSelection::WgpuRequired(mode) => {
                gpu::GpuBackend::new(config.dimension, mode, config.gpu_allocation_budget_bytes)
                    .await
                    .map(|gpu| (Backend::Wgpu(Box::new(gpu)), None))
                    .map_err(|error| Error::Preparation(error.to_string()))
            }
            #[cfg(feature = "gpu-wgpu")]
            BackendSelection::Automatic(mode) => match gpu::GpuBackend::new(
                config.dimension,
                mode,
                config.gpu_allocation_budget_bytes,
            )
            .await
            {
                Ok(gpu) => Ok((Backend::Wgpu(Box::new(gpu)), None)),
                Err(error) => Ok((Backend::Cpu, Some(error.to_string()))),
            },
        }
    }

    pub fn add(
        &mut self,
        id: u64,
        user_id: u64,
        timestamp: i64,
        vector: &[f32],
    ) -> Result<(), Error> {
        self.ensure_open()?;
        if self.path.is_some() {
            self.commit(&[Mutation::Add(NewRecord {
                id,
                user_id,
                timestamp,
                vector: vector.to_vec(),
            })])?;
            return Ok(());
        }
        self.store.add(id, user_id, timestamp, vector)?;
        self.prepared_generation = None;
        self.preparation_reason = PreparationReason::Mutation;
        Ok(())
    }

    pub fn delete(&mut self, id: u64) -> Result<(), Error> {
        self.ensure_open()?;
        if self.path.is_some() {
            self.commit(&[Mutation::Delete(id)])?;
            return Ok(());
        }
        self.store.delete(id)?;
        self.prepared_generation = None;
        self.preparation_reason = PreparationReason::Mutation;
        Ok(())
    }

    /// Validate an entire ordered batch, publish it durably, then make it visible.
    ///
    /// Any validation or pre-publication I/O error leaves the collection unchanged.
    /// An error syncing a published directory entry returns `CommitUncertain` and
    /// closes the handle; reopen to resolve the outcome. An empty batch is a no-op.
    pub fn commit(&mut self, mutations: &[Mutation]) -> Result<CommitReport, Error> {
        self.ensure_open()?;
        let started = Instant::now();
        let persistence = if mutations.is_empty() {
            Measurement::unavailable("empty transaction")
        } else {
            let core_mutations = mutations
                .iter()
                .map(|mutation| match mutation {
                    Mutation::Add(row) => CoreMutation::Add {
                        id: row.id,
                        user_id: row.user_id,
                        timestamp: row.timestamp,
                        vector: &row.vector,
                    },
                    Mutation::Delete(id) => CoreMutation::Delete(*id),
                })
                .collect::<Vec<_>>();
            self.store.validate_batch(&core_mutations)?;
            let added = core_mutations
                .iter()
                .filter(|mutation| matches!(mutation, CoreMutation::Add { .. }))
                .count();
            storage::check_store_admission(
                self.store.dimension(),
                self.store
                    .len()
                    .checked_add(added)
                    .ok_or_else(|| Error::Storage("record capacity exceeded".into()))?,
                self.storage_options.max_load_bytes,
            )
            .map_err(|error| Error::Storage(error.to_string()))?;
            let persistence = if let Some(path) = &self.path {
                let writing = Instant::now();
                if let Err(error) = storage::append_wal(
                    path,
                    self.store.dimension(),
                    self.store.generation(),
                    &core_mutations,
                    self.storage_options.max_load_bytes,
                ) {
                    if matches!(error, storage::StorageError::CommitUncertain(_)) {
                        self.closed = true;
                        self.storage_lock = None;
                        return Err(Error::CommitUncertain(error.to_string()));
                    }
                    return Err(Error::Storage(error.to_string()));
                }
                Measurement::Available(writing.elapsed())
            } else {
                Measurement::unavailable("in-memory collection")
            };
            self.store.apply_batch(&core_mutations)?;
            if self.path.is_some() {
                self.durable_generation = Some(self.store.generation());
            }
            self.prepared_generation = None;
            self.preparation_reason = PreparationReason::Mutation;
            persistence
        };
        let report = CommitReport {
            operation_id: 0,
            lock_wait: Duration::ZERO,
            generation: self.store.generation(),
            durable_generation: self.durable_generation,
            mutations: mutations.len(),
            total_duration: started.elapsed(),
            persistence,
        };
        self.last_commit = Some(report.clone());
        Ok(report)
    }

    fn record_single_commit(
        &mut self,
        operation_id: u64,
        lock_wait: Duration,
        total_duration: Duration,
    ) {
        let persistence = if self.path.is_some() {
            self.last_commit
                .as_ref()
                .map(|r| r.persistence.clone())
                .unwrap_or_else(|| Measurement::unavailable("no persistence measurement"))
        } else {
            Measurement::unavailable("in-memory collection")
        };
        self.last_commit = Some(CommitReport {
            operation_id,
            lock_wait,
            generation: self.store.generation(),
            durable_generation: self.durable_generation,
            mutations: 1,
            total_duration,
            persistence,
        });
    }

    /// Resolve the eligible public IDs without exposing internal row slots.
    pub fn filter(&self, filter: &Filter) -> Vec<u64> {
        if self.closed {
            return Vec::new();
        }
        self.store
            .filter(filter)
            .into_iter()
            .filter_map(|row| self.store.record(row).map(|r| r.id()))
            .collect()
    }

    pub async fn prepare(&mut self) -> Result<bool, Error> {
        self.ensure_open()?;
        let healthy = match &self.backend {
            #[cfg(feature = "gpu-wgpu")]
            Backend::Wgpu(gpu) => gpu.is_healthy(),
            _ => true,
        };
        if self.prepared_generation == Some(self.store.generation()) && healthy {
            return Ok(false);
        }
        if !healthy {
            self.preparation_reason = PreparationReason::DeviceRecovery;
        }
        // Cancellation or failure cannot leave a removed resident index marked ready.
        self.prepared_generation = None;
        let preparation: Result<(), Error> = match &mut self.backend {
            Backend::Cpu => Ok(()),
            #[cfg(feature = "usearch")]
            Backend::Usearch(index) => index.rebuild(&self.store),
            #[cfg(feature = "gpu-wgpu")]
            Backend::Wgpu(gpu) => gpu
                .prepare(&self.store)
                .await
                .map_err(|error| Error::Preparation(error.to_string())),
        };
        #[cfg(feature = "gpu-wgpu")]
        if let Err(error) = preparation {
            if matches!(self.config.backend, BackendSelection::Automatic(_)) {
                self.fallback_reason = Some(error.to_string());
                self.backend = Backend::Cpu;
            } else {
                return Err(error);
            }
        }
        #[cfg(not(feature = "gpu-wgpu"))]
        preparation?;
        self.prepared_generation = Some(self.store.generation());
        if let Some(path) = &self.path {
            let started = Instant::now();
            self.index_persistence = match index_state::save(
                path,
                self.store.dimension(),
                self.store.generation(),
                backend_tag(&self.backend),
            ) {
                Ok(()) => {
                    self.persisted_index_generation = Some(self.store.generation());
                    Measurement::Available(started.elapsed())
                }
                Err(_) => Measurement::unavailable(
                    "derived readiness marker could not be persisted; canonical data unaffected",
                ),
            };
        }
        Ok(true)
    }

    async fn search_inner(
        &self,
        query: &[f32],
        filter: &Filter,
        k: usize,
        rebuilt: bool,
        preparation: Measurement<Duration>,
    ) -> Result<SearchResponse, Error> {
        self.ensure_open()?;
        if !(1..=MAX_K).contains(&k) {
            return Err(Error::InvalidK(k));
        }
        let started = Instant::now();
        let execution_started = Instant::now();
        #[allow(unused_mut)]
        let mut fallback_reason = self.fallback_reason.clone();
        #[allow(unused_mut)]
        let mut routing_reason = None;
        let output = match &self.backend {
            Backend::Cpu => {
                let exact = self.store.search(query, filter, k)?;
                BackendOutput::cpu(
                    exact.hits,
                    exact.evaluated_rows as u64,
                    execution_started.elapsed(),
                )
            }
            #[cfg(feature = "usearch")]
            Backend::Usearch(index) => index.search(&self.store, query, filter, k)?,
            #[cfg(feature = "gpu-wgpu")]
            Backend::Wgpu(gpu) => {
                let automatic_eligible_rows =
                    matches!(self.config.backend, BackendSelection::Automatic(_)).then(|| {
                        if *filter == Filter::ALL {
                            self.store.live_len()
                        } else {
                            self.store.filter(filter).len()
                        }
                    });
                if automatic_eligible_rows.is_some_and(route_automatic_to_cpu) {
                    let eligible_rows = automatic_eligible_rows.expect("checked as present");
                    routing_reason = Some(format!(
                        "automatic CPU route: {eligible_rows} eligible rows below GPU crossover {AUTOMATIC_GPU_MIN_ELIGIBLE_ROWS}"
                    ));
                    let exact = self.store.search(query, filter, k)?;
                    BackendOutput::cpu(
                        exact.hits,
                        exact.evaluated_rows as u64,
                        execution_started.elapsed(),
                    )
                } else {
                    if let Some(eligible_rows) = automatic_eligible_rows {
                        routing_reason = Some(format!(
                            "automatic GPU route: {eligible_rows} eligible rows meet GPU crossover {AUTOMATIC_GPU_MIN_ELIGIBLE_ROWS}"
                        ));
                    }
                    match gpu.search(&self.store, query, filter, k).await {
                        Ok(output) => output,
                        Err(error)
                            if matches!(self.config.backend, BackendSelection::Automatic(_)) =>
                        {
                            fallback_reason = Some(error.to_string());
                            let exact = self.store.search(query, filter, k)?;
                            let mut output = BackendOutput::cpu(
                                exact.hits,
                                exact.evaluated_rows as u64,
                                execution_started.elapsed(),
                            );
                            output.upload_bytes = Measurement::unavailable(
                                "failed GPU attempt: partial transfers unavailable",
                            );
                            output.readback_bytes = Measurement::unavailable(
                                "failed GPU attempt: partial readback unavailable",
                            );
                            output.dispatch_count = Measurement::unavailable(
                                "failed GPU attempt: partial dispatch count unavailable",
                            );
                            output.allocation_bytes = Measurement::unavailable(
                                "failed GPU attempt may retain resident allocations",
                            );
                            output
                        }
                        Err(error) => return Err(Error::Search(error.to_string())),
                    }
                }
            }
        };
        Ok(self.response_from_output(
            output,
            rebuilt,
            preparation,
            routing_reason,
            fallback_reason,
            started.elapsed(),
        ))
    }

    async fn search_batch_inner(
        &self,
        queries: &[&[f32]],
        filter: &Filter,
        k: usize,
        context: SearchBatchContext<'_>,
    ) -> Result<Vec<SearchResponse>, Error> {
        let SearchBatchContext {
            rebuilt,
            preparation,
            #[cfg(feature = "gpu-wgpu")]
            row_preparation,
            #[cfg(feature = "gpu-wgpu")]
            row_cache,
            #[cfg(feature = "gpu-wgpu")]
            router_profile,
            #[cfg(not(feature = "gpu-wgpu"))]
                marker: _,
        } = context;
        #[cfg(feature = "gpu-wgpu")]
        if let Backend::Wgpu(gpu) = &self.backend {
            let uses_host_rows = matches!(
                gpu.filter_mode(),
                GpuFilterMode::CpuMask | GpuFilterMode::CpuEligibleRows
            );
            let plan = EligibilityPlan::compile(
                &self.store,
                filter,
                gpu.filter_mode(),
                row_preparation,
                row_cache,
            );
            let eligible_rows = plan.eligible_count;
            let automatic = matches!(self.config.backend, BackendSelection::Automatic(_));
            let matched_profile = router_profile.as_ref().filter(|profile| {
                profile.adapter_name == gpu.capabilities().adapter_name
                    && profile.dimension == self.store.dimension()
                    && profile.batch_size == queries.len()
                    && profile.filter_mode == gpu.filter_mode()
                    && profile.cached_rows == (row_preparation == GpuRowPreparation::Cached)
            });
            let use_cpu = automatic
                && route_automatic_to_cpu_batch(
                    eligible_rows,
                    queries.len(),
                    matched_profile.map(|profile| profile.gpu_min_eligible_rows),
                );
            if use_cpu {
                let source = if matched_profile.is_some() {
                    "matched tuning profile"
                } else {
                    "static fallback"
                };
                let routing_reason = Some(format!(
                    "automatic CPU route ({source}): {eligible_rows} eligible rows across {} queries",
                    queries.len()
                ));
                let started = Instant::now();
                let rows = plan.rows.as_deref().expect("eligibility plan retains rows");
                let exact = queries
                    .iter()
                    .map(|query| self.store.search_rows(query, filter, rows, k))
                    .collect::<Result<Vec<_>, _>>()?;
                let elapsed = started.elapsed();
                return Ok(exact
                    .into_iter()
                    .map(|exact| {
                        let mut output =
                            BackendOutput::cpu(exact.hits, exact.evaluated_rows as u64, elapsed);
                        plan.annotate(&mut output, row_preparation, uses_host_rows);
                        let mut response = self.response_from_output(
                            output,
                            rebuilt,
                            preparation.clone(),
                            routing_reason.clone(),
                            self.fallback_reason.clone(),
                            elapsed,
                        );
                        response.report.eligible_rows =
                            Measurement::Available(eligible_rows as u64);
                        response
                    })
                    .collect());
            }
            if !use_cpu {
                let routing_reason = automatic.then(|| {
                    let source = if matched_profile.is_some() {
                        "matched tuning profile"
                    } else {
                        "static fallback"
                    };
                    format!("automatic GPU route ({source}): {eligible_rows} eligible rows across {} queries", queries.len())
                });
                let started = Instant::now();
                match gpu
                    .search_batch(&self.store, queries, filter, k, plan.rows.as_deref())
                    .await
                {
                    Ok(mut outputs) => {
                        let elapsed = started.elapsed();
                        for output in &mut outputs {
                            plan.annotate(output, row_preparation, uses_host_rows);
                        }
                        return Ok(outputs
                            .into_iter()
                            .map(|output| {
                                let mut response = self.response_from_output(
                                    output,
                                    rebuilt,
                                    preparation.clone(),
                                    routing_reason.clone(),
                                    self.fallback_reason.clone(),
                                    elapsed,
                                );
                                response.report.eligible_rows =
                                    Measurement::Available(eligible_rows as u64);
                                response
                            })
                            .collect());
                    }
                    Err(error) if automatic => {
                        let fallback = Some(error.to_string());
                        let elapsed = started.elapsed();
                        let rows = plan.rows.as_deref().expect("eligibility plan retains rows");
                        return queries
                            .iter()
                            .map(|query| {
                                let exact = self.store.search_rows(query, filter, rows, k)?;
                                let mut output = BackendOutput::cpu(
                                    exact.hits,
                                    exact.evaluated_rows as u64,
                                    elapsed,
                                );
                                plan.annotate(&mut output, row_preparation, uses_host_rows);
                                let mut response = self.response_from_output(
                                    output,
                                    rebuilt,
                                    preparation.clone(),
                                    Some("automatic CPU fallback after batched GPU failure".into()),
                                    fallback.clone(),
                                    elapsed,
                                );
                                response.report.eligible_rows =
                                    Measurement::Available(eligible_rows as u64);
                                Ok(response)
                            })
                            .collect();
                    }
                    Err(error) => return Err(Error::Search(error.to_string())),
                }
            }
        }
        let mut responses = Vec::with_capacity(queries.len());
        for query in queries {
            responses.push(
                self.search_inner(query, filter, k, rebuilt, preparation.clone())
                    .await?,
            );
        }
        Ok(responses)
    }

    fn response_from_output(
        &self,
        output: BackendOutput,
        rebuilt: bool,
        preparation: Measurement<Duration>,
        routing_reason: Option<String>,
        fallback_reason: Option<String>,
        total_duration: Duration,
    ) -> SearchResponse {
        let cpu_distance_path = if output.actual_backend == BackendKind::Cpu {
            match &output.candidates {
                Measurement::Available(rows) => {
                    Some(qenlo_core::cpu_distance_path_for_eligible_count(
                        usize::try_from(*rows).unwrap_or(usize::MAX),
                    ))
                }
                Measurement::Unavailable(_) => Some(qenlo_core::cpu_distance_path()),
            }
        } else {
            None
        };
        let results = output
            .hits
            .into_iter()
            .map(|hit| SearchResult {
                id: hit.id,
                distance: hit.distance,
            })
            .collect::<Vec<_>>();
        let report = ExecutionReport {
            operation_id: 0,
            cpu_distance_path,
            eligible_rows: Measurement::unavailable("detailed diagnostics disabled"),
            last_commit: self.last_commit.clone(),
            preparation_reason: None,
            lock_wait: Duration::ZERO,
            ann_parameters: match &self.backend {
                #[cfg(feature = "usearch")]
                Backend::Usearch(index) => Some(index.parameters()),
                _ => None,
            },
            requested_backend: self.config.backend,
            actual_backend: output.actual_backend,
            algorithm: output.algorithm,
            filter_execution: output.filter_execution,
            index_generation: self.store.generation(),
            rebuilt,
            routing_reason,
            fallback_reason,
            total_duration,
            phases: output.phases.with_preparation(preparation),
            upload_bytes: output.upload_bytes,
            readback_bytes: output.readback_bytes,
            dispatch_count: output.dispatch_count,
            qenlo_allocation_bytes: output.allocation_bytes,
            candidates: output.candidates,
            gpu_row_preparation: output.gpu_row_preparation,
            predicate_traversals: output.predicate_traversals,
            row_materialization: output.row_materialization,
            materialized_rows: output.materialized_rows,
            row_cache_hit: output.row_cache_hit,
            eligibility_predicate_kind: output.eligibility_predicate_kind,
            eligibility_representation: output.eligibility_representation,
            eligibility_generation: output.eligibility_generation,
            corpus_rows: output.corpus_rows,
            eligible_selectivity: output.eligible_selectivity,
            eligibility_transfer_bytes: output.eligibility_transfer_bytes,
            eligible_contiguous_runs: output.eligible_contiguous_runs,
            eligibility_cacheable: output.eligibility_cacheable,
            eligibility_resident: output.eligibility_resident,
            results: results.len(),
            batch_size: 1,
        };
        SearchResponse { results, report }
    }

    pub fn stats(&self) -> CollectionStats {
        CollectionStats {
            persisted_index_generation: self.persisted_index_generation,
            preparation_reason: self.preparation_reason,
            index_persistence: self.index_persistence.clone(),
            dimension: self.store.dimension(),
            rows: self.store.len(),
            live_rows: self.store.live_len(),
            generation: self.store.generation(),
            prepared_generation: self.prepared_generation,
            durable_generation: self.durable_generation,
            recovered_interrupted_write: self.recovered_interrupted_write,
            closed: self.closed,
        }
    }

    /// Durably publish the current canonical generation.
    pub fn flush(&mut self) -> Result<(), Error> {
        self.ensure_open()?;
        let Some(path) = &self.path else {
            return Ok(());
        };
        if self.durable_generation == Some(self.store.generation()) {
            return Ok(());
        }
        storage::write_snapshot_with_limit(path, &self.store, self.storage_options.max_load_bytes)
            .map_err(|error| Error::Storage(error.to_string()))?;
        self.durable_generation = Some(self.store.generation());
        Ok(())
    }

    /// Flush and close this handle. Further operations return [`Error::Closed`].
    pub fn close(&mut self) -> Result<(), Error> {
        if self.closed {
            return Ok(());
        }
        self.flush()?;
        self.closed = true;
        self.storage_lock = None;
        Ok(())
    }

    fn ensure_open(&self) -> Result<(), Error> {
        if self.closed {
            Err(Error::Closed)
        } else {
            Ok(())
        }
    }
}

#[cfg(feature = "gpu-wgpu")]
fn route_automatic_to_cpu(eligible_rows: usize) -> bool {
    // ponytail: static crossover; replace with the persisted per-device autotune profile.
    eligible_rows < AUTOMATIC_GPU_MIN_ELIGIBLE_ROWS
}

#[cfg(feature = "gpu-wgpu")]
fn route_automatic_to_cpu_batch(
    eligible_rows: usize,
    batch_size: usize,
    profiled_gpu_min_rows: Option<usize>,
) -> bool {
    let below_crossover = profiled_gpu_min_rows.map_or_else(
        || route_automatic_to_cpu(eligible_rows),
        |minimum| eligible_rows < minimum,
    );
    below_crossover && batch_size < 8
}

fn backend_tag(backend: &Backend) -> u8 {
    match backend {
        Backend::Cpu => 0,
        #[cfg(feature = "usearch")]
        Backend::Usearch(_) => 1,
        #[cfg(feature = "gpu-wgpu")]
        Backend::Wgpu(_) => 2,
    }
}

pub(crate) struct BackendOutput {
    hits: Vec<SearchHit>,
    actual_backend: BackendKind,
    algorithm: Algorithm,
    filter_execution: FilterExecution,
    phases: PhaseTimings,
    upload_bytes: Measurement<u64>,
    readback_bytes: Measurement<u64>,
    dispatch_count: Measurement<u32>,
    allocation_bytes: Measurement<u64>,
    candidates: Measurement<u64>,
    gpu_row_preparation: Option<GpuRowPreparation>,
    predicate_traversals: u32,
    row_materialization: Measurement<Duration>,
    materialized_rows: Measurement<u64>,
    row_cache_hit: Option<bool>,
    eligibility_predicate_kind: Option<EligibilityPredicateKind>,
    eligibility_representation: Option<EligibilityRepresentation>,
    eligibility_generation: Option<u64>,
    corpus_rows: Measurement<u64>,
    eligible_selectivity: Measurement<f64>,
    eligibility_transfer_bytes: Measurement<u64>,
    eligible_contiguous_runs: Measurement<u64>,
    eligibility_cacheable: Option<bool>,
    eligibility_resident: Option<bool>,
}

impl BackendOutput {
    fn cpu(hits: Vec<SearchHit>, candidates: u64, execution: Duration) -> Self {
        Self {
            hits,
            actual_backend: BackendKind::Cpu,
            algorithm: Algorithm::Exact,
            filter_execution: FilterExecution::OrderedMetadataIndexes,
            phases: PhaseTimings::cpu(Measurement::unavailable("set by collection"), execution),
            upload_bytes: Measurement::Available(0),
            readback_bytes: Measurement::Available(0),
            dispatch_count: Measurement::Available(0),
            allocation_bytes: Measurement::unavailable("CPU allocator bytes are not instrumented"),
            candidates: Measurement::Available(candidates),
            gpu_row_preparation: None,
            predicate_traversals: 1,
            row_materialization: Measurement::unavailable("CPU exact execution owns filtering"),
            materialized_rows: Measurement::unavailable("CPU exact execution has no GPU row list"),
            row_cache_hit: None,
            eligibility_predicate_kind: None,
            eligibility_representation: None,
            eligibility_generation: None,
            corpus_rows: Measurement::unavailable("CPU execution has no compiled eligibility plan"),
            eligible_selectivity: Measurement::unavailable(
                "CPU execution has no compiled eligibility plan",
            ),
            eligibility_transfer_bytes: Measurement::Available(0),
            eligible_contiguous_runs: Measurement::unavailable(
                "CPU execution has no compiled eligibility plan",
            ),
            eligibility_cacheable: None,
            eligibility_resident: None,
        }
    }
}

impl PhaseTimings {
    fn with_preparation(mut self, preparation: Measurement<Duration>) -> Self {
        self.preparation = preparation;
        self
    }
}

#[cfg(feature = "usearch")]
fn eligible_ids(store: &CoreStore, predicate: &Predicate) -> HashSet<u64> {
    store
        .filter(predicate)
        .into_iter()
        .filter_map(|row| store.record(row).map(|record| record.id()))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        path::PathBuf,
        task::{Context, Poll, Waker},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[cfg(feature = "gpu-wgpu")]
    #[test]
    fn automatic_router_keeps_selective_work_on_cpu() {
        assert!(route_automatic_to_cpu(0));
        assert!(route_automatic_to_cpu(AUTOMATIC_GPU_MIN_ELIGIBLE_ROWS - 1));
        assert!(!route_automatic_to_cpu(AUTOMATIC_GPU_MIN_ELIGIBLE_ROWS));
    }

    use super::*;

    fn block_on<F: Future>(future: F) -> F::Output {
        let mut context = Context::from_waker(Waker::noop());
        let mut future = std::pin::pin!(future);
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(output) => return output,
                Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("qenlo-api-{label}-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn cpu_collection_rebuilds_after_mutation_and_reports_execution() {
        block_on(async {
            let collection = Collection::new(CollectionConfig::cpu_exact(2))
                .await
                .unwrap();
            collection.add(2, 7, 10, &[1.0, 0.0]).unwrap();
            collection.add(1, 7, 11, &[1.0, 0.0]).unwrap();
            let filter = Filter {
                user_id: Some(7),
                timestamp: TimestampRange::default(),
            };
            let first = collection.search(&[1.0, 0.0], &filter, 2).await.unwrap();
            assert_eq!(
                first.results.iter().map(|hit| hit.id).collect::<Vec<_>>(),
                [1, 2]
            );
            assert!(first.report.rebuilt);
            assert_eq!(first.report.actual_backend, BackendKind::Cpu);
            assert!(matches!(first.report.candidates, Measurement::Available(2)));
            assert!(
                !collection
                    .search(&[1.0, 0.0], &filter, 2)
                    .await
                    .unwrap()
                    .report
                    .rebuilt
            );
            collection.delete(1).unwrap();
            let after_delete = collection.search(&[1.0, 0.0], &filter, 2).await.unwrap();
            assert!(after_delete.report.rebuilt);
            assert_eq!(
                after_delete
                    .results
                    .iter()
                    .map(|hit| hit.id)
                    .collect::<Vec<_>>(),
                [2]
            );
        });
    }

    #[test]
    fn durable_collection_survives_restart_with_tombstones() {
        block_on(async {
            let path = temp_dir("restart");
            let config = CollectionConfig::cpu_exact(2);
            let collection = Collection::create(&path, config.clone()).await.unwrap();
            collection.add(1, 7, i64::MIN, &[1.0, 0.0]).unwrap();
            collection.add(2, 8, i64::MAX, &[0.0, 1.0]).unwrap();
            collection.delete(1).unwrap();
            collection.flush().unwrap();
            assert_eq!(collection.stats().durable_generation, Some(3));
            collection.close().unwrap();
            assert!(matches!(
                collection.add(3, 9, 0, &[1.0, 0.0]),
                Err(Error::Closed)
            ));

            let reopened = Collection::open(&path, config).await.unwrap();
            assert_eq!(reopened.stats().rows, 2);
            assert_eq!(reopened.stats().live_rows, 1);
            assert_eq!(
                reopened
                    .search(&[0.0, 1.0], &Filter::ALL, 2)
                    .await
                    .unwrap()
                    .results
                    .iter()
                    .map(|result| result.id)
                    .collect::<Vec<_>>(),
                [2]
            );
            reopened.close().unwrap();
            std::fs::remove_dir_all(path).unwrap();
        });
    }

    #[test]
    fn portable_qn_round_trip_is_single_file_exact_and_non_overwriting() {
        block_on(async {
            let root = temp_dir("portable-qn");
            std::fs::create_dir_all(&root).unwrap();
            let path = root.join("vectors.qn");
            let collection = Collection::new(CollectionConfig::cpu_exact(2))
                .await
                .unwrap();
            collection.add(9, 7, -5, &[1.0, 0.0]).unwrap();
            collection.add(2, 8, 10, &[0.0, 1.0]).unwrap();
            collection.delete(9).unwrap();
            collection.export_qn(&path).unwrap();

            assert!(path.is_file());
            assert_eq!(&std::fs::read(&path).unwrap()[..8], b"QENLODB\0");
            assert!(matches!(
                collection.export_qn(&path),
                Err(Error::Storage(message)) if message.contains("already exists")
            ));

            let imported = Collection::import_qn(&path, CollectionConfig::cpu_exact(2))
                .await
                .unwrap();
            assert_eq!(imported.stats().rows, 2);
            assert_eq!(imported.stats().live_rows, 1);
            assert_eq!(imported.stats().generation, 3);
            assert_eq!(imported.stats().durable_generation, None);
            assert_eq!(
                imported
                    .search(&[0.0, 1.0], &Filter::ALL, 10)
                    .await
                    .unwrap()
                    .results
                    .iter()
                    .map(|hit| hit.id)
                    .collect::<Vec<_>>(),
                [2]
            );
            assert!(matches!(
                Collection::import_qn(&path, CollectionConfig::cpu_exact(3)).await,
                Err(Error::Storage(message)) if message.contains("dimension mismatch")
            ));
            assert!(matches!(
                collection.export_qn(root.join("vectors.qenlo")),
                Err(Error::Storage(message)) if message.contains(".qn extension")
            ));

            let recovered_path = root.join("recovered.qn");
            collection.export_qn(&recovered_path).unwrap();
            let pending_path = recovered_path.with_extension("qn.pending");
            std::fs::rename(&recovered_path, &pending_path).unwrap();
            let recovered = Collection::import_qn(&recovered_path, CollectionConfig::cpu_exact(2))
                .await
                .unwrap();
            assert!(recovered_path.is_file());
            assert!(!pending_path.exists());
            assert!(recovered.stats().recovered_interrupted_write);

            let corrupt_path = root.join("corrupt.qn");
            let corrupt_pending = corrupt_path.with_extension("qn.pending");
            std::fs::write(&corrupt_pending, b"incomplete").unwrap();
            assert!(matches!(
                Collection::import_qn(&corrupt_path, CollectionConfig::cpu_exact(2)).await,
                Err(Error::Storage(message)) if message.contains("shorter than its header")
            ));
            assert!(!corrupt_path.exists());
            assert!(corrupt_pending.is_file());
            std::fs::remove_dir_all(root).unwrap();
        });
    }

    #[test]
    fn transactions_validate_all_rows_and_roll_back_before_publication() {
        block_on(async {
            let path = temp_dir("transaction");
            let collection = Collection::create(&path, CollectionConfig::cpu_exact(2))
                .await
                .unwrap();
            let row = NewRecord {
                id: 1,
                user_id: 2,
                timestamp: i64::MIN,
                vector: vec![1.0, 0.0],
            };
            assert!(collection.add_batch(&[row.clone(), row.clone()]).is_err());
            assert_eq!(collection.stats().generation, 0);
            collection.add_batch(&[row]).unwrap();
            assert!(collection.delete_batch(&[1, 99]).is_err());
            assert_eq!(collection.filter(&Filter::ALL), [1]);
            let report = collection
                .commit(&[
                    Mutation::Delete(1),
                    Mutation::Add(NewRecord {
                        id: 2,
                        user_id: 3,
                        timestamp: i64::MAX,
                        vector: vec![0.0, 1.0],
                    }),
                ])
                .unwrap();
            assert_eq!(report.mutations, 2);
            assert_eq!(report.durable_generation, Some(3));
            // No flush or close: successful commits already survive handle loss.
            drop(collection);
            let collection = Collection::open(&path, CollectionConfig::cpu_exact(2))
                .await
                .unwrap();
            assert_eq!(collection.filter(&Filter::ALL), [2]);
            drop(collection);
            std::fs::remove_dir_all(path).unwrap();
        });
    }

    #[test]
    fn transaction_io_failure_preserves_memory_and_last_commit() {
        block_on(async {
            let path = temp_dir("transaction-io");
            let collection = Collection::create(&path, CollectionConfig::cpu_exact(2))
                .await
                .unwrap();
            // A directory at the precise staging file path deterministically fails writing.
            let obstruction = path.join("wal-00000000000000000001.pending");
            std::fs::create_dir(&obstruction).unwrap();
            assert!(collection.add(1, 2, 3, &[1.0, 0.0]).is_err());
            assert_eq!(collection.stats().live_rows, 0);
            drop(collection);
            let collection = Collection::open(&path, CollectionConfig::cpu_exact(2))
                .await
                .unwrap();
            assert_eq!(collection.stats().generation, 0);
            drop(collection);
            std::fs::remove_dir_all(path).unwrap();
        });
    }

    #[test]
    fn manifest_publication_failure_is_uncertain_and_reopen_resolves_wal() {
        block_on(async {
            let path = temp_dir("manifest-uncertain");
            let config = CollectionConfig::cpu_exact(2);
            let collection = Collection::create(&path, config.clone()).await.unwrap();
            let obstruction = path.join("MANIFEST.pending");
            std::fs::create_dir(&obstruction).unwrap();
            assert!(matches!(
                collection.add(1, 2, 3, &[1.0, 0.0]),
                Err(Error::CommitUncertain(_))
            ));
            assert!(collection.stats().closed);
            std::fs::remove_dir(&obstruction).unwrap();
            let reopened = Collection::open(&path, config).await.unwrap();
            assert_eq!(reopened.filter(&Filter::ALL), [1]);
            assert!(reopened.stats().recovered_interrupted_write);
            reopened.close().unwrap();
            std::fs::remove_dir_all(path).unwrap();
        });
    }

    #[test]
    fn shared_collection_and_search_futures_are_send() {
        fn send_sync<T: Send + Sync>() {}
        fn send<T: Send>(_: T) {}
        send_sync::<Collection>();
        let collection = block_on(Collection::new(CollectionConfig::cpu_exact(2))).unwrap();
        send(collection.search(&[1.0, 0.0], &Filter::ALL, 1));
    }

    #[test]
    fn index_markers_are_disposable_and_explicit_policy_never_serves_stale_data() {
        block_on(async {
            let path = temp_dir("index-lifecycle");
            let config = CollectionConfig::cpu_exact(2);
            let collection = Collection::create(&path, config.clone()).await.unwrap();
            collection.add(1, 1, 0, &[1.0, 0.0]).unwrap();
            collection
                .set_rebuild_policy(RebuildPolicy::Explicit)
                .unwrap();
            assert!(matches!(
                collection.search(&[1.0, 0.0], &Filter::ALL, 1).await,
                Err(Error::IndexNotPrepared)
            ));
            collection.prepare().await.unwrap();
            assert_eq!(collection.stats().persisted_index_generation, Some(1));
            collection.delete(1).unwrap();
            assert!(matches!(
                collection.search(&[1.0, 0.0], &Filter::ALL, 1).await,
                Err(Error::IndexNotPrepared)
            ));
            collection.close().unwrap();
            let reopened = Collection::open(&path, config.clone()).await.unwrap();
            assert_eq!(
                reopened.stats().preparation_reason,
                PreparationReason::StaleIndex
            );
            let response = reopened.search(&[1.0, 0.0], &Filter::ALL, 1).await.unwrap();
            assert!(response.results.is_empty());
            assert_eq!(
                response.report.preparation_reason,
                Some(PreparationReason::StaleIndex)
            );
            reopened.close().unwrap();
            for (reason, corrupt) in [
                (PreparationReason::CorruptIndex, true),
                (PreparationReason::MissingIndex, false),
            ] {
                let marker = path.join("index.qidx");
                if corrupt {
                    std::fs::write(&marker, b"corrupt").unwrap();
                } else {
                    std::fs::remove_file(&marker).unwrap();
                }
                let reopened = Collection::open(&path, config.clone()).await.unwrap();
                assert_eq!(reopened.stats().preparation_reason, reason);
                assert!(
                    reopened
                        .search(&[1.0, 0.0], &Filter::ALL, 1)
                        .await
                        .unwrap()
                        .results
                        .is_empty()
                );
                reopened.close().unwrap();
            }
            std::fs::remove_dir_all(path).unwrap();
        });
    }

    #[test]
    fn diagnostics_preserve_results_and_disclose_missing_measurements() {
        block_on(async {
            let collection = Collection::new(CollectionConfig::cpu_exact(2))
                .await
                .unwrap();
            collection.add(1, 900001, i64::MIN, &[0.25, 0.75]).unwrap();
            let mut previous_id = 0;
            for diagnostics in [
                Diagnostics::Disabled,
                Diagnostics::Basic,
                Diagnostics::Detailed,
            ] {
                collection.set_diagnostics(diagnostics);
                let response = collection
                    .search(&[0.25, 0.75], &Filter::ALL, 1)
                    .await
                    .unwrap();
                assert_eq!(response.results[0].id, 1);
                assert!(response.report.operation_id > previous_id);
                previous_id = response.report.operation_id;
                assert!(response.report.cpu_distance_path.is_some());
                assert!(response.report.last_commit.is_some());
                assert_eq!(
                    matches!(response.report.eligible_rows, Measurement::Available(1)),
                    diagnostics == Diagnostics::Detailed
                );
            }
        });
    }

    #[test]
    fn custom_storage_budget_rejects_unreopenable_and_unpublished_writes() {
        block_on(async {
            let path = temp_dir("options");
            let config = CollectionConfig::cpu_exact(2);
            let options = StorageOptions {
                max_load_bytes: 600,
            };
            let collection = Collection::create_with_options(&path, config.clone(), options)
                .await
                .unwrap();
            collection.add(1, 1, 0, &[1.0, 0.0]).unwrap();
            assert!(collection.add(2, 1, 0, &[1.0, 0.0]).is_err());
            collection.close().unwrap();
            let reopened = Collection::open_with_options(&path, config.clone(), options)
                .await
                .unwrap();
            assert_eq!(reopened.stats().live_rows, 1);
            let obstruction = path.join("wal-00000000000000000002.qwal");
            std::fs::create_dir(&obstruction).unwrap();
            assert!(matches!(reopened.delete(1), Err(Error::Storage(_))));
            assert!(!reopened.stats().closed);
            assert_eq!(reopened.stats().live_rows, 1);
            std::fs::remove_dir(&obstruction).unwrap();
            reopened.delete(1).unwrap();
            drop(reopened);
            let resolved = Collection::open(&path, config).await.unwrap();
            assert_eq!(resolved.stats().live_rows, 0);
            resolved.close().unwrap();
            std::fs::remove_dir_all(path).unwrap();
        });
    }

    #[test]
    fn searches_observe_whole_transactions_while_writer_runs() {
        use std::sync::{Arc, Barrier};
        let collection =
            Arc::new(block_on(Collection::new(CollectionConfig::cpu_exact(2))).unwrap());
        collection.add(0, 1, 0, &[1.0, 0.0]).unwrap();
        collection.add(1, 1, 0, &[1.0, 0.0]).unwrap();
        let barrier = Arc::new(Barrier::new(5));
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let collection = Arc::clone(&collection);
                let barrier = Arc::clone(&barrier);
                scope.spawn(move || {
                    barrier.wait();
                    for _ in 0..100 {
                        let response =
                            block_on(collection.search(&[1.0, 0.0], &Filter::ALL, 64)).unwrap();
                        assert_eq!(response.results.len(), 2);
                        assert_eq!(response.results[1].id, response.results[0].id + 1);
                        assert!(response.report.total_duration >= response.report.lock_wait);
                    }
                });
            }
            barrier.wait();
            for pair in 1..=50_u64 {
                collection
                    .commit(&[
                        Mutation::Delete((pair - 1) * 2),
                        Mutation::Delete((pair - 1) * 2 + 1),
                        Mutation::Add(NewRecord {
                            id: pair * 2,
                            user_id: 1,
                            timestamp: 0,
                            vector: vec![1.0, 0.0],
                        }),
                        Mutation::Add(NewRecord {
                            id: pair * 2 + 1,
                            user_id: 1,
                            timestamp: 0,
                            vector: vec![1.0, 0.0],
                        }),
                    ])
                    .unwrap();
            }
        });
        // Ready reads can hold the lock at the same time, rather than a hidden mutex.
        let first = collection.inner.try_read().unwrap();
        let second = collection.inner.try_read().unwrap();
        assert_eq!(first.store.generation(), second.store.generation());
    }

    #[cfg(feature = "usearch")]
    #[test]
    fn usearch_filters_during_graph_search_and_reports_unknown_traversal_count() {
        block_on(async {
            let collection = Collection::new(CollectionConfig {
                dimension: 2,
                backend: BackendSelection::Usearch,
                gpu_allocation_budget_bytes: DEFAULT_GPU_BUDGET_BYTES,
            })
            .await
            .unwrap();
            collection.add(1, 10, 0, &[1.0, 0.0]).unwrap();
            collection.add(2, 20, 0, &[1.0, 0.0]).unwrap();
            let response = collection
                .search(
                    &[1.0, 0.0],
                    &Filter {
                        user_id: Some(20),
                        timestamp: TimestampRange::ALL,
                    },
                    1,
                )
                .await
                .unwrap();
            assert_eq!(response.results[0].id, 2);
            assert_eq!(response.report.actual_backend, BackendKind::Usearch);
            assert!(matches!(
                response.report.candidates,
                Measurement::Unavailable(_)
            ));
        });
    }

    #[cfg(feature = "gpu-wgpu")]
    #[test]
    fn eligibility_plan_matches_canonical_filter_across_mutations() {
        let mut store = CoreStore::new(2).unwrap();
        let mut seed = 0x51_7a_9d_23_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            seed
        };
        for id in 0..200_u64 {
            let user = next() % 11;
            let timestamp = (next() % 401) as i64 - 200;
            store.add(id, user, timestamp, [1.0, 0.5]).unwrap();
        }
        for id in (0..200_u64).filter(|id| id % 3 == 0) {
            store.delete(id).unwrap();
        }

        let cache = Mutex::new(None);
        for _ in 0..256 {
            let user = (next() & 1 != 0).then(|| next() % 11);
            let lower = (next() & 1 != 0).then(|| (next() % 501) as i64 - 250);
            let upper = (next() & 1 != 0).then(|| (next() % 501) as i64 - 250);
            let filter = Filter::new(user, TimestampRange::new(lower, upper));
            let expected = store.filter(&filter);
            for mode in [
                GpuFilterMode::CpuEligibleRows,
                GpuFilterMode::CpuMask,
                GpuFilterMode::GpuPredicate,
            ] {
                let plan = EligibilityPlan::compile(
                    &store,
                    &filter,
                    mode,
                    GpuRowPreparation::OnePass,
                    &cache,
                );
                assert_eq!(plan.generation, store.generation());
                assert_eq!(plan.eligible_count, expected.len());
                if let Some(rows) = plan.rows {
                    assert_eq!(rows.as_ref(), expected.as_slice());
                }
                assert_eq!(plan.contiguous_runs, contiguous_run_count(&expected));
            }
        }

        let filter = Filter::ALL;
        let first = EligibilityPlan::compile(
            &store,
            &filter,
            GpuFilterMode::CpuEligibleRows,
            GpuRowPreparation::Cached,
            &cache,
        );
        assert_eq!(first.cache_hit, Some(false));
        let second = EligibilityPlan::compile(
            &store,
            &filter,
            GpuFilterMode::CpuEligibleRows,
            GpuRowPreparation::Cached,
            &cache,
        );
        assert_eq!(second.cache_hit, Some(true));
        store.add(200, 4, 0, [0.5, 1.0]).unwrap();
        let after_mutation = EligibilityPlan::compile(
            &store,
            &filter,
            GpuFilterMode::CpuEligibleRows,
            GpuRowPreparation::Cached,
            &cache,
        );
        assert_eq!(after_mutation.cache_hit, Some(false));
        assert_eq!(after_mutation.eligible_count, store.filter(&filter).len());
    }

    #[cfg(feature = "gpu-wgpu")]
    #[test]
    fn gpu_row_preparation_modes_are_observable_and_exact() {
        block_on(async {
            let created = Collection::new(CollectionConfig {
                dimension: 2,
                backend: BackendSelection::WgpuRequired(GpuFilterMode::CpuEligibleRows),
                gpu_allocation_budget_bytes: DEFAULT_GPU_BUDGET_BYTES,
            })
            .await;
            let collection = match created {
                Ok(collection) => collection,
                Err(error) if std::env::var_os("QENLO_REQUIRE_GPU").is_none() => {
                    eprintln!("GPU unavailable: {error}");
                    return;
                }
                Err(error) => panic!("GPU required: {error}"),
            };
            collection.add(1, 7, 0, &[1.0, 0.0]).unwrap();
            collection.add(2, 8, 0, &[0.0, 1.0]).unwrap();
            collection.prepare().await.unwrap();
            let filter = Filter::new(Some(7), TimestampRange::ALL);
            for (mode, traversals) in [
                (GpuRowPreparation::LegacyTwoPass, 2),
                (GpuRowPreparation::OnePass, 1),
            ] {
                collection.set_gpu_row_preparation(mode);
                let response = collection
                    .search_batch(&[&[1.0, 0.0]], &filter, 1)
                    .await
                    .unwrap()
                    .remove(0);
                assert_eq!(response.results[0].id, 1);
                assert_eq!(response.report.gpu_row_preparation, Some(mode));
                assert_eq!(response.report.predicate_traversals, traversals);
                assert_eq!(response.report.row_cache_hit, None);
            }
            collection.set_gpu_row_preparation(GpuRowPreparation::Cached);
            for (expected_hit, traversals) in [(false, 1), (true, 0)] {
                let response = collection
                    .search_batch(&[&[1.0, 0.0]], &filter, 1)
                    .await
                    .unwrap()
                    .remove(0);
                assert_eq!(response.results[0].id, 1);
                assert_eq!(response.report.row_cache_hit, Some(expected_hit));
                assert_eq!(response.report.predicate_traversals, traversals);
            }
        });
    }

    #[cfg(feature = "gpu-wgpu")]
    #[test]
    fn device_loss_automatic_falls_back_but_required_returns_error() {
        block_on(async {
            for backend in [
                BackendSelection::Automatic(GpuFilterMode::GpuPredicate),
                BackendSelection::WgpuRequired(GpuFilterMode::GpuPredicate),
            ] {
                let created = Collection::new(CollectionConfig {
                    dimension: 2,
                    backend,
                    gpu_allocation_budget_bytes: DEFAULT_GPU_BUDGET_BYTES,
                })
                .await;
                let collection = match created {
                    Ok(collection) => collection,
                    Err(error) if std::env::var_os("QENLO_REQUIRE_GPU").is_none() => {
                        eprintln!("GPU unavailable: {error}");
                        continue;
                    }
                    Err(error) => panic!("GPU required: {error}"),
                };
                let rows = (1..=AUTOMATIC_GPU_MIN_ELIGIBLE_ROWS as u64)
                    .map(|id| NewRecord {
                        id,
                        user_id: 1,
                        timestamp: i64::MIN,
                        vector: vec![1.0, 0.0],
                    })
                    .collect::<Vec<_>>();
                collection.add_batch(&rows).unwrap();
                collection.prepare().await.unwrap();
                match &collection.inner.read_blocking().backend {
                    Backend::Wgpu(gpu) => gpu.destroy_device_for_test(),
                    _ => continue,
                }
                let response = collection.search(&[1.0, 0.0], &Filter::ALL, 1).await;
                if matches!(backend, BackendSelection::Automatic(_)) {
                    let response = response.unwrap();
                    assert_eq!(response.results[0].id, 1);
                    assert_eq!(response.report.actual_backend, BackendKind::Cpu);
                    assert!(response.report.fallback_reason.is_some());
                } else {
                    assert!(matches!(response, Err(Error::Search(_))));
                    assert!(collection.prepare().await.unwrap());
                    assert_eq!(
                        collection
                            .search(&[1.0, 0.0], &Filter::ALL, 1)
                            .await
                            .unwrap()
                            .results[0]
                            .id,
                        1
                    );
                }
            }
        });
    }

    #[cfg(feature = "gpu-wgpu")]
    #[test]
    fn automatic_gpu_mode_discloses_over_budget_cpu_fallback_if_adapter_is_available() {
        block_on(async {
            let Ok(collection) = Collection::new(CollectionConfig {
                dimension: 2,
                backend: BackendSelection::Automatic(GpuFilterMode::CpuMask),
                gpu_allocation_budget_bytes: 1,
            })
            .await
            else {
                return;
            };
            if !matches!(&collection.inner.read_blocking().backend, &Backend::Wgpu(_)) {
                return;
            }
            for id in 1..=AUTOMATIC_GPU_MIN_ELIGIBLE_ROWS as u64 {
                collection.add(id, 7, 0, &[1.0, 0.0]).unwrap();
            }
            let response = collection
                .search(&[1.0, 0.0], &Filter::ALL, 1)
                .await
                .unwrap();
            assert_eq!(response.results[0].id, 1);
            assert_eq!(response.report.actual_backend, BackendKind::Cpu);
            assert!(
                response
                    .report
                    .fallback_reason
                    .as_deref()
                    .is_some_and(|reason| reason.contains("budget"))
            );
        });
    }
}
