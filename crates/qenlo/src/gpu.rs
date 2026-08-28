//! Experimental exact filtered search implemented with portable wgpu compute.
//!
//! The selector uses a parallel workgroup reduction for exact top-k. Vector and
//! metadata buffers stay resident; query scratch is allocated and released per chunk so the
//! configured budget is a hard bound on Qenlo-owned allocations.

use std::{
    fmt,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

const DEFAULT_BUDGET: u64 = 512 * 1024 * 1024;
const CANDIDATE_BYTES: u64 = 16;
const PARAM_BYTES: u64 = 48;
const WORKGROUP_SIZE: u64 = 256;
const SCORE_ROWS_PER_WORKGROUP: u64 = 8;
// ponytail: one workgroup selects each chunk's top-k; hierarchical selection if this dominates.
const MAX_CHUNK_ROWS: u64 = 131_072;
const DEVICE_TIMEOUT: Duration = Duration::from_secs(30);

/// Capabilities of the adapter selected for this collection, not physical VRAM usage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuCapabilities {
    /// Adapter's driver-reported name.
    pub adapter_name: String,
    /// Portable backend name, such as Vulkan or Dx12.
    pub backend: String,
    /// Driver-reported adapter category.
    pub device_type: String,
    /// Maximum individual buffer size.
    pub max_buffer_size: u64,
    /// Maximum storage binding size.
    pub max_storage_buffer_binding_size: u64,
    /// Maximum compute workgroups along one dimension.
    pub max_compute_workgroups_per_dimension: u32,
    /// Whether timestamp queries are supported; this prototype does not enable them.
    pub timestamp_queries_supported: bool,
    /// Configured bound on buffers and conservative transfer staging estimates.
    pub allocation_budget_bytes: u64,
}

#[derive(Clone, Default)]
struct DeviceHealth(Arc<Mutex<Option<String>>>);

impl DeviceHealth {
    fn fail(&self, reason: String) {
        let mut failure = self.0.lock().unwrap_or_else(|error| error.into_inner());
        failure.get_or_insert(reason);
    }

    fn check(&self) -> Result<(), GpuError> {
        match self
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
        {
            Some(reason) => Err(GpuError::Device(reason.clone())),
            None => Ok(()),
        }
    }

