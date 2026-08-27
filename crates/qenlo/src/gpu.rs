//! Experimental exact filtered search implemented with portable wgpu compute.
//!
//! The selector is intentionally a correctness-first `O(rows * k)` GPU kernel. Vector and
//! metadata buffers stay resident; query scratch is allocated and released per chunk so the
//! configured budget is a hard bound on Qenlo-owned allocations.

use std::{fmt, sync::mpsc};

const DEFAULT_BUDGET: u64 = 512 * 1024 * 1024;
const CANDIDATE_BYTES: u64 = 16;
const PARAM_BYTES: u64 = 48;
const WORKGROUP_SIZE: u64 = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FilterMode<'a> {
    /// One entry per collection row; `true` means eligible.
    Mask(&'a [bool]),
    /// Collection row numbers. Duplicates are harmless but waste work.
    EligibleRows(&'a [u32]),
    /// Fixed `(user == value) AND (lower <= timestamp < upper)` predicate.
    Predicate { user: u64, lower: i64, upper: i64 },
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
    init_pipeline: wgpu::ComputePipeline,
    score_pipeline: wgpu::ComputePipeline,
    select_pipeline: wgpu::ComputePipeline,
    group0_layout: wgpu::BindGroupLayout,
    group1_layout: wgpu::BindGroupLayout,
    chunks: Vec<Chunk>,
    dimension: u32,
    rows: u32,
    budget: u64,
    resident_bytes: u64,
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
            || vectors.len() != rows * dimension
        {
            return Err(GpuError::InvalidInput(
                "vector and metadata lengths disagree".into(),
            ));
        }

        let instance = wgpu::Instance::default();
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
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("qenlo exact-search device"),
                required_limits: adapter_limits.clone(),
                ..Default::default()
            })
            .await
            .map_err(|e| GpuError::Unavailable(e.to_string()))?;

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
        let init_pipeline = make_pipeline("init_scores");
        let score_pipeline = make_pipeline("score");
        let select_pipeline = make_pipeline("select");

        let budget = budget.unwrap_or(DEFAULT_BUDGET);
        let row_bytes = dimension as u64 * 4 + 8 + 8 + 8;
        let resident_bytes = row_bytes
            .checked_mul(rows as u64)
            .ok_or_else(|| GpuError::InvalidInput("allocation size overflow".into()))?;
        // Query, eligibility, scores, candidates, staging and uniforms are live together.
        let minimum_scratch =
            dimension as u64 * 4 + rows as u64 * 8 + 64 * CANDIDATE_BYTES * 2 + PARAM_BYTES;
        ensure_budget(resident_bytes + minimum_scratch, budget)?;

        let vector_row_bytes = dimension as u64 * 4;
        let max_binding = adapter_limits.max_storage_buffer_binding_size as u64;
        let max_dispatch_rows =
            adapter_limits.max_compute_workgroups_per_dimension as u64 * WORKGROUP_SIZE;
        let rows_per_chunk = (max_binding / vector_row_bytes)
            .min(max_binding / 8)
            .min(max_dispatch_rows)
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
                ),
                users: uploaded(&device, &queue, "qenlo users", &packed_users),
                timestamps: uploaded(&device, &queue, "qenlo timestamps", &packed_timestamps),
                ids: uploaded(&device, &queue, "qenlo ids", &packed_ids),
                live: live[start..end]
                    .iter()
                    .map(|value| u32::from(*value))
                    .collect(),
            });
        }

        Ok(Self {
            device,
            queue,
            init_pipeline,
            score_pipeline,
            select_pipeline,
            group0_layout,
            group1_layout,
            chunks,
            dimension: dimension as u32,
            rows: rows as u32,
            budget,
            resident_bytes,
        })
    }

    pub(crate) async fn search(
        &self,
        query: &[f32],
        k: usize,
        filter: FilterMode<'_>,
    ) -> Result<(Vec<GpuHit>, GpuExecution), GpuError> {
        if query.len() != self.dimension as usize {
            return Err(GpuError::InvalidInput("query dimension mismatch".into()));
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
        let mut merged = Vec::with_capacity(k * self.chunks.len());

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
            let scratch = query.len() as u64 * 4
                + eligibility.len() as u64 * 4
                + chunk.rows as u64 * 4
                + k as u64 * CANDIDATE_BYTES * 2
                + PARAM_BYTES;
            ensure_budget(self.resident_bytes + scratch, self.budget)?;
            execution.allocation_bytes = execution
                .allocation_bytes
                .max(self.resident_bytes + scratch);
            execution.upload_bytes +=
                query.len() as u64 * 4 + eligibility.len() as u64 * 4 + PARAM_BYTES;
            execution.readback_bytes += k as u64 * CANDIDATE_BYTES;

            let query_buffer = uploaded(&self.device, &self.queue, "qenlo query", query);
            let eligibility_buffer =
                uploaded(&self.device, &self.queue, "qenlo eligibility", &eligibility);
            let params_buffer =
                uploaded_uniform(&self.device, &self.queue, "qenlo params", &params);
            let scores = buffer(
                &self.device,
                "qenlo scores",
                chunk.rows as u64 * 4,
                wgpu::BufferUsages::STORAGE,
            );
            let selected = buffer(
                &self.device,
                "qenlo candidates",
                k as u64 * CANDIDATE_BYTES,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            );
            let staging = buffer(
                &self.device,
                "qenlo readback",
                k as u64 * CANDIDATE_BYTES,
                wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            );

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
                pass.set_pipeline(&self.init_pipeline);
                pass.dispatch_workgroups(chunk.rows.div_ceil(WORKGROUP_SIZE as u32), 1, 1);
                if dispatch_items != 0 {
                    pass.set_pipeline(&self.score_pipeline);
                    pass.dispatch_workgroups(dispatch_items.div_ceil(WORKGROUP_SIZE as u32), 1, 1);
                }
                pass.set_pipeline(&self.select_pipeline);
                pass.dispatch_workgroups(1, 1, 1);
            }
            encoder.copy_buffer_to_buffer(&selected, 0, &staging, 0, k as u64 * CANDIDATE_BYTES);
            self.queue.submit([encoder.finish()]);
            execution.dispatch_count += if dispatch_items == 0 { 2 } else { 3 };

            map_wait(&self.device, &staging)?;
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
    exact: Option<GpuExact>,
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
        wgpu::Instance::default()
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
                ..Default::default()
            })
            .await
            .map_err(|error| GpuError::Unavailable(error.to_string()))?;
        Ok(Self {
            dimension,
            mode,
            budget,
            exact: None,
        })
    }

    pub(crate) async fn prepare(&mut self, store: &qenlo_core::CoreStore) -> Result<(), GpuError> {
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
        self.exact = Some(
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
        );
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
        let eligible = store.filter(predicate);
        let filter = match self.mode {
            super::GpuFilterMode::CpuMask => {
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
                user: predicate.user_id.ok_or_else(|| {
                    GpuError::InvalidInput("GPU predicate mode requires user equality".into())
                })?,
                lower: predicate.timestamp.lower.ok_or_else(|| {
                    GpuError::InvalidInput("GPU predicate mode requires a lower timestamp".into())
                })?,
                upper: predicate.timestamp.upper.ok_or_else(|| {
                    GpuError::InvalidInput("GPU predicate mode requires an upper timestamp".into())
                })?,
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
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage,
        mapped_at_creation: false,
    })
}

