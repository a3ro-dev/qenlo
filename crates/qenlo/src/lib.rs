//! Observable embedded filtered vector search.
//!
//! Default features contain only the portable exact CPU backend. Applications
//! opt into C++ (`usearch`) and GPU (`gpu-wgpu`) build requirements explicitly.

use async_lock::{RwLock, RwLockWriteGuard};
#[cfg(feature = "usearch")]
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[cfg(feature = "usearch")]
use qenlo_core::Predicate;
use qenlo_core::{CoreStore, Error as CoreError, SearchHit};
use thiserror::Error;
use tracing::{Instrument, info_span};
use web_time::{Duration, Instant};

#[cfg(feature = "gpu-wgpu")]
mod gpu;
mod storage;
#[cfg(feature = "usearch")]
mod usearch_backend;

pub use qenlo_core::{Predicate as Filter, TimestampRange};

/// Maximum supported result count for this prototype.
pub const MAX_K: usize = 64;
/// Default cap for all Qenlo-owned GPU allocations, including scratch buffers.
pub const DEFAULT_GPU_BUDGET_BYTES: u64 = 512 * 1024 * 1024;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Cpu,
    Usearch,
    Wgpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    Exact,
    Hnsw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuFilterMode {
    CpuMask,
    CpuEligibleRows,
    GpuPredicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterExecution {
    OrderedMetadataIndexes,
    GraphPredicate,
    Gpu(GpuFilterMode),
}

#[derive(Debug, Clone)]
pub struct CollectionConfig {
    pub dimension: usize,
    pub backend: BackendSelection,
    pub gpu_allocation_budget_bytes: u64,
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

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub id: u64,
    pub distance: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unavailable {
    pub reason: String,
}

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

#[derive(Debug, Clone)]
pub struct ExecutionReport {
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
    pub fallback_reason: Option<String>,
    pub total_duration: Duration,
    pub phases: PhaseTimings,
    pub upload_bytes: Measurement<u64>,
    pub readback_bytes: Measurement<u64>,
    pub dispatch_count: Measurement<u32>,
    pub qenlo_allocation_bytes: Measurement<u64>,
    pub candidates: Measurement<u64>,
    pub results: usize,
}

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
    pub generation: u64,
    pub durable_generation: Option<u64>,
    pub mutations: usize,
    pub total_duration: Duration,
    pub persistence: Measurement<Duration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionStats {
    pub dimension: usize,
    pub rows: usize,
    pub live_rows: usize,
    pub generation: u64,
    pub prepared_generation: Option<u64>,
    pub durable_generation: Option<u64>,
    pub recovered_interrupted_write: bool,
    pub closed: bool,
}

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
}

enum Backend {
    Cpu,
    #[cfg(feature = "usearch")]
    Usearch(usearch_backend::UsearchBackend),
    #[cfg(feature = "gpu-wgpu")]
    Wgpu(gpu::GpuBackend),
}

/// Embedded collection shared with `Arc<Collection>` across threads.
///
/// Ready CPU and USearch searches share a read lock. Mutations and rebuilds
/// take a write lock. GPU queries additionally serialize their scratch buffers.
/// Synchronous mutation/inspection methods block; use a blocking thread when
/// contending with async GPU work on a single-thread executor. No workers run.
pub struct Collection {
    inner: RwLock<CollectionState>,
    #[cfg(feature = "gpu-wgpu")]
    gpu_gate: async_lock::Mutex<()>,
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

    fn from_state(state: CollectionState) -> Self {
        Self {
            inner: RwLock::new(state),
            #[cfg(feature = "gpu-wgpu")]
            gpu_gate: async_lock::Mutex::new(()),
        }
    }

    /// Validate and add one row; durable collections sync before returning.
    pub fn add(&self, id: u64, user_id: u64, timestamp: i64, vector: &[f32]) -> Result<(), Error> {
        self.inner
            .write_blocking()
            .add(id, user_id, timestamp, vector)
    }

    /// Delete one row immediately and durably; IDs are never reused.
    pub fn delete(&self, id: u64) -> Result<(), Error> {
        self.inner.write_blocking().delete(id)
    }

    /// Commit ordered add/delete operations atomically. Errors roll back before publication.
    pub fn commit(&self, mutations: &[Mutation]) -> Result<CommitReport, Error> {
        self.inner.write_blocking().commit(mutations)
    }

    /// Add all rows atomically, rejecting duplicate IDs and invalid vectors.
    pub fn add_batch(&self, rows: &[NewRecord]) -> Result<CommitReport, Error> {
        self.inner.write_blocking().add_batch(rows)
    }

    /// Delete all IDs atomically; any invalid deletion rolls back the batch.
    pub fn delete_batch(&self, ids: &[u64]) -> Result<CommitReport, Error> {
        self.inner.write_blocking().delete_batch(ids)
    }

    /// Return live eligible IDs. Retains the original empty-on-closed behavior.
    pub fn filter(&self, filter: &Filter) -> Vec<u64> {
        self.inner.read_blocking().filter(filter)
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
        self.search_locked(query, filter, k)
            .instrument(info_span!("qenlo.search", operation = "search"))
            .await
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
        let mut preparation = Measurement::unavailable("index already current");
        if state.prepared_generation != Some(state.store.generation()) {
            drop(state);
            let waiting = Instant::now();
            let mut writer = self.inner.write().await;
            lock_wait += waiting.elapsed();
            let preparing = Instant::now();
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
        let mut response = state
            .search_inner(query, filter, k, rebuilt, preparation)
            .await?;
        response.report.lock_wait = lock_wait;
        response.report.total_duration = started.elapsed();
        Ok(response)
    }

    /// Run queries in order. Each query sees its own committed generation.
    pub async fn search_batch(
        &self,
        queries: &[&[f32]],
        filter: &Filter,
        k: usize,
    ) -> Result<Vec<SearchResponse>, Error> {
        let mut responses = Vec::with_capacity(queries.len());
        for query in queries {
            responses.push(self.search(query, filter, k).await?);
        }
        Ok(responses)
    }

    /// Inspect canonical and durable generations without scanning vectors.
    pub fn stats(&self) -> CollectionStats {
        self.inner.read_blocking().stats()
    }

    /// Sync pending canonical data. Normal durable mutations are already synced.
    pub fn flush(&self) -> Result<(), Error> {
        self.inner.write_blocking().flush()
    }

    /// Flush, mark closed, and release the exclusive filesystem lock. Idempotent.
    pub fn close(&self) -> Result<(), Error> {
        self.inner.write_blocking().close()
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

    /// Open the newest committed generation of a durable collection.
    ///
    /// The configured dimension must exactly match the stored dimension. Format
    /// versions are never inferred or migrated implicitly.
    #[tracing::instrument(name = "qenlo.open", skip_all, fields(backend = ?config.backend))]
    pub async fn open(path: impl AsRef<Path>, config: CollectionConfig) -> Result<Self, Error> {
        let path = path.as_ref().to_path_buf();
        let opened = storage::open(&path).map_err(|error| Error::Storage(error.to_string()))?;
        if opened.store.dimension() != config.dimension {
            return Err(CoreError::DimensionMismatch {
                expected: opened.store.dimension(),
                actual: config.dimension,
            }
            .into());
        }
        let durable_generation = opened.store.generation();
        let (backend, fallback_reason) = Self::initialize_backend(&config).await?;
        Ok(Self {
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
                    .map(|gpu| (Backend::Wgpu(gpu), None))
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
                Ok(gpu) => Ok((Backend::Wgpu(gpu), None)),
                Err(error) => Ok((Backend::Cpu, Some(error.to_string()))),
            },
        }
    }

    #[tracing::instrument(name = "qenlo.add", skip_all, fields(operation = "add"))]
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
        Ok(())
    }

    #[tracing::instrument(name = "qenlo.delete", skip_all, fields(operation = "delete"))]
    pub fn delete(&mut self, id: u64) -> Result<(), Error> {
        self.ensure_open()?;
        if self.path.is_some() {
            self.commit(&[Mutation::Delete(id)])?;
            return Ok(());
        }
        self.store.delete(id)?;
        self.prepared_generation = None;
        Ok(())
    }

    /// Validate an entire ordered batch, publish it durably, then make it visible.
    ///
    /// Any validation or pre-publication I/O error leaves the collection unchanged.
    /// An error syncing a published directory entry returns `CommitUncertain` and
    /// closes the handle; reopen to resolve the outcome. An empty batch is a no-op.
    #[tracing::instrument(name = "qenlo.commit", skip_all, fields(operation = "commit"))]
    pub fn commit(&mut self, mutations: &[Mutation]) -> Result<CommitReport, Error> {
        self.ensure_open()?;
        let started = Instant::now();
        let persistence = if mutations.is_empty() {
            Measurement::unavailable("empty transaction")
        } else {
            // ponytail: copy-on-commit costs one extra canonical store and O(n)
            // snapshot I/O; add a WAL only when measured write workloads need it.
            let mut staged = self.store.clone();
            for mutation in mutations {
                match mutation {
                    Mutation::Add(row) => {
                        staged.add(row.id, row.user_id, row.timestamp, &row.vector)?;
                    }
                    Mutation::Delete(id) => staged.delete(*id)?,
                }
            }
            let persistence = if let Some(path) = &self.path {
                let writing = Instant::now();
                if let Err(error) = storage::write_snapshot(path, &staged) {
                    if matches!(error, storage::StorageError::CommitUncertain(_)) {
                        self.closed = true;
                        self.storage_lock = None;
                        return Err(Error::CommitUncertain(error.to_string()));
                    }
                    return Err(Error::Storage(error.to_string()));
                }
                self.durable_generation = Some(staged.generation());
                Measurement::Available(writing.elapsed())
            } else {
                Measurement::unavailable("in-memory collection")
            };
            self.store = staged;
            self.prepared_generation = None;
            persistence
        };
        Ok(CommitReport {
            generation: self.store.generation(),
            durable_generation: self.durable_generation,
            mutations: mutations.len(),
            total_duration: started.elapsed(),
            persistence,
        })
    }

    /// Atomically add all rows; duplicate IDs within the batch are errors.
    pub fn add_batch(&mut self, rows: &[NewRecord]) -> Result<CommitReport, Error> {
        self.commit(&rows.iter().cloned().map(Mutation::Add).collect::<Vec<_>>())
    }

    /// Atomically delete all IDs; an unknown or already-deleted ID rolls back all.
    pub fn delete_batch(&mut self, ids: &[u64]) -> Result<CommitReport, Error> {
        self.commit(
            &ids.iter()
                .copied()
                .map(Mutation::Delete)
                .collect::<Vec<_>>(),
        )
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

    #[tracing::instrument(name = "qenlo.prepare", skip_all, fields(operation = "prepare"))]
    pub async fn prepare(&mut self) -> Result<bool, Error> {
        self.ensure_open()?;
        if self.prepared_generation == Some(self.store.generation()) {
            return Ok(false);
        }
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
            Backend::Wgpu(gpu) => gpu
                .search(&self.store, query, filter, k)
                .await
                .map_err(|e| Error::Search(e.to_string()))?,
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
            fallback_reason: self.fallback_reason.clone(),
            total_duration: started.elapsed(),
            phases: output.phases.with_preparation(preparation),
            upload_bytes: output.upload_bytes,
            readback_bytes: output.readback_bytes,
            dispatch_count: output.dispatch_count,
            qenlo_allocation_bytes: output.allocation_bytes,
            candidates: output.candidates,
            results: results.len(),
        };
        Ok(SearchResponse { results, report })
    }

    pub fn stats(&self) -> CollectionStats {
        CollectionStats {
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
        storage::write_snapshot(path, &self.store)
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
            allocation_bytes: Measurement::Available(0),
            candidates: Measurement::Available(candidates),
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
            let obstruction = path.join("canonical-00000000000000000001.pending");
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
    fn shared_collection_and_search_futures_are_send() {
        fn send_sync<T: Send + Sync>() {}
        fn send<T: Send>(_: T) {}
        send_sync::<Collection>();
        let collection = block_on(Collection::new(CollectionConfig::cpu_exact(2))).unwrap();
        send(collection.search(&[1.0, 0.0], &Filter::ALL, 1));
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
            collection.add(1, 7, 0, &[1.0, 0.0]).unwrap();
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