    fn install(device: &wgpu::Device) -> Self {
        let health = Self::default();
        let lost = health.clone();
        device.set_device_lost_callback(move |reason, message| {
            lost.fail(format!("device lost ({reason:?}): {message}"));
        });
        let uncaptured = health.clone();
        // wgpu's default handler panics. Make every driver/validation error sticky instead.
        device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
            uncaptured.fail(error.to_string());
        }));
        health
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FilterMode<'a> {
    /// One entry per collection row; `true` means eligible.
    Mask(&'a [bool]),
    /// Collection row numbers; duplicates are removed before dispatch.
    EligibleRows(&'a [u32]),
    /// All supplied user/time clauses joined by AND; absent clauses are unrestricted.
    Predicate {
        user: Option<u64>,
        lower: Option<i64>,
        upper: Option<i64>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GpuHit {
    pub id: u64,
    pub distance: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct GpuExecution {
    pub dispatch_count: u32,
    pub upload_bytes: u64,
    pub readback_bytes: u64,
    pub allocation_bytes: u64,
    pub chunks: u32,
}

#[derive(Debug)]
pub(crate) enum GpuError {
    Unavailable(String),
    InvalidInput(String),
    OverBudget { required: u64, budget: u64 },
    Device(String),
}

impl fmt::Display for GpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(reason) => write!(f, "GPU unavailable: {reason}"),
            Self::InvalidInput(reason) => write!(f, "invalid GPU search input: {reason}"),
            Self::OverBudget { required, budget } => {
                write!(
                    f,
                    "GPU allocation requires {required} bytes, budget is {budget} bytes"
                )
            }
            Self::Device(reason) => write!(f, "GPU execution failed: {reason}"),
        }
    }
}

impl std::error::Error for GpuError {}

struct Chunk {
    row_base: u32,
    rows: u32,
    vectors: wgpu::Buffer,
    users: wgpu::Buffer,
    timestamps: wgpu::Buffer,
    ids: wgpu::Buffer,
    live: Vec<u32>,
}

pub(crate) struct GpuExact {
    device: wgpu::Device,
    queue: wgpu::Queue,
    score_pipeline: wgpu::ComputePipeline,
    select_pipeline: wgpu::ComputePipeline,
    group0_layout: wgpu::BindGroupLayout,
    group1_layout: wgpu::BindGroupLayout,
    chunks: Vec<Chunk>,
    dimension: u32,
    rows: u32,
    budget: u64,
    resident_bytes: u64,
    health: DeviceHealth,
    capabilities: GpuCapabilities,
}

impl GpuExact {
    pub(crate) async fn new(
        dimension: usize,
        vectors: &[f32],
        ids: &[u64],
        users: &[u64],
        timestamps: &[i64],
        live: &[bool],
        budget: Option<u64>,
    ) -> Result<Self, GpuError> {
        if dimension == 0 || dimension > u32::MAX as usize {
            return Err(GpuError::InvalidInput(
                "dimension must fit a non-zero u32".into(),
            ));
        }
        let rows = ids.len();
        if rows == 0 || rows > u32::MAX as usize {
            return Err(GpuError::InvalidInput(
                "row count must fit a non-zero u32".into(),
            ));
        }
        if users.len() != rows
            || timestamps.len() != rows
            || live.len() != rows
            || rows.checked_mul(dimension) != Some(vectors.len())
        {
            return Err(GpuError::InvalidInput(
                "vector and metadata lengths disagree".into(),
            ));
        }

        let budget = budget.unwrap_or(DEFAULT_BUDGET);
        let row_bytes = dimension as u64 * 4 + 24;
        let resident_bytes = row_bytes
            .checked_mul(rows as u64)
            .ok_or_else(|| GpuError::InvalidInput("allocation size overflow".into()))?;
        let fixed_scratch = dimension as u64 * 8 + 64 * CANDIDATE_BYTES * 2 + PARAM_BYTES * 2;
        let minimum = resident_bytes
            .checked_add((fixed_scratch + 12).max(row_bytes))
            .ok_or_else(|| GpuError::InvalidInput("allocation size overflow".into()))?;
        ensure_budget(minimum, budget)?;
        let instance = gpu_instance();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
                ..Default::default()
            })
            .await
            .map_err(|e| GpuError::Unavailable(e.to_string()))?;
        let adapter_limits = adapter.limits();
        validate_limits(&adapter_limits, dimension)?;
        let capabilities = adapter_capabilities(&adapter, budget);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("qenlo exact-search device"),
                required_limits: adapter_limits.clone(),
                ..Default::default()
            })
            .await
            .map_err(|e| GpuError::Unavailable(e.to_string()))?;
        let health = DeviceHealth::install(&device);

        let group0_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qenlo exact group 0"),
            entries: &[
                storage_entry(0, true),
                storage_entry(1, true),
                storage_entry(2, true),
                storage_entry(3, true),
                storage_entry(4, true),
                storage_entry(5, true),
                storage_entry(6, false),
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let group1_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qenlo exact candidates"),
            entries: &[storage_entry(0, false)],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("qenlo exact pipeline layout"),
            bind_group_layouts: &[Some(&group0_layout), Some(&group1_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qenlo exact shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_exact.wgsl").into()),
        });
        let make_pipeline = |entry_point| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry_point),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let score_pipeline = make_pipeline("score");
        let select_pipeline = make_pipeline("select_topk");
        health.check()?;

        let vector_row_bytes = dimension as u64 * 4;
        let max_binding = adapter_limits
            .max_storage_buffer_binding_size
            .min(adapter_limits.max_buffer_size);
        let max_dispatch_rows =
            adapter_limits.max_compute_workgroups_per_dimension as u64 * SCORE_ROWS_PER_WORKGROUP;
        let rows_per_chunk = (max_binding / vector_row_bytes)
            .min(max_binding / 8)
            .min(max_dispatch_rows)
            .min(MAX_CHUNK_ROWS)
            .min(u32::MAX as u64 / dimension as u64)
            // Reserve upload staging during rebuild and worst-case k=64 query scratch.
            .min((budget - resident_bytes - fixed_scratch) / 12)
            .min((budget - resident_bytes) / row_bytes)
            .min(u32::MAX as u64) as usize;
        if rows_per_chunk == 0 {
            return Err(GpuError::Unavailable(
                "adapter limits cannot hold one vector row".into(),
            ));
        }

        let mut chunks = Vec::new();
        for start in (0..rows).step_by(rows_per_chunk) {
            let end = (start + rows_per_chunk).min(rows);
            let packed_ids = pack_u64(&ids[start..end]);
            let packed_users = pack_u64(&users[start..end]);
            let packed_timestamps = pack_i64(&timestamps[start..end]);
            chunks.push(Chunk {
                row_base: start as u32,
                rows: (end - start) as u32,
                vectors: uploaded(
                    &device,
                    &queue,
                    "qenlo vectors",
                    &vectors[start * dimension..end * dimension],
                    &health,
                )?,
                users: uploaded(&device, &queue, "qenlo users", &packed_users, &health)?,
                timestamps: uploaded(
                    &device,
                    &queue,
                    "qenlo timestamps",
                    &packed_timestamps,
                    &health,
                )?,
                ids: uploaded(&device, &queue, "qenlo ids", &packed_ids, &health)?,
                live: live[start..end]
                    .iter()
                    .map(|value| u32::from(*value))
                    .collect(),
            });
            queue.submit([]);
            poll_wait(&device)?;
            health.check()?;
        }

        Ok(Self {
            device,
            queue,
            score_pipeline,
            select_pipeline,
            group0_layout,
            group1_layout,
            chunks,
            dimension: dimension as u32,
            rows: rows as u32,
            budget,
            resident_bytes,
            health,
            capabilities,
        })
    }

    pub(crate) async fn search(
        &self,
        query: &[f32],
        k: usize,
        filter: FilterMode<'_>,
    ) -> Result<(Vec<GpuHit>, GpuExecution), GpuError> {
        self.health.check()?;
        if query.len() != self.dimension as usize {
            return Err(GpuError::InvalidInput("query dimension mismatch".into()));
        }
        if query.iter().any(|value| !value.is_finite()) {
            return Err(GpuError::InvalidInput("query must be finite".into()));
        }
        if !(1..=64).contains(&k) {
            return Err(GpuError::InvalidInput("k must be in 1..=64".into()));
        }
        if let FilterMode::Mask(mask) = filter
            && mask.len() != self.rows as usize
        {
            return Err(GpuError::InvalidInput(
                "mask length must equal row count".into(),
            ));
        }
        if let FilterMode::EligibleRows(rows) = filter
            && rows.iter().any(|&row| row >= self.rows)
        {
            return Err(GpuError::InvalidInput(
                "eligible row is out of bounds".into(),
            ));
        }

        let mut execution = GpuExecution {
            allocation_bytes: self.resident_bytes,
            chunks: self.chunks.len() as u32,
            ..Default::default()
        };
        let mut merged = Vec::with_capacity(k * 2);

        for chunk in &self.chunks {
            let (mode, eligibility) = chunk_eligibility(filter, chunk);
            let dispatch_items = if mode == 1 {
                eligibility.len() as u32
            } else {
                chunk.rows
            };
            let eligibility = if eligibility.is_empty() {
                vec![0]
            } else {
                eligibility
            };
            let params = params(
                mode,
                chunk,
                self.dimension,
                k as u32,
                dispatch_items,
                filter,
            );
            let scratch = query.len() as u64 * 8
                + eligibility.len() as u64 * 8
                + chunk.rows as u64 * 4
                + k as u64 * CANDIDATE_BYTES * 2
                + PARAM_BYTES * 2;
            ensure_budget(self.resident_bytes + scratch, self.budget)?;
            execution.allocation_bytes = execution
                .allocation_bytes
                .max(self.resident_bytes + scratch);
            execution.upload_bytes +=
                query.len() as u64 * 4 + eligibility.len() as u64 * 4 + PARAM_BYTES;
            execution.readback_bytes += k as u64 * CANDIDATE_BYTES;

            let query_buffer = uploaded(
                &self.device,
                &self.queue,
                "qenlo query",
                query,
                &self.health,
            )?;
            let eligibility_buffer = uploaded(
                &self.device,
                &self.queue,
                "qenlo eligibility",
                &eligibility,
                &self.health,
            )?;
            let params_buffer = uploaded_uniform(
                &self.device,
                &self.queue,
                "qenlo params",
                &params,
                &self.health,
            )?;
            let scores = buffer(
                &self.device,
                "qenlo scores",
                chunk.rows as u64 * 4,
                wgpu::BufferUsages::STORAGE,
                &self.health,
            )?;
            let selected = buffer(
                &self.device,
                "qenlo candidates",
                k as u64 * CANDIDATE_BYTES,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                &self.health,
            )?;
            let staging = buffer(
                &self.device,
                "qenlo readback",
                k as u64 * CANDIDATE_BYTES,
                wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                &self.health,
            )?;

            let group0 = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("qenlo exact resources"),
                layout: &self.group0_layout,
                entries: &[
                    binding(0, &chunk.vectors),
                    binding(1, &chunk.users),
                    binding(2, &chunk.timestamps),
                    binding(3, &chunk.ids),
                    binding(4, &query_buffer),
                    binding(5, &eligibility_buffer),
                    binding(6, &scores),
                    binding(7, &params_buffer),
                ],
            });
            let group1 = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("qenlo candidate output"),
                layout: &self.group1_layout,
                entries: &[binding(0, &selected)],
            });
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("qenlo exact commands"),
                });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("qenlo exact search"),
                    timestamp_writes: None,
                });
                pass.set_bind_group(0, &group0, &[]);
                pass.set_bind_group(1, &group1, &[]);
                if dispatch_items != 0 {
                    pass.set_pipeline(&self.score_pipeline);
                    pass.dispatch_workgroups(
                        dispatch_items.div_ceil(SCORE_ROWS_PER_WORKGROUP as u32),
                        1,
                        1,
                    );
                }
                pass.set_pipeline(&self.select_pipeline);
                pass.dispatch_workgroups(1, 1, 1);
            }
            encoder.copy_buffer_to_buffer(&selected, 0, &staging, 0, k as u64 * CANDIDATE_BYTES);
            self.queue.submit([encoder.finish()]);
            self.health.check()?;
            execution.dispatch_count += if dispatch_items == 0 { 1 } else { 2 };

            map_wait(&self.device, &staging).inspect_err(|error| {
                self.health.fail(error.to_string());
                self.device.destroy();
            })?;
            self.health.check()?;
            let mapped = staging
                .slice(..)
                .get_mapped_range()
                .map_err(|error| GpuError::Device(error.to_string()))?;
            for bytes in mapped.as_chunks::<16>().0 {
                let distance = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
                let row = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
                let id = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
                if row != u32::MAX && distance.is_finite() {
                    merged.push(GpuHit { id, distance });
                }
            }
            drop(mapped);
            staging.unmap();
            merged.sort_by(|a, b| {
                a.distance
                    .total_cmp(&b.distance)
                    .then_with(|| a.id.cmp(&b.id))
            });
            merged.dedup_by_key(|hit| hit.id);
            merged.truncate(k);
        }

        merged.sort_by(|a, b| {
            a.distance
                .total_cmp(&b.distance)
                .then_with(|| a.id.cmp(&b.id))
        });
        merged.dedup_by_key(|hit| hit.id);
        merged.truncate(k);
        Ok((merged, execution))
    }
}