fn uploaded<T: Copy>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    value: &[T],
) -> wgpu::Buffer {
    let bytes = bytes_of(value);
    let result = buffer(
        device,
        label,
        bytes.len() as u64,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    );
    queue.write_buffer(&result, 0, bytes);
    result
}

fn uploaded_uniform(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    value: &[u32; 12],
) -> wgpu::Buffer {
    let bytes = bytes_of(value);
    let result = buffer(
        device,
        label,
        PARAM_BYTES,
        wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    );
    queue.write_buffer(&result, 0, bytes);
    result
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
        FilterMode::Mask(mask) => (0, mask[start..end].iter().map(|v| u32::from(*v)).collect()),
        FilterMode::EligibleRows(rows) => (
            1,
            rows.iter()
                .copied()
                .filter(|row| (*row as usize) >= start && (*row as usize) < end)
                .map(|row| row - chunk.row_base)
                .collect(),
        ),
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
        FilterMode::Predicate { user, lower, upper } => (user, lower as u64, upper as u64, 7),
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

fn map_wait(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Result<(), GpuError> {
    let (sender, receiver) = mpsc::sync_channel(1);
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| GpuError::Device(e.to_string()))?;
    receiver
        .recv()
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
        let Ok(exact) = block_on(GpuExact::new(
            2,
            &[1.0, 0.0, 0.0, 1.0],
            &[20, 10],
            &[1, 1],
            &[0, 0],
            &[true, true],
            Some(16 * 1024 * 1024),
        )) else {
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
                user: 1,
                lower: 0,
                upper: 1,
            },
        ))
        .expect("GPU predicate search completes");
        assert_eq!(hits.iter().map(|hit| hit.id).collect::<Vec<_>>(), [20, 10]);
    }

    #[test]
    fn gpu_predicate_handles_signed_timestamps_and_tombstones_if_adapter_is_available() {
        let Ok(exact) = block_on(GpuExact::new(
            2,
            &[1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0],
            &[1, 2, 3, 4],
            &[7, 7, 7, 7],
            &[i64::MIN, -1, 1, i64::MAX],
            &[true, true, false, true],
            Some(16 * 1024 * 1024),
        )) else {
            return;
        };
        let (hits, _) = block_on(exact.search(
            &[1.0, 0.0],
            4,
            FilterMode::Predicate {
                user: 7,
                lower: -2,
                upper: 2,
            },
        ))
        .expect("GPU predicate search completes");
        assert_eq!(hits.iter().map(|hit| hit.id).collect::<Vec<_>>(), [2]);
    }
}