/// Adapter expected by the public collection. `prepare` rebuilds all resident derived buffers.
pub(crate) struct GpuBackend {
    dimension: usize,
    mode: super::GpuFilterMode,
    budget: u64,
    exact: Option<Box<GpuExact>>,
    capabilities: GpuCapabilities,
}

impl GpuBackend {
    pub(crate) async fn new(
        dimension: usize,
        mode: super::GpuFilterMode,
        budget: u64,
    ) -> Result<Self, GpuError> {
        if dimension == 0 || dimension > u32::MAX as usize {
            return Err(GpuError::InvalidInput(
                "dimension must fit a non-zero u32".into(),
            ));
        }
        // Probe now so `WgpuRequired` fails at collection construction rather than first search.
        let adapter = gpu_instance()
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
                ..Default::default()
            })
            .await
            .map_err(|error| GpuError::Unavailable(error.to_string()))?;
        validate_limits(&adapter.limits(), dimension)?;
        let capabilities = adapter_capabilities(&adapter, budget);
        Ok(Self {
            dimension,
            mode,
            budget,
            exact: None,
            capabilities,
        })
    }

    pub(crate) fn capabilities(&self) -> &GpuCapabilities {
        self.exact
            .as_ref()
            .map_or(&self.capabilities, |exact| &exact.capabilities)
    }

    pub(crate) fn is_healthy(&self) -> bool {
        self.exact
            .as_ref()
            .is_none_or(|exact| exact.health.check().is_ok())
    }

    #[cfg(test)]
    pub(crate) fn destroy_device_for_test(&self) {
        if let Some(exact) = &self.exact {
            exact.device.destroy();
            let _ = exact.device.poll(wgpu::PollType::Poll);
        }
    }

    pub(crate) async fn prepare(&mut self, store: &qenlo_core::CoreStore) -> Result<(), GpuError> {
        // Canonical storage is authoritative; release old residents before allocating replacements.
        if let Some(old) = self.exact.take() {
            old.device.destroy();
            drop(old);
        }
        if store.is_empty() {
            self.exact = None;
            return Ok(());
        }
        let mut vectors = Vec::with_capacity(store.len() * self.dimension);
        let mut ids = Vec::with_capacity(store.len());
        let mut users = Vec::with_capacity(store.len());
        let mut timestamps = Vec::with_capacity(store.len());
        let mut live = Vec::with_capacity(store.len());
        for (_, record) in store.records() {
            vectors.extend_from_slice(record.vector());
            ids.push(record.id());
            users.push(record.user_id());
            timestamps.push(record.timestamp());
            live.push(record.is_live());
        }
        self.exact = Some(Box::new(
            GpuExact::new(
                self.dimension,
                &vectors,
                &ids,
                &users,
                &timestamps,
                &live,
                Some(self.budget),
            )
            .await?,
        ));
        Ok(())
    }

    pub(crate) async fn search(
        &self,
        store: &qenlo_core::CoreStore,
        query: &[f32],
        predicate: &qenlo_core::Predicate,
        k: usize,
    ) -> Result<super::BackendOutput, GpuError> {
        let started = web_time::Instant::now();
        let normalized = qenlo_core::normalize_vector(query, self.dimension)
            .map_err(|error| GpuError::InvalidInput(error.to_string()))?;
        let filter = match self.mode {
            super::GpuFilterMode::CpuMask => {
                let eligible = store.filter(predicate);
                let mut mask = vec![false; store.len()];
                for &row in &eligible {
                    mask[row as usize] = true;
                }
                // Keep the owned input alive across the awaited search.
                return self
                    .search_owned(
                        store,
                        &normalized,
                        k,
                        eligible.len(),
                        OwnedFilter::Mask(mask),
                        started,
                    )
                    .await;
            }
            super::GpuFilterMode::CpuEligibleRows => {
                let eligible = store.filter(predicate);
                return self
                    .search_owned(
                        store,
                        &normalized,
                        k,
                        eligible.len(),
                        OwnedFilter::Rows(eligible),
                        started,
                    )
                    .await;
            }
            super::GpuFilterMode::GpuPredicate => FilterMode::Predicate {
                user: predicate.user_id,
                lower: predicate.timestamp.lower,
                upper: predicate.timestamp.upper,
            },
        };
        self.finish_search(&normalized, k, filter, None, started)
            .await
    }

    async fn search_owned(
        &self,
        _store: &qenlo_core::CoreStore,
        query: &[f32],
        k: usize,
        candidates: usize,
        filter: OwnedFilter,
        started: web_time::Instant,
    ) -> Result<super::BackendOutput, GpuError> {
        let borrowed = match &filter {
            OwnedFilter::Mask(mask) => FilterMode::Mask(mask),
            OwnedFilter::Rows(rows) => FilterMode::EligibleRows(rows),
        };
        self.finish_search(query, k, borrowed, Some(candidates as u64), started)
            .await
    }

    async fn finish_search(
        &self,
        query: &[f32],
        k: usize,
        filter: FilterMode<'_>,
        candidates: Option<u64>,
        started: web_time::Instant,
    ) -> Result<super::BackendOutput, GpuError> {
        let Some(exact) = &self.exact else {
            return Ok(super::BackendOutput {
                hits: Vec::new(),
                actual_backend: super::BackendKind::Wgpu,
                algorithm: super::Algorithm::Exact,
                filter_execution: super::FilterExecution::Gpu(self.mode),
                phases: gpu_phases(started.elapsed()),
                upload_bytes: super::Measurement::Available(0),
                readback_bytes: super::Measurement::Available(0),
                dispatch_count: super::Measurement::Available(0),
                allocation_bytes: super::Measurement::Available(0),
                candidates: super::Measurement::Available(0),
            });
        };
        let (hits, execution) = exact.search(query, k, filter).await?;
        Ok(super::BackendOutput {
            hits: hits.into_iter().map(|hit| qenlo_core::SearchHit { id: hit.id, distance: hit.distance }).collect(),
            actual_backend: super::BackendKind::Wgpu,
            algorithm: super::Algorithm::Exact,
            filter_execution: super::FilterExecution::Gpu(self.mode),
            phases: gpu_phases(started.elapsed()),
            upload_bytes: super::Measurement::Available(execution.upload_bytes),
            readback_bytes: super::Measurement::Available(execution.readback_bytes),
            dispatch_count: super::Measurement::Available(execution.dispatch_count),
            allocation_bytes: super::Measurement::Available(execution.allocation_bytes),
            candidates: candidates.map_or_else(
                || super::Measurement::Unavailable(super::Unavailable {
                    reason: "basic diagnostics do not scan the GPU predicate result to count candidates".into(),
                }),
                super::Measurement::Available,
            ),
        })
    }
}

enum OwnedFilter {
    Mask(Vec<bool>),
    Rows(Vec<u32>),
}

fn gpu_phases(execution: web_time::Duration) -> super::PhaseTimings {
    super::PhaseTimings {
        preparation: super::Measurement::Unavailable(super::Unavailable {
            reason: "set by collection".into(),
        }),
        filtering: super::Measurement::Unavailable(super::Unavailable {
            reason: "included in GPU execution".into(),
        }),
        upload: super::Measurement::Unavailable(super::Unavailable {
            reason: "phase timing not yet instrumented".into(),
        }),
        execution: super::Measurement::Available(execution),
        readback: super::Measurement::Unavailable(super::Unavailable {
            reason: "included in GPU execution".into(),
        }),
        selection: super::Measurement::Unavailable(super::Unavailable {
            reason: "included in GPU execution".into(),
        }),
    }
}

fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn binding(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

fn buffer(
    device: &wgpu::Device,
    label: &str,
    size: u64,
    usage: wgpu::BufferUsages,
    health: &DeviceHealth,
) -> Result<wgpu::Buffer, GpuError> {
    health.check()?;
    let result = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage,
        mapped_at_creation: false,
    });
    health.check()?;
    Ok(result)
}

fn uploaded<T: Copy>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    value: &[T],
    health: &DeviceHealth,
) -> Result<wgpu::Buffer, GpuError> {
    let bytes = bytes_of(value);
    let result = buffer(
        device,
        label,
        bytes.len() as u64,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        health,
    )?;
    queue.write_buffer(&result, 0, bytes);
    health.check()?;
    Ok(result)
}

fn uploaded_uniform(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    value: &[u32; 12],
    health: &DeviceHealth,
) -> Result<wgpu::Buffer, GpuError> {
    let bytes = bytes_of(value);
    let result = buffer(
        device,
        label,
        PARAM_BYTES,
        wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        health,
    )?;
    queue.write_buffer(&result, 0, bytes);
    health.check()?;
    Ok(result)
}

fn bytes_of<T: Copy>(value: &[T]) -> &[u8] {
    // SAFETY: u8 has alignment one and the returned view cannot outlive the source slice.
    unsafe { std::slice::from_raw_parts(value.as_ptr().cast(), std::mem::size_of_val(value)) }
}

fn pack_u64(values: &[u64]) -> Vec<u32> {
    values
        .iter()
        .flat_map(|v| [*v as u32, (*v >> 32) as u32])
        .collect()
}

fn pack_i64(values: &[i64]) -> Vec<u32> {
    values
        .iter()
        .flat_map(|v| {
            let bits = *v as u64;
            [bits as u32, (bits >> 32) as u32]
        })
        .collect()
}

fn split(value: u64) -> (u32, u32) {
    (value as u32, (value >> 32) as u32)
}

fn chunk_eligibility(filter: FilterMode<'_>, chunk: &Chunk) -> (u32, Vec<u32>) {
    let start = chunk.row_base as usize;
    let end = start + chunk.rows as usize;
    match filter {
        FilterMode::Mask(mask) => (
            0,
            mask[start..end]
                .iter()
                .zip(&chunk.live)
                .map(|(eligible, live)| u32::from(*eligible) & live)
                .collect(),
        ),
        FilterMode::EligibleRows(rows) => {
            let mut local: Vec<_> = rows
                .iter()
                .copied()
                .filter(|row| (*row as usize) >= start && (*row as usize) < end)
                .map(|row| row - chunk.row_base)
                .filter(|row| chunk.live[*row as usize] != 0)
                .collect();
            // Duplicate rows would cause concurrent shader writes and unbounded dispatch sizes.
            local.sort_unstable();
            local.dedup();
            (1, local)
        }
        FilterMode::Predicate { .. } => (2, chunk.live.clone()),
    }
}

fn params(
    mode: u32,
    chunk: &Chunk,
    dimension: u32,
    k: u32,
    eligible_count: u32,
    filter: FilterMode<'_>,
) -> [u32; 12] {
    let (user, lower, upper, flags) = match filter {
        FilterMode::Predicate { user, lower, upper } => (
            user.unwrap_or(0),
            lower.unwrap_or(0) as u64,
            upper.unwrap_or(0) as u64,
            u32::from(user.is_some())
                | (u32::from(lower.is_some()) << 1)
                | (u32::from(upper.is_some()) << 2),
        ),
        _ => (0, 0, 0, 0),
    };
    let (ul, uh) = split(user);
    let (ll, lh) = split(lower);
    let (xl, xh) = split(upper);
    [
        chunk.rows,
        dimension,
        mode,
        eligible_count,
        ul,
        uh,
        ll,
        lh,
        xl,
        xh,
        flags,
        k,
    ]
}

fn ensure_budget(required: u64, budget: u64) -> Result<(), GpuError> {
    if required > budget {
        Err(GpuError::OverBudget { required, budget })
    } else {
        Ok(())
    }
}

fn gpu_instance() -> wgpu::Instance {
    // Honor WGPU_BACKEND for reproducible DX12/Vulkan runtime verification.
    wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env())
}

fn validate_limits(limits: &wgpu::Limits, dimension: usize) -> Result<(), GpuError> {
    if limits.max_storage_buffers_per_shader_stage < 8
        || limits.max_bind_groups < 2
        || limits.max_bindings_per_bind_group < 8
        || limits.max_compute_workgroup_size_x < WORKGROUP_SIZE as u32
        || limits.max_compute_invocations_per_workgroup < WORKGROUP_SIZE as u32
        || limits.max_compute_workgroup_storage_size < (WORKGROUP_SIZE * CANDIDATE_BYTES) as u32
        || limits.max_compute_workgroups_per_dimension == 0
        || limits.max_storage_buffer_binding_size < 64 * CANDIDATE_BYTES
        || limits.max_buffer_size < 64 * CANDIDATE_BYTES
        || limits.max_uniform_buffer_binding_size < PARAM_BYTES
        || limits
            .max_storage_buffer_binding_size
            .min(limits.max_buffer_size)
            < dimension as u64 * 4
    {
        Err(GpuError::Unavailable(
            "adapter limits cannot run the exact-search kernel".into(),
        ))
    } else {
        Ok(())
    }
}

fn adapter_capabilities(adapter: &wgpu::Adapter, budget: u64) -> GpuCapabilities {
    let info = adapter.get_info();
    let limits = adapter.limits();
    GpuCapabilities {
        adapter_name: info.name,
        backend: format!("{:?}", info.backend),
        device_type: format!("{:?}", info.device_type),
        max_buffer_size: limits.max_buffer_size,
        max_storage_buffer_binding_size: limits.max_storage_buffer_binding_size,
        max_compute_workgroups_per_dimension: limits.max_compute_workgroups_per_dimension,
        timestamp_queries_supported: adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY),
        allocation_budget_bytes: budget,
    }
}

fn poll_wait(device: &wgpu::Device) -> Result<(), GpuError> {
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(DEVICE_TIMEOUT),
        })
        .map(|_| ())
        .map_err(|error| GpuError::Device(error.to_string()))
}

fn map_wait(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Result<(), GpuError> {
    let started = web_time::Instant::now();
    let (sender, receiver) = mpsc::sync_channel(1);
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    poll_wait(device)?;
    receiver
        .recv_timeout(DEVICE_TIMEOUT.saturating_sub(started.elapsed()))
        .map_err(|e| GpuError::Device(e.to_string()))?
        .map_err(|e| GpuError::Device(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        future::Future,
        pin::pin,
        task::{Context, Poll, Waker},
    };

    fn block_on<T>(future: impl Future<Output = T>) -> T {
        let mut future = pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(value) => return value,
                Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    fn available(result: Result<GpuExact, GpuError>) -> Option<GpuExact> {
        match result {
            Ok(exact) => {
                eprintln!(
                    "GPU runtime: {} / {}",
                    exact.capabilities.adapter_name, exact.capabilities.backend
                );
                Some(exact)
            }
            Err(GpuError::Unavailable(reason))
                if std::env::var_os("QENLO_REQUIRE_GPU").is_none() =>
            {
                eprintln!(
                    "SKIP GPU runtime: {reason}; set QENLO_REQUIRE_GPU=1 to require hardware"
                );
                None
            }
            Err(error) => panic!("GPU initialization failed: {error}"),
        }
    }

    fn tiny_gpu() -> Option<GpuExact> {
        available(block_on(GpuExact::new(
            2,
            &[1.0, 0.0],
            &[1],
            &[1],
            &[0],
            &[true],
            None,
        )))
    }

    #[test]
    fn signed_i64_word_order_reference_handles_extremes() {
        fn key(value: i64) -> (u32, u32) {
            let bits = value as u64;
            ((bits >> 32) as u32 ^ 0x8000_0000, bits as u32)
        }
        let values = [i64::MIN, -1, 0, 1, i64::MAX];
        assert!(values.windows(2).all(|pair| key(pair[0]) < key(pair[1])));
    }

    #[test]
    fn packed_words_round_trip() {
        let values = [0, 1, u64::MAX, 0x1234_5678_9abc_def0];
        let packed = pack_u64(&values);
        let decoded: Vec<_> = packed
            .as_chunks::<2>()
            .0
            .iter()
            .map(|v| v[0] as u64 | ((v[1] as u64) << 32))
            .collect();
        assert_eq!(decoded, values);
    }

    #[test]
    fn budget_failure_is_explicit() {
        assert!(matches!(
            ensure_budget(11, 10),
            Err(GpuError::OverBudget {
                required: 11,
                budget: 10
            })
        ));
    }

    #[test]
    fn gpu_exact_mask_smoke_if_adapter_is_available() {
        let Some(exact) = available(block_on(GpuExact::new(
            2,
            &[1.0, 0.0, 0.0, 1.0],
            &[20, 10],
            &[1, 1],
            &[0, 0],
            &[true, true],
            Some(16 * 1024 * 1024),
        ))) else {
            return;
        };
        let (hits, report) =
            block_on(exact.search(&[1.0, 0.0], 2, FilterMode::Mask(&[true, true])))
                .expect("available adapter completes exact search");
        assert_eq!(hits.iter().map(|hit| hit.id).collect::<Vec<_>>(), [20, 10]);
        assert_eq!(report.readback_bytes, 32);

        let (hits, _) = block_on(exact.search(&[1.0, 0.0], 2, FilterMode::EligibleRows(&[1])))
            .expect("eligible-list search completes");
        assert_eq!(hits.iter().map(|hit| hit.id).collect::<Vec<_>>(), [10]);

        let (hits, _) = block_on(exact.search(
            &[1.0, 0.0],
            2,
            FilterMode::Predicate {
                user: Some(1),
                lower: Some(0),
                upper: Some(1),
            },
        ))
        .expect("GPU predicate search completes");
        assert_eq!(hits.iter().map(|hit| hit.id).collect::<Vec<_>>(), [20, 10]);
    }

    #[test]
    fn gpu_predicate_handles_signed_timestamps_and_tombstones_if_adapter_is_available() {
        let Some(exact) = available(block_on(GpuExact::new(
            2,
            &[1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0],
            &[1, 2, 3, 4],
            &[7, 7, 7, 7],
            &[i64::MIN, -1, 1, i64::MAX],
            &[true, true, false, true],
            Some(16 * 1024 * 1024),
        ))) else {
            return;
        };
        let (hits, _) = block_on(exact.search(
            &[1.0, 0.0],
            4,
            FilterMode::Predicate {
                user: Some(7),
                lower: Some(-2),
                upper: Some(2),
            },
        ))
        .expect("GPU predicate search completes");
        assert_eq!(hits.iter().map(|hit| hit.id).collect::<Vec<_>>(), [2]);
    }

    #[test]
    fn all_optional_predicates_match_cpu_including_signed_extremes() {
        let timestamps = [i64::MIN, i64::MIN + 1, -1, 0, 1, i64::MAX - 1, i64::MAX];
        let ids = [u64::MAX, 1, 2, 3, 4, 5, 1 << 32];
        let users = [u64::MAX, 0, 1, u64::MAX, 1, 0, u64::MAX];
        let live = [true, true, false, true, true, true, true];
        let vectors: Vec<_> = timestamps.iter().flat_map(|_| [1.0, 0.0]).collect();
        let Some(exact) = available(block_on(GpuExact::new(
            2,
            &vectors,
            &ids,
            &users,
            &timestamps,
            &live,
            None,
        ))) else {
            return;
        };
        let bounds = [
            None,
            Some(i64::MIN),
            Some(-1),
            Some(0),
            Some(1),
            Some(i64::MAX),
        ];
        for user in [None, Some(0), Some(1), Some(u64::MAX)] {
            for lower in bounds {
                for upper in bounds {
                    let mask: Vec<_> = (0..ids.len())
                        .map(|row| {
                            live[row]
                                && user.is_none_or(|user| users[row] == user)
                                && lower.is_none_or(|lower| timestamps[row] >= lower)
                                && upper.is_none_or(|upper| timestamps[row] < upper)
                        })
                        .collect();
                    let rows: Vec<_> = mask
                        .iter()
                        .enumerate()
                        .filter(|(_, live)| **live)
                        .flat_map(|(row, _)| [row as u32, row as u32])
                        .collect();
                    let mut expected: Vec<_> = mask
                        .iter()
                        .enumerate()
                        .filter(|(_, live)| **live)
                        .map(|(row, _)| ids[row])
                        .collect();
                    expected.sort_unstable();
                    for filter in [
                        FilterMode::Predicate { user, lower, upper },
                        FilterMode::Mask(&mask),
                        FilterMode::EligibleRows(&rows),
                    ] {
                        let (hits, report) = block_on(exact.search(&[1.0, 0.0], 8, filter))
                            .expect("exact predicate execution");
                        assert_eq!(
                            hits.iter().map(|hit| hit.id).collect::<Vec<_>>(),
                            expected,
                            "{filter:?}"
                        );
                        assert!(report.allocation_bytes <= exact.budget);
                        assert_eq!(report.readback_bytes, 8 * CANDIDATE_BYTES);
                    }
                }
            }
        }
        // Masks/lists cannot resurrect deleted rows even if the caller includes them.
        for filter in [
            FilterMode::Mask(&[true; 7]),
            FilterMode::EligibleRows(&[2, 2]),
        ] {
            let (hits, _) = block_on(exact.search(&[1.0, 0.0], 8, filter)).unwrap();
            assert!(!hits.iter().any(|hit| hit.id == ids[2]));
        }
    }

    #[test]
    fn chunks_keep_readback_and_allocations_bounded() {
        let rows = MAX_CHUNK_ROWS as usize + 1;
        let ids: Vec<_> = (1..=rows as u64).rev().collect();
        let Some(exact) = available(block_on(GpuExact::new(
            1,
            &vec![1.0; rows],
            &ids,
            &vec![0; rows],
            &vec![0; rows],
            &vec![true; rows],
            Some(16 * 1024 * 1024),
        ))) else {
            return;
        };
        assert_eq!(exact.chunks.len(), 2);
        let (hits, report) = block_on(exact.search(
            &[1.0],
            3,
            FilterMode::Predicate {
                user: None,
                lower: None,
                upper: None,
            },
        ))
        .unwrap();
        assert_eq!(hits.iter().map(|hit| hit.id).collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(report.readback_bytes, 2 * 3 * CANDIDATE_BYTES);
        assert!(report.allocation_bytes <= exact.budget);
    }

    #[test]
    fn parallel_scoring_and_selection_match_f64_oracle() {
        // Exercise partial workgroups, dimensions either side of the 32-lane boundary,
        // maximum k, unsigned IDs above 2^32, and selection after sparse row compaction.
        let rows = 521;
        let ids: Vec<_> = (0..rows)
            .map(|row| u64::MAX - row as u64 * 0x100000001)
            .collect();
        let users: Vec<_> = (0..rows).map(|row| (1u64 << 40) + row as u64 % 3).collect();
        let timestamps: Vec<_> = (0..rows).map(|row| row as i64 - 260).collect();
        let live: Vec<_> = (0..rows).map(|row| row % 13 != 0).collect();
        let mask: Vec<_> = (0..rows).map(|row| row % 5 < 3).collect();
        let eligible: Vec<_> = (0..rows)
            .filter(|&row| mask[row])
            .flat_map(|row| [row as u32; 2])
            .collect();
        let mut random = 0x9e3779b97f4a7c15u64;
        for dimension in [1, 7, 31, 32, 33, 128, 384, 768] {
            let mut next_vector = || {
                let raw: Vec<_> = (0..dimension)
                    .map(|_| {
                        random ^= random << 13;
                        random ^= random >> 7;
                        random ^= random << 17;
                        (random as u32 as f64 / u32::MAX as f64 * 2.0 - 1.0) as f32
                    })
                    .collect();
                qenlo_core::normalize_vector(&raw, dimension).unwrap()
            };
            let vectors: Vec<_> = (0..rows).flat_map(|_| next_vector()).collect();
            let query = next_vector();
            let Some(exact) = available(block_on(GpuExact::new(
                dimension,
                &vectors,
                &ids,
                &users,
                &timestamps,
                &live,
                None,
            ))) else {
                return;
            };
            for filter in [
                FilterMode::Mask(&mask),
                FilterMode::EligibleRows(&eligible),
                FilterMode::EligibleRows(&[]),
                FilterMode::EligibleRows(&[1, 2]),
                FilterMode::Predicate {
                    user: None,
                    lower: None,
                    upper: None,
                },
                FilterMode::Predicate {
                    user: Some((1u64 << 40) + 1),
                    lower: Some(-190),
                    upper: Some(190),
                },
            ] {
                let mut expected: Vec<_> = (0..rows)
                    .filter(|&row| {
                        live[row]
                            && match filter {
                                FilterMode::Mask(mask) => mask[row],
                                FilterMode::EligibleRows(eligible) => {
                                    eligible.contains(&(row as u32))
                                }
                                FilterMode::Predicate { user, lower, upper } => {
                                    user.is_none_or(|user| users[row] == user)
                                        && lower.is_none_or(|lower| timestamps[row] >= lower)
                                        && upper.is_none_or(|upper| timestamps[row] < upper)
                                }
                            }
                    })
                    .map(|row| {
                        let dot: f64 = vectors[row * dimension..(row + 1) * dimension]
                            .iter()
                            .zip(&query)
                            .map(|(&v, &q)| v as f64 * q as f64)
                            .sum();
                        (ids[row], 1.0 - dot)
                    })
                    .collect();
                expected.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
                for k in [1, 10, 64] {
                    let (hits, report) = block_on(exact.search(&query, k, filter)).unwrap();
                    let expected = &expected[..k.min(expected.len())];
                    assert_eq!(hits.len(), expected.len());
                    for (hit, &(id, distance)) in hits.iter().zip(expected) {
                        assert_eq!(
                            hit.id, id,
                            "dimension={dimension}, k={k}, filter={filter:?}"
                        );
                        assert!((hit.distance as f64 - distance).abs() < 2e-6);
                    }
                    assert_eq!(report.readback_bytes, k as u64 * CANDIDATE_BYTES);
                    assert!(report.allocation_bytes <= exact.budget);
                }
            }
        }
    }

    #[test]
    fn real_device_destroy_returns_sticky_error() {
        let Some(exact) = tiny_gpu() else {
            return;
        };
        exact.device.destroy();
        let _ = exact.device.poll(wgpu::PollType::Poll);
        assert!(matches!(
            block_on(exact.search(&[1.0, 0.0], 1, FilterMode::Mask(&[true]))),
            Err(GpuError::Device(_))
        ));
        assert!(matches!(exact.health.check(), Err(GpuError::Device(_))));
    }

    #[test]
    fn allocation_validation_is_returned_instead_of_uncaptured_panic() {
        let Some(exact) = tiny_gpu() else {
            return;
        };
        let result = buffer(
            &exact.device,
            "deliberately invalid allocation",
            exact.device.limits().max_buffer_size + 4,
            wgpu::BufferUsages::STORAGE,
            &exact.health,
        );
        assert!(matches!(result, Err(GpuError::Device(_))));
    }

    #[test]
    fn initialization_and_search_reject_overbudget() {
        assert!(matches!(
            block_on(GpuExact::new(
                2,
                &[1.0, 0.0],
                &[1],
                &[1],
                &[0],
                &[true],
                Some(1)
            )),
            Err(GpuError::OverBudget { .. })
        ));
        let Some(mut exact) = tiny_gpu() else {
            return;
        };
        exact.budget = exact.resident_bytes;
        assert!(matches!(
            block_on(exact.search(&[1.0, 0.0], 1, FilterMode::Mask(&[true]))),
            Err(GpuError::OverBudget { .. })
        ));
    }

    #[test]
    fn unsupported_adapter_limits_are_unavailable() {
        let limits = wgpu::Limits {
            max_storage_buffers_per_shader_stage: 7,
            ..wgpu::Limits::default()
        };
        assert!(matches!(
            validate_limits(&limits, 2),
            Err(GpuError::Unavailable(_))
        ));
    }

    #[test]
    fn failed_rebuild_releases_old_residents_and_can_retry() {
        let Some(exact) = tiny_gpu() else {
            return;
        };
        let mut backend = GpuBackend {
            dimension: 2,
            mode: super::super::GpuFilterMode::GpuPredicate,
            budget: 1,
            capabilities: exact.capabilities.clone(),
            exact: Some(Box::new(exact)),
        };
        let mut store = qenlo_core::CoreStore::new(2).unwrap();
        store.add(2, 1, 0, [1.0, 0.0]).unwrap();
        assert!(matches!(
            block_on(backend.prepare(&store)),
            Err(GpuError::OverBudget { .. })
        ));
        assert!(backend.exact.is_none());
        backend.budget = DEFAULT_BUDGET;
        block_on(backend.prepare(&store)).unwrap();
        let output =
            block_on(backend.search(&store, &[1.0, 0.0], &qenlo_core::Predicate::ALL, 1)).unwrap();
        assert_eq!(output.hits[0].id, 2);
        assert!(backend.is_healthy());
        backend.destroy_device_for_test();
        assert!(!backend.is_healthy());
        assert!(matches!(
            block_on(backend.search(&store, &[1.0, 0.0], &qenlo_core::Predicate::ALL, 1)),
            Err(GpuError::Device(_))
        ));
    }

    #[test]
    fn empty_backend_selection_has_no_adapter() {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::empty();
        let instance = wgpu::Instance::new(descriptor);
        assert!(
            block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).is_err()
        );
    }
}
