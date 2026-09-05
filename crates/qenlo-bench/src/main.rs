//! One explicit, reproducible workload cell per invocation; no benchmark framework.

mod replay;

use std::{
    alloc::{GlobalAlloc, Layout, System},
    collections::{BTreeMap, HashSet},
    error::Error,
    fs::File,
    future::Future,
    io::{BufWriter, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
    time::Instant,
};

use qenlo::{
    BackendSelection, Collection, CollectionConfig, Filter, Measurement, NewRecord, TimestampRange,
};
#[cfg(feature = "gpu-wgpu")]
use qenlo::{GpuFilterMode, GpuRowPreparation, RouterProfile};
use qenlo_bench::{
    MetadataDistribution, OracleFilter, OracleRecord, PreparedOracle,
    dataset::{self, DatasetSpec},
    exact_cosine_search, exact_cosine_tie_compatible, nearest_rank_percentile, recall_at_k,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct SourceState {
    git_revision: String,
    git_worktree_dirty: String,
    bundle_sha256: String,
}

fn source_state() -> SourceState {
    let git_revision = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_else(|| "unavailable".into());
    let git_worktree_dirty = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| (!output.stdout.is_empty()).to_string())
        .unwrap_or_else(|| "unavailable".into());
    let bundle_sha256 = std::env::var("QENLO_SOURCE_BUNDLE_SHA256")
        .ok()
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "unavailable".into());
    SourceState {
        git_revision,
        git_worktree_dirty,
        bundle_sha256,
    }
}

struct CountingAllocator;

static ALLOCATION_COUNT: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

// Counts gross process-wide allocator traffic during a completed call. This includes
// Rust dependencies and any background Rust work, not just allocations owned by Qenlo.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        // SAFETY: this allocator delegates the unchanged layout to the system allocator.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        // SAFETY: this allocator delegates the unchanged layout to the system allocator.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: the pointer and layout came from the delegated system allocator.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        // SAFETY: the pointer/layout pair came from System and new_size is forwarded unchanged.
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

#[derive(Clone, Copy)]
struct AllocationSnapshot {
    count: u64,
    bytes: u64,
}

impl AllocationSnapshot {
    fn now() -> Self {
        Self {
            count: ALLOCATION_COUNT.load(Ordering::Relaxed),
            bytes: ALLOCATED_BYTES.load(Ordering::Relaxed),
        }
    }

    fn since(self, before: Self) -> Self {
        Self {
            count: self.count.wrapping_sub(before.count),
            bytes: self.bytes.wrapping_sub(before.bytes),
        }
    }
}

const HELP: &str = "qenlo-bench: deterministic preparation and one workload cell per run

prepare --dataset PATH [--rows 256|100k|1m] [--dimensions 16|384|768]
        [--tuning 8] [--evaluation 32] [--seed 42]
        [--input RAW_F32_LE --expect-crc32 HEX]
run     --dataset PATH --output NEW_DIRECTORY [--dimensions 16|384|768]
        [--backend cpu|usearch|gpu-mask|gpu-rows|gpu-predicate|automatic]
        [--distribution independent|positive|negative|skewed]
        [--fraction 1|0.1|0.01|0.001|0.0001|empty|fewer]
        [--eligible-count ROWS]
        [--gpu-row-preparation legacy-two-pass|one-pass|cached]
        [--order-seed U64]
        [--router-profile KEY_VALUE_FILE]
        [--user-id U64]
        [--batch 1|8|16|32|64] [--k 1|10|64]
        [--warmups 8] [--repetitions 3]
        [--recall-target 0.95|0.99] [--expansion-search 128]
        [--tune-expansion-search 128,256,512,1024]
        [--allow-recall-miss true|false]
        [--oracle-reference COMPLETED_EXACT_CPU_DIRECTORY]
        [--diagnostics disabled|basic|detailed]
        [--vector-budget-mib 512] [--gpu-budget-mib 512]
lifecycle --dataset PATH --output NEW_DIRECTORY [--dimensions 16|384|768]
        [--backend cpu|gpu-mask|gpu-rows|gpu-predicate|automatic]
        [--repetitions 3] [--write-batch 8]
        [--vector-budget-mib 512] [--gpu-budget-mib 512]

Defaults are small synthetic smoke workloads, NOT scale measurements.
Reference protocol: --rows 100k or 1m, --dimensions 384 or 768,
--tuning 1000 --evaluation 5000; run --warmups 200 --repetitions 5.
All vectors come from disjoint source-row intervals; imported content may repeat.
Each batch uses one shared filter. --user-id adds equality AND a bounded timestamp
range; --fraction then applies within that user's population. k is bounded at 64.
Raw CSV samples, timing/recall summaries and configuration.txt are retained.
Vector budget is a payload estimate, not an RSS/allocator limit. External dataset
downloads and distinct-filter batches
are not implemented. Optional backends require the matching cargo feature.";

fn main() {
    if let Err(error) = run_cli(std::env::args().skip(1).collect()) {
        eprintln!("qenlo-bench: {error}");
        std::process::exit(1);
    }
}

fn parse(args: Vec<String>) -> Result<(String, BTreeMap<String, String>)> {
    let mut args = args.into_iter();
    let command = args.next().unwrap_or_else(|| "--help".into());
    let mut options = BTreeMap::new();
    while let Some(key) = args.next() {
        if !key.starts_with("--") {
            return Err(format!("expected option, got {key}").into());
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {key}"))?;
        if value.starts_with("--") {
            return Err(format!("missing value for {key}").into());
        }
        if options.insert(key.clone(), value).is_some() {
            return Err(format!("duplicate {key}").into());
        }
    }
    Ok((command, options))
}

fn take(options: &mut BTreeMap<String, String>, key: &str, default: &str) -> String {
    options.remove(key).unwrap_or_else(|| default.into())
}

fn count(value: &str) -> Result<usize> {
    Ok(match value {
        "100k" => 100_000,
        "1m" => 1_000_000,
        value => value.parse()?,
    })
}

fn exhausted(options: BTreeMap<String, String>) -> Result<()> {
    if let Some(key) = options.keys().next() {
        return Err(format!("unknown option {key}").into());
    }
    Ok(())
}

fn run_cli(args: Vec<String>) -> Result<()> {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let (command, mut options) = parse(args)?;
    let path = PathBuf::from(options.remove("--dataset").ok_or("--dataset is required")?);
    let dimension = count(&take(&mut options, "--dimensions", "16"))?;
    match command.as_str() {
        "prepare" => {
            let spec = DatasetSpec {
                dimension,
                corpus: count(&take(&mut options, "--rows", "256"))?,
                tuning: count(&take(&mut options, "--tuning", "8"))?,
                evaluation: count(&take(&mut options, "--evaluation", "32"))?,
                seed: take(&mut options, "--seed", "42").parse()?,
            };
            let input = options.remove("--input").map(PathBuf::from);
            let expected = options
                .remove("--expect-crc32")
                .map(|value| u32::from_str_radix(value.trim_start_matches("0x"), 16))
                .transpose()?;
            exhausted(options)?;
            let source = match (input.as_deref(), expected) {
                (Some(path), Some(crc)) => Some((path, crc)),
                (None, None) => None,
                _ => return Err("--input and --expect-crc32 must be supplied together".into()),
            };
            let checksum = dataset::prepare(&path, spec, source)?;
            println!(
                "prepared rows={} tuning={} evaluation={} dimension={} crc32={checksum:08x} source={}",
                spec.corpus,
                spec.tuning,
                spec.evaluation,
                spec.dimension,
                if input.is_some() {
                    "raw-f32-le"
                } else {
                    "synthetic-uniform-v1"
                }
            );
            Ok(())
        }
        "run" => block_on(run_cell(path, dimension, options)),
        "lifecycle" => block_on(run_lifecycle(path, dimension, options)),
        _ => Err("expected prepare, run or lifecycle; use --help".into()),
    }
}

struct ThreadWake(std::thread::Thread);
impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

// A blocking CLI executor, not a library runtime; wake-aware for GPU completion.
fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(ThreadWake(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::park(),
        }
    }
}

async fn run_lifecycle(
    path: PathBuf,
    dimension: usize,
    mut options: BTreeMap<String, String>,
) -> Result<()> {
    let output = PathBuf::from(options.remove("--output").ok_or("--output is required")?);
    let backend_name = take(&mut options, "--backend", "cpu");
    let repetitions = count(&take(&mut options, "--repetitions", "3"))?;
    let write_batch = count(&take(&mut options, "--write-batch", "8"))?;
    if repetitions == 0 || write_batch == 0 || write_batch > 64 {
        return Err("--repetitions must be nonzero and --write-batch must be in 1..=64".into());
    }
    let vector_budget = (count(&take(&mut options, "--vector-budget-mib", "512"))? as u64)
        .checked_mul(1 << 20)
        .ok_or("vector budget overflow")?;
    let gpu_budget = (count(&take(&mut options, "--gpu-budget-mib", "512"))? as u64)
        .checked_mul(1 << 20)
        .ok_or("GPU budget overflow")?;
    exhausted(options)?;
    let config = CollectionConfig {
        dimension,
        backend: backend(&backend_name)?,
        gpu_allocation_budget_bytes: gpu_budget,
    };
    let data = dataset::load(&path, dimension, vector_budget)?;
    std::fs::create_dir(&output)?;
    let durable = output.join("collection");
    let collection = Collection::create(&durable, config.clone()).await?;
    let build_started = Instant::now();
    for chunk in data.corpus.chunks(4_096) {
        let rows = chunk
            .iter()
            .map(|row| NewRecord {
                id: row.id,
                user_id: row.user_id,
                timestamp: row.timestamp_micros,
                vector: row.vector.clone(),
            })
            .collect::<Vec<_>>();
        collection.add_batch(&rows)?;
    }
    let build_ns = build_started.elapsed().as_nanos();
    let prepare_started = Instant::now();
    collection.prepare().await?;
    let initial_prepare_ns = prepare_started.elapsed().as_nanos();
    let query = data
        .evaluation
        .first()
        .ok_or("dataset has no evaluation query")?;
    let warm = collection.search(query, &Filter::ALL, 10).await?;
    if warm.results.is_empty() {
        return Err("lifecycle fixture unexpectedly returned no warm result".into());
    }

    let mut samples = BufWriter::new(File::create_new(output.join("lifecycle.csv"))?);
    writeln!(
        samples,
        "phase,repetition,mutation_ns,first_search_ns,actual_backend,rebuilt,generation,upload_bytes,allocation_bytes,result_count,deleted_id_absent"
    )?;
    let mut phase_search_ns = BTreeMap::<String, Vec<u128>>::new();
    let mut next_id = data.spec.corpus as u64;
    for repetition in 0..repetitions {
        let add_id = next_id;
        next_id += 1;
        let mutation_started = Instant::now();
        collection.add(add_id, u64::MAX - 1, repetition as i64, &data.tuning[0])?;
        let mutation_ns = mutation_started.elapsed().as_nanos();
        let search_started = Instant::now();
        let response = collection.search(query, &Filter::ALL, 10).await?;
        let search_ns = search_started.elapsed().as_nanos();
        phase_search_ns
            .entry("add-one".into())
            .or_default()
            .push(search_ns);
        write_lifecycle_row(
            &mut samples,
            "add-one",
            repetition,
            mutation_ns,
            search_ns,
            &response,
            true,
        )?;

        let mutation_started = Instant::now();
        collection.delete(add_id)?;
        let mutation_ns = mutation_started.elapsed().as_nanos();
        let search_started = Instant::now();
        let response = collection.search(query, &Filter::ALL, 10).await?;
        let search_ns = search_started.elapsed().as_nanos();
        let deleted_id_absent = response.results.iter().all(|hit| hit.id != add_id);
        if !deleted_id_absent {
            return Err("deleted ID was returned by the first post-delete search".into());
        }
        phase_search_ns
            .entry("delete-one".into())
            .or_default()
            .push(search_ns);
        write_lifecycle_row(
            &mut samples,
            "delete-one",
            repetition,
            mutation_ns,
            search_ns,
            &response,
            deleted_id_absent,
        )?;

        let batch_ids = (next_id..next_id + write_batch as u64).collect::<Vec<_>>();
        next_id += write_batch as u64;
        let batch = batch_ids
            .iter()
            .enumerate()
            .map(|(offset, &id)| NewRecord {
                id,
                user_id: u64::MAX - 2,
                timestamp: offset as i64,
                vector: data.tuning[offset % data.tuning.len()].clone(),
            })
            .collect::<Vec<_>>();
        let mutation_started = Instant::now();
        collection.add_batch(&batch)?;
        let mutation_ns = mutation_started.elapsed().as_nanos();
        let search_started = Instant::now();
        let response = collection.search(query, &Filter::ALL, 10).await?;
        let search_ns = search_started.elapsed().as_nanos();
        phase_search_ns
            .entry("add-batch".into())
            .or_default()
            .push(search_ns);
        write_lifecycle_row(
            &mut samples,
            "add-batch",
            repetition,
            mutation_ns,
            search_ns,
            &response,
            true,
        )?;

        let mutation_started = Instant::now();
        collection.delete_batch(&batch_ids)?;
        let mutation_ns = mutation_started.elapsed().as_nanos();
        let search_started = Instant::now();
        let response = collection.search(query, &Filter::ALL, 10).await?;
        let search_ns = search_started.elapsed().as_nanos();
        let deleted_id_absent = response
            .results
            .iter()
            .all(|hit| !batch_ids.contains(&hit.id));
        if !deleted_id_absent {
            return Err("deleted batch ID was returned by the first post-delete search".into());
        }
        phase_search_ns
            .entry("delete-batch".into())
            .or_default()
            .push(search_ns);
        write_lifecycle_row(
            &mut samples,
            "delete-batch",
            repetition,
            mutation_ns,
            search_ns,
            &response,
            deleted_id_absent,
        )?;
    }
    samples.flush()?;
    collection.flush()?;
    collection.close()?;
    let reopen_started = Instant::now();
    let reopened = Collection::open(&durable, config).await?;
    let reopen_ns = reopen_started.elapsed().as_nanos();
    let search_started = Instant::now();
    let response = reopened.search(query, &Filter::ALL, 10).await?;
    let reopen_first_search_ns = search_started.elapsed().as_nanos();
    write_lifecycle_row(
        &mut samples,
        "reopen",
        0,
        reopen_ns,
        reopen_first_search_ns,
        &response,
        true,
    )?;
    samples.flush()?;
    reopened.close()?;

    let source = source_state();
    let mut summary = BufWriter::new(File::create_new(output.join("summary.txt"))?);
    writeln!(summary, "status=completed")?;
    writeln!(summary, "format=qenlo-lifecycle-v1")?;
    writeln!(summary, "git_revision={}", source.git_revision)?;
    writeln!(summary, "git_worktree_dirty={}", source.git_worktree_dirty)?;
    writeln!(summary, "source_bundle_sha256={}", source.bundle_sha256)?;
    writeln!(summary, "backend={backend_name}")?;
    writeln!(summary, "rows={}", data.spec.corpus)?;
    writeln!(summary, "dimensions={dimension}")?;
    writeln!(summary, "repetitions={repetitions}")?;
    writeln!(summary, "write_batch={write_batch}")?;
    writeln!(summary, "build_ns={build_ns}")?;
    writeln!(summary, "initial_prepare_ns={initial_prepare_ns}")?;
    writeln!(summary, "reopen_ns={reopen_ns}")?;
    writeln!(summary, "reopen_first_search_ns={reopen_first_search_ns}")?;
    writeln!(
        summary,
        "timing_scope=completed synchronous mutation then completed first search"
    )?;
    writeln!(summary, "filter_scope=unfiltered")?;
    writeln!(
        summary,
        "host_rss_bytes=unavailable:measure process externally"
    )?;
    for (phase, values) in phase_search_ns {
        let durations = values
            .iter()
            .map(|&value| std::time::Duration::from_nanos(value.min(u64::MAX as u128) as u64))
            .collect::<Vec<_>>();
        writeln!(
            summary,
            "{phase}_p50_first_search_ns={}",
            nearest_rank_percentile(&durations, 0.5)
                .ok_or("lifecycle phase has no samples")?
                .as_nanos()
        )?;
        writeln!(
            summary,
            "{phase}_p95_first_search_ns={}",
            nearest_rank_percentile(&durations, 0.95)
                .ok_or("lifecycle phase has no samples")?
                .as_nanos()
        )?;
    }
    summary.flush()?;
    println!(
        "completed lifecycle {}: rows={} dimension={} backend={backend_name}",
        output.display(),
        data.spec.corpus,
        dimension
    );
    Ok(())
}

fn write_lifecycle_row(
    output: &mut impl Write,
    phase: &str,
    repetition: usize,
    mutation_ns: u128,
    search_ns: u128,
    response: &qenlo::SearchResponse,
    deleted_id_absent: bool,
) -> Result<()> {
    writeln!(
        output,
        "{phase},{repetition},{mutation_ns},{search_ns},{:?},{},{},{},{},{},{deleted_id_absent}",
        response.report.actual_backend,
        response.report.rebuilt,
        response.report.index_generation,
        csv_value(bytes(response.report.upload_bytes.clone())),
        csv_value(bytes(response.report.qenlo_allocation_bytes.clone())),
        response.results.len()
    )?;
    Ok(())
}

fn shuffle(order: &mut [usize], mut seed: u64) {
    for i in (1..order.len()).rev() {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        order.swap(i, (seed % (i as u64 + 1)) as usize);
    }
}

fn metadata(records: &mut [OracleRecord], distribution: MetadataDistribution, seed: u64) {
    let mut order: Vec<_> = (0..records.len()).collect();
    match distribution {
        MetadataDistribution::Independent => shuffle(&mut order, seed),
        MetadataDistribution::Skewed => {
            let score = |row: &OracleRecord| {
                let x = f64::from(row.vector[0]);
                let y = f64::from(*row.vector.get(1).unwrap_or(&row.vector[0]));
                0.2 * x + y.powi(3)
            };
            order.sort_unstable_by(|&a, &b| {
                score(&records[a])
                    .total_cmp(&score(&records[b]))
                    .then(a.cmp(&b))
            });
        }
        _ => order.sort_unstable_by(|&a, &b| {
            records[a].vector[0]
                .total_cmp(&records[b].vector[0])
                .then(a.cmp(&b))
        }),
    }
    if distribution == MetadataDistribution::NegativelyCorrelated {
        order.reverse();
    }
    let rows = records.len();
    for (rank, index) in order.into_iter().enumerate() {
        // Metadata is synthetic. Positive/negative relate to vector component 0.
        records[index].user_id =
            if distribution == MetadataDistribution::Skewed && rank < rows * 9 / 10 {
                0
            } else {
                (rank * 100 / rows) as u64
            };
        let position = if distribution == MetadataDistribution::Skewed {
            // Nonuniform spacing, with unique, ordered timestamps even for small sets.
            ((rank as f64 / rows as f64).powi(3) * 1e12) as i64 + rank as i64
        } else {
            rank as i64
        };
        records[index].timestamp_micros = position - rows as i64 / 2;
    }
}

fn workload_filter(
    records: &[OracleRecord],
    fraction: &str,
    user_id: Option<u64>,
    eligible_count: Option<usize>,
) -> Result<(OracleFilter, usize)> {
    let mut timestamps: Vec<_> = records
        .iter()
        .filter(|row| user_id.is_none_or(|user| row.user_id == user))
        .map(|row| row.timestamp_micros)
        .collect();
    timestamps.sort_unstable();
    let population = timestamps.len();
    let count = eligible_count.unwrap_or(match fraction {
        "1" => population,
        "0.1" => population / 10,
        "0.01" => population / 100,
        "0.001" => population / 1_000,
        "0.0001" => population / 10_000,
        "empty" => 0,
        "fewer" => population.min(5),
        _ => return Err("unsupported --fraction".into()),
    });
    if count > population {
        return Err("--eligible-count exceeds the filtered population".into());
    }
    let upper = timestamps
        .get(count)
        .copied()
        .or_else(|| user_id.and_then(|_| timestamps.last()?.checked_add(1)));
    Ok((
        OracleFilter {
            user_id,
            timestamp_from: user_id.and_then(|_| timestamps.first().copied()),
            timestamp_to: upper,
        },
        count,
    ))
}

fn backend(name: &str) -> Result<BackendSelection> {
    Ok(match name {
        "cpu" => BackendSelection::CpuExact,
        #[cfg(feature = "usearch")]
        "usearch" => BackendSelection::Usearch,
        #[cfg(feature = "gpu-wgpu")]
        "gpu-mask" => BackendSelection::WgpuRequired(qenlo::GpuFilterMode::CpuMask),
        #[cfg(feature = "gpu-wgpu")]
        "gpu-rows" => BackendSelection::WgpuRequired(qenlo::GpuFilterMode::CpuEligibleRows),
        #[cfg(feature = "gpu-wgpu")]
        "gpu-predicate" => BackendSelection::WgpuRequired(qenlo::GpuFilterMode::GpuPredicate),
        #[cfg(feature = "gpu-wgpu")]
        "automatic" => BackendSelection::Automatic(qenlo::GpuFilterMode::GpuPredicate),
        _ => return Err(format!("unknown or feature-disabled backend: {name}").into()),
    })
}

fn bytes(value: Measurement<u64>) -> Option<u64> {
    match value {
        Measurement::Available(value) => Some(value),
        Measurement::Unavailable(_) => None,
    }
}

fn duration_ns(value: &Measurement<std::time::Duration>) -> Option<u128> {
    match value {
        Measurement::Available(value) => Some(value.as_nanos()),
        Measurement::Unavailable(_) => None,
    }
}

#[cfg(feature = "gpu-wgpu")]
fn router_profile(path: &std::path::Path) -> Result<RouterProfile> {
    let fields = std::fs::read_to_string(path)?
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_owned(), value.trim().to_owned()))
        .collect::<BTreeMap<_, _>>();
    let required = |key: &str| -> Result<String> {
        Ok(fields
            .get(key)
            .cloned()
            .ok_or_else(|| format!("router profile missing {key}"))?)
    };
    let filter_mode = match required("filter_mode")?.as_str() {
        "gpu-mask" => GpuFilterMode::CpuMask,
        "gpu-rows" => GpuFilterMode::CpuEligibleRows,
        "gpu-predicate" => GpuFilterMode::GpuPredicate,
        _ => return Err("router profile filter_mode is invalid".into()),
    };
    Ok(RouterProfile {
        adapter_name: required("adapter_name")?,
        dimension: required("dimension")?.parse()?,
        batch_size: required("batch_size")?.parse()?,
        filter_mode,
        cached_rows: required("cached_rows")?.parse()?,
        gpu_min_eligible_rows: required("gpu_min_eligible_rows")?.parse()?,
    })
}

fn tuning_expansions(value: Option<String>, backend: &str, fixed: usize) -> Result<Vec<usize>> {
    let Some(value) = value else {
        return Ok(vec![fixed]);
    };
    if backend != "usearch" {
        return Err("--tune-expansion-search requires --backend usearch".into());
    }
    let mut values: Vec<usize> = value
        .split(',')
        .map(str::parse)
        .collect::<std::result::Result<_, _>>()?;
    if values.contains(&0) {
        return Err("tuning expansions must be nonzero".into());
    }
    values.sort_unstable();
    values.dedup();
    Ok(values)
}

fn csv_value(value: Option<u64>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}

fn recall_passes(recall: f64, target: f64) -> bool {
    recall + 1e-12 >= target
}

async fn run_cell(
    path: PathBuf,
    dimension: usize,
    mut options: BTreeMap<String, String>,
) -> Result<()> {
    let output = PathBuf::from(options.remove("--output").ok_or("--output is required")?);
    let oracle_reference = options.remove("--oracle-reference").map(PathBuf::from);
    let router_profile_path = options.remove("--router-profile").map(PathBuf::from);
    let backend_name = take(&mut options, "--backend", "cpu");
    let distribution_name = take(&mut options, "--distribution", "independent");
    let distribution = match distribution_name.as_str() {
        "independent" => MetadataDistribution::Independent,
        "positive" => MetadataDistribution::PositivelyCorrelated,
        "negative" => MetadataDistribution::NegativelyCorrelated,
        "skewed" => MetadataDistribution::Skewed,
        _ => return Err("unknown --distribution".into()),
    };
    let fraction = take(&mut options, "--fraction", "0.1");
    let eligible_count = options
        .remove("--eligible-count")
        .map(|value| count(&value))
        .transpose()?;
    let user_id = options
        .remove("--user-id")
        .map(|value| value.parse::<u64>())
        .transpose()?;
    let batch = count(&take(&mut options, "--batch", "1"))?;
    if ![1, 8, 16, 32, 64].contains(&batch) {
        return Err("--batch must be 1, 8, 16, 32 or 64".into());
    }
    let k = count(&take(&mut options, "--k", "10"))?;
    if !(1..=64).contains(&k) {
        return Err("--k must be between 1 and 64".into());
    }
    let warmups = count(&take(&mut options, "--warmups", "8"))?;
    let repetitions = count(&take(&mut options, "--repetitions", "3"))?;
    if repetitions == 0 {
        return Err("--repetitions must be nonzero".into());
    }
    let order_seed: u64 = take(&mut options, "--order-seed", "42").parse()?;
    let row_preparation_name = take(&mut options, "--gpu-row-preparation", "one-pass");
    #[cfg(feature = "gpu-wgpu")]
    let row_preparation = match row_preparation_name.as_str() {
        "legacy-two-pass" => GpuRowPreparation::LegacyTwoPass,
        "one-pass" => GpuRowPreparation::OnePass,
        "cached" => GpuRowPreparation::Cached,
        _ => {
            return Err("--gpu-row-preparation must be legacy-two-pass, one-pass or cached".into());
        }
    };
    #[cfg(not(feature = "gpu-wgpu"))]
    if row_preparation_name != "one-pass" {
        return Err("--gpu-row-preparation requires the gpu-wgpu feature".into());
    }
    #[cfg(not(feature = "gpu-wgpu"))]
    if router_profile_path.is_some() {
        return Err("--router-profile requires the gpu-wgpu feature".into());
    }
    let target: f64 = take(&mut options, "--recall-target", "0.95").parse()?;
    if target != 0.95 && target != 0.99 {
        return Err("--recall-target must be 0.95 or 0.99".into());
    }
    let allow_recall_miss: bool = take(&mut options, "--allow-recall-miss", "false").parse()?;
    let expansion = count(&take(&mut options, "--expansion-search", "128"))?;
    if expansion == 0 {
        return Err("--expansion-search must be nonzero".into());
    }
    let expansions = tuning_expansions(
        options.remove("--tune-expansion-search"),
        &backend_name,
        expansion,
    )?;
    let vector_budget = (count(&take(&mut options, "--vector-budget-mib", "512"))? as u64)
        .checked_mul(1 << 20)
        .ok_or("vector budget overflow")?;
    let gpu_budget = (count(&take(&mut options, "--gpu-budget-mib", "512"))? as u64)
        .checked_mul(1 << 20)
        .ok_or("GPU budget overflow")?;
    let diagnostics_name = take(&mut options, "--diagnostics", "basic");
    let diagnostics = match diagnostics_name.as_str() {
        "disabled" => qenlo::Diagnostics::Disabled,
        "basic" => qenlo::Diagnostics::Basic,
        "detailed" => qenlo::Diagnostics::Detailed,
        _ => return Err("--diagnostics must be disabled, basic or detailed".into()),
    };
    exhausted(options)?;
    let requested = backend(&backend_name)?;
    let load_started = Instant::now();
    let mut data = dataset::load(&path, dimension, vector_budget)?;
    metadata(&mut data.corpus, distribution, data.spec.seed);
    let (oracle_filter, eligible) =
        workload_filter(&data.corpus, &fraction, user_id, eligible_count)?;
    let filter = Filter {
        user_id: oracle_filter.user_id,
        timestamp: TimestampRange {
            lower: oracle_filter.timestamp_from,
            upper: oracle_filter.timestamp_to,
        },
    };
    let load_duration = load_started.elapsed();
    std::fs::create_dir(&output)?;
    let mut manifest = BufWriter::new(File::create_new(output.join("configuration.txt"))?);
    let source = source_state();
    writeln!(
        manifest,
        "format=qenlo-bench-run-v3\nstatus=incomplete-until-summary-exists\ndataset={}\ndataset_crc32={:08x}\nsource=prepared-f32-rows-see-preparation-record\nseed={}\norder_seed={}\nrows={}\ndimensions={}\ncorpus_range=0..{}\ntuning_range={}..{}\nevaluation_range={}..{}\nbackend={}\ngpu_row_preparation={}\nmetadata={}\nfraction_requested={}\neligible_count={}\neligible_fraction_actual={}\nbatch={}\nfilter_mode=shared\nk={}\nwarmup_queries={}\nrepetitions={}\nrecall_target={}\nexpansion_search={}\nvector_budget_bytes={}\ngpu_budget_bytes={}\nplatform={}-{}\npackage_version={}\ngit_revision={}\npercentile=nearest-rank\nquery_latency=batch-call-completion\nqps_window=includes-driver-validation-and-csv\nhost_rss_bytes=unavailable:no-portable-process-measurement\nprocess_allocator_scope=gross-process-wide-rust-allocator-traffic-during-completed-call\nprocess_allocated_bytes=gross-not-live-or-peak-bytes\nvector_budget_scope=source-plus-normalized-corpus-payload-only\ngpu_allocation_scope=qenlo-owned-not-physical-vram\nmissing_csv_measurement=empty:not-reported-by-backend\nscale_gate=untested-by-this-single-cell\nload_ns={}",
        path.display(),
        data.checksum,
        data.spec.seed,
        order_seed,
        data.spec.corpus,
        dimension,
        data.spec.corpus,
        data.spec.corpus,
        data.spec.corpus + data.spec.tuning,
        data.spec.corpus + data.spec.tuning,
        data.spec.corpus + data.spec.tuning + data.spec.evaluation,
        backend_name,
        row_preparation_name,
        distribution.label(),
        fraction,
        eligible,
        eligible as f64 / data.spec.corpus as f64,
        batch,
        k,
        warmups,
        repetitions,
        target,
        expansion,
        vector_budget,
        gpu_budget,
        std::env::consts::OS,
        std::env::consts::ARCH,
        env!("CARGO_PKG_VERSION"),
        source.git_revision,
        load_duration.as_nanos()
    )?;
    writeln!(
        manifest,
        "source_kind={}\nsource_crc32={}",
        if data.source_checksum.is_some() {
            "imported-raw-f32-le"
        } else {
            "synthetic-uniform-v1"
        },
        data.source_checksum
            .map(|value| format!("{value:08x}"))
            .unwrap_or_else(|| "not-applicable".into())
    )?;
    writeln!(manifest, "diagnostics={diagnostics_name}\nsubscriber=none")?;
    writeln!(manifest, "git_worktree_dirty={}", source.git_worktree_dirty)?;
    writeln!(manifest, "source_bundle_sha256={}", source.bundle_sha256)?;
    writeln!(
        manifest,
        "filter_user_id={}\nfilter_timestamp_from={}\nfilter_timestamp_to={}\nfraction_scope={}\nreplay_format=qenlo-csv-v1",
        oracle_filter
            .user_id
            .map(|v| v.to_string())
            .unwrap_or_default(),
        oracle_filter
            .timestamp_from
            .map(|v| v.to_string())
            .unwrap_or_default(),
        oracle_filter
            .timestamp_to
            .map(|v| v.to_string())
            .unwrap_or_default(),
        if user_id.is_some() {
            "selected-user-population"
        } else {
            "corpus"
        }
    )?;
    manifest.flush()?;
    let mut replay_metadata = BufWriter::new(File::create_new(output.join("metadata.csv"))?);
    writeln!(replay_metadata, "id,user_id,timestamp_micros")?;
    for row in &data.corpus {
        writeln!(
            replay_metadata,
            "{},{},{}",
            row.id, row.user_id, row.timestamp_micros
        )?;
    }
    replay_metadata.flush()?;
    let build_started = Instant::now();
    let collection = Collection::new_with_options(
        CollectionConfig {
            dimension,
            backend: requested,
            gpu_allocation_budget_bytes: gpu_budget,
        },
        qenlo::StorageOptions {
            max_load_bytes: vector_budget,
        },
    )
    .await?;
    collection.set_diagnostics(diagnostics);
    #[cfg(feature = "gpu-wgpu")]
    collection.set_gpu_row_preparation(row_preparation);
    #[cfg(feature = "gpu-wgpu")]
    if let Some(path) = &router_profile_path {
        collection.set_router_profile(Some(router_profile(path)?));
        writeln!(manifest, "router_profile={}", path.display())?;
    }
    #[cfg(feature = "usearch")]
    if backend_name == "usearch" {
        collection.set_ann_search_expansion(expansion)?;
    }
    // Bound the temporary normalized copy while avoiding one million lock acquisitions.
    for chunk in data.corpus.chunks(4_096) {
        let records = chunk
            .iter()
            .map(|row| NewRecord {
                id: row.id,
                user_id: row.user_id,
                timestamp: row.timestamp_micros,
                vector: row.vector.clone(),
            })
            .collect::<Vec<_>>();
        collection.add_batch(&records)?;
    }
    let build_time = build_started.elapsed();
    let readiness = Instant::now();
    collection.prepare().await?;
    let readiness_time = readiness.elapsed();
    #[cfg(feature = "gpu-wgpu")]
    if let Some(capabilities) = collection.gpu_capabilities() {
        writeln!(
            manifest,
            "gpu_adapter={:?}\ngpu_api={}\ngpu_device_type={}\ngpu_max_buffer_size={}\ngpu_max_storage_buffer_binding_size={}\ngpu_max_compute_workgroups_per_dimension={}\ngpu_timestamp_queries_supported={}\ngpu_capability_allocation_budget_bytes={}",
            capabilities.adapter_name,
            capabilities.backend,
            capabilities.device_type,
            capabilities.max_buffer_size,
            capabilities.max_storage_buffer_binding_size,
            capabilities.max_compute_workgroups_per_dimension,
            capabilities.timestamp_queries_supported,
            capabilities.allocation_budget_bytes
        )?;
    }
    // Ground truth exhausts the eligible subset. It is outside every measured query window.
    let oracle_started = Instant::now();
    let reference_truth = oracle_reference
        .as_ref()
        .map(|path| replay::load(path, &data, distribution, oracle_filter, eligible, k))
        .transpose()?;
    if let Some(path) = &oracle_reference {
        writeln!(
            manifest,
            "oracle_reference={}\noracle_reference_truth_crc32={:08x}",
            path.display(),
            dataset::checksum(&path.join("truth.csv"))?
        )?;
        manifest.flush()?;
    }
    let oracle = if reference_truth.is_none() {
        Some(PreparedOracle::new(&data.corpus, dimension, oracle_filter)?)
    } else {
        None
    };
    let tuning_truth: Vec<Vec<u64>> = if let Some(truth) = &reference_truth {
        truth.tuning.clone()
    } else {
        data.tuning
            .iter()
            .map(|query| {
                oracle
                    .as_ref()
                    .unwrap()
                    .search(query, k)
                    .map(|hits| hits.into_iter().map(|hit| hit.id).collect())
            })
            .collect::<std::result::Result<_, _>>()?
    };
    let mut last_tuning_recall = None;
    let mut replay_truth = BufWriter::new(File::create_new(output.join("truth.csv"))?);
    writeln!(replay_truth, "split,query_index,ids")?;
    for (index, ids) in tuning_truth.iter().enumerate() {
        writeln!(
            replay_truth,
            "tuning,{index},{}",
            ids.iter().map(u64::to_string).collect::<Vec<_>>().join(";")
        )?;
    }
    let mut tuning_samples = BufWriter::new(File::create_new(output.join("tuning.csv"))?);
    writeln!(
        tuning_samples,
        "expansion_search,query_count,k,recall_at_k,recall_at_10,wall_ns"
    )?;
    let mut selected_expansion = None;
    for candidate in expansions {
        #[cfg(feature = "usearch")]
        if backend_name == "usearch" {
            collection.set_ann_search_expansion(candidate)?;
        }
        let mut tuning_recall = 0.0;
        let tuning_started = Instant::now();
        for (query, expected) in data.tuning.iter().zip(&tuning_truth) {
            let response = collection.search(query, &filter, k).await?;
            validate_scores(&data.corpus, query, &response.results)?;
            let actual: Vec<_> = response.results.iter().map(|hit| hit.id).collect();
            if actual.len() != expected.len() {
                return Err("backend returned fewer results than min(k, eligible_count)".into());
            }
            validate_results(&data.corpus, oracle_filter, &actual, k)?;
            tuning_recall += recall_at_k(expected, &actual, k)?;
        }
        tuning_recall /= data.tuning.len() as f64;
        last_tuning_recall = Some(tuning_recall);
        writeln!(
            tuning_samples,
            "{candidate},{},{k},{tuning_recall},{},{}",
            data.tuning.len(),
            if k == 10 {
                tuning_recall.to_string()
            } else {
                String::new()
            },
            tuning_started.elapsed().as_nanos()
        )?;
        tuning_samples.flush()?;
        if recall_passes(tuning_recall, target) && selected_expansion.is_none() {
            selected_expansion = Some((candidate, tuning_recall));
        }
    }
    let (effective_expansion, selected_tuning_recall) = match selected_expansion {
        Some(selected) => selected,
        None if allow_recall_miss => (
            expansion,
            last_tuning_recall.ok_or("no ANN expansion was evaluated")?,
        ),
        None => {
            return Err("no supplied expansion met tuning recall target; held-out evaluation not run; tuning.csv retained".into());
        }
    };
    let tuning_recall = selected_tuning_recall;
    #[cfg(feature = "usearch")]
    if backend_name == "usearch" {
        collection.set_ann_search_expansion(effective_expansion)?;
    }
    writeln!(
        manifest,
        "expansion_search_effective={effective_expansion}\ntuning_selection=smallest-supplied-expansion-meeting-target-before-heldout"
    )?;
    manifest.flush()?;
    let truth: Vec<Vec<u64>> = if let Some(truth) = reference_truth {
        truth.evaluation
    } else {
        data.evaluation
            .iter()
            .map(|query| {
                oracle
                    .as_ref()
                    .unwrap()
                    .search(query, k)
                    .map(|hits| hits.into_iter().map(|hit| hit.id).collect())
            })
            .collect::<std::result::Result<_, _>>()?
    };
    for (index, ids) in truth.iter().enumerate() {
        writeln!(
            replay_truth,
            "evaluation,{index},{}",
            ids.iter().map(u64::to_string).collect::<Vec<_>>().join(";")
        )?;
    }
    replay_truth.flush()?;
    let oracle_time = oracle_started.elapsed();
    for i in 0..warmups {
        let query = data.tuning[i % data.tuning.len()].as_slice();
        collection.search_batch(&[query], &filter, k).await?;
    }
    let mut samples = BufWriter::new(File::create_new(output.join("samples.csv"))?);
    writeln!(
        samples,
        "run,batch_index,query_indices,query_count,batch_latency_ns,k,recall_at_k,recall_at_10,result_count,eligible_count,upload_bytes,readback_bytes,max_qenlo_allocation_bytes,process_allocation_count,process_allocated_bytes,upload_enqueue_ns,readback_completion_ns,backend_execution_ns,device_scoring_ns,device_selection_ns,actual_backend,backend_counts,lock_wait_ns,cpu_distance_path,routing_reasons,fallback,gpu_row_preparation,predicate_traversals,row_materialization_ns,materialized_rows,row_cache_hit,eligibility_predicate_kind,eligibility_representation,eligibility_generation,corpus_rows,eligibility_transfer_bytes,eligible_contiguous_runs"
    )?;
    let mut runs = BufWriter::new(File::create_new(output.join("runs.csv"))?);
    writeln!(
        runs,
        "run,batches,queries,p50_batch_ns,p95_batch_ns,p99_batch_ns,wall_ns,qps,k,recall_at_k,recall_at_10,recall_target_passed"
    )?;
    let mut p95s = Vec::new();
    let mut all_recall = 0.0;
    let mut all_passed = recall_passes(tuning_recall, target);
    for run in 0..repetitions {
        let mut order: Vec<_> = (0..data.evaluation.len()).collect();
        shuffle(&mut order, order_seed.wrapping_add(run as u64));
        let started = Instant::now();
        let mut latencies = Vec::new();
        let mut run_recall = 0.0;
        for (batch_index, indices) in order.chunks(batch).enumerate() {
            let queries: Vec<_> = indices
                .iter()
                .map(|&i| data.evaluation[i].as_slice())
                .collect();
            let allocations_before = AllocationSnapshot::now();
            let call_started = Instant::now();
            let responses = collection.search_batch(&queries, &filter, k).await?;
            let latency = call_started.elapsed();
            let call_allocations = AllocationSnapshot::now().since(allocations_before);
            if responses.len() != queries.len() {
                return Err("backend returned wrong response count for batch".into());
            }
            latencies.push(latency);
            let first_report = &responses[0].report;
            let gpu_row_preparation = first_report
                .gpu_row_preparation
                .map(|mode| format!("{mode:?}"))
                .unwrap_or_default();
            let predicate_traversals = first_report.predicate_traversals;
            let row_materialization_ns = duration_ns(&first_report.row_materialization)
                .map(|value| value.to_string())
                .unwrap_or_default();
            let materialized_rows = match &first_report.materialized_rows {
                Measurement::Available(value) => value.to_string(),
                Measurement::Unavailable(_) => String::new(),
            };
            let row_cache_hit = first_report
                .row_cache_hit
                .map(|value| value.to_string())
                .unwrap_or_default();
            let eligibility_predicate_kind = first_report
                .eligibility_predicate_kind
                .map(|value| format!("{value:?}"))
                .unwrap_or_default();
            let eligibility_representation = first_report
                .eligibility_representation
                .map(|value| format!("{value:?}"))
                .unwrap_or_default();
            let eligibility_generation = first_report
                .eligibility_generation
                .map(|value| value.to_string())
                .unwrap_or_default();
            let corpus_rows = bytes(first_report.corpus_rows.clone())
                .map(|value| value.to_string())
                .unwrap_or_default();
            let eligibility_transfer_bytes = bytes(first_report.eligibility_transfer_bytes.clone())
                .map(|value| value.to_string())
                .unwrap_or_default();
            let eligible_contiguous_runs = bytes(first_report.eligible_contiguous_runs.clone())
                .map(|value| value.to_string())
                .unwrap_or_default();
            let upload_enqueue_ns = duration_ns(&first_report.phases.upload)
                .map(|value| value.to_string())
                .unwrap_or_default();
            let readback_completion_ns = duration_ns(&first_report.phases.readback)
                .map(|value| value.to_string())
                .unwrap_or_default();
            let backend_execution_ns = duration_ns(&first_report.phases.execution)
                .map(|value| value.to_string())
                .unwrap_or_default();
            let device_scoring_ns = duration_ns(&first_report.phases.scoring)
                .map(|value| value.to_string())
                .unwrap_or_default();
            let device_selection_ns = duration_ns(&first_report.phases.selection)
                .map(|value| value.to_string())
                .unwrap_or_default();
            let mut recall = 0.0;
            let mut results = 0;
            let mut upload = Some(0_u64);
            let mut readback = Some(0_u64);
            let mut allocated = Some(0_u64);
            let mut backend_counts = BTreeMap::<String, usize>::new();
            let mut lock_wait_ns = 0;
            let mut cpu_distance_path = String::from("not-applicable");
            let mut routing_reasons = BTreeMap::<String, usize>::new();
            let mut fallback = false;
            for (&index, response) in indices.iter().zip(responses) {
                let ids: Vec<_> = response.results.iter().map(|hit| hit.id).collect();
                if ids.len() != truth[index].len() {
                    return Err("backend returned fewer results than min(k, eligible_count)".into());
                }
                validate_results(&data.corpus, oracle_filter, &ids, k)?;
                validate_scores(&data.corpus, &data.evaluation[index], &response.results)?;
                let query_recall = recall_at_k(&truth[index], &ids, k)?;
                if response.report.actual_backend != qenlo::BackendKind::Usearch
                    && query_recall != 1.0
                    && !exact_cosine_tie_compatible(
                        &data.corpus,
                        &data.evaluation[index],
                        oracle_filter,
                        &ids,
                        k,
                        1e-5,
                    )?
                {
                    return Err("exact backend differs from independent oracle IDs outside the 1e-5 boundary-tie tolerance".into());
                }
                recall += query_recall;
                results += ids.len();
                upload = upload
                    .zip(bytes(response.report.upload_bytes))
                    .map(|(a, b)| a + b);
                readback = readback
                    .zip(bytes(response.report.readback_bytes))
                    .map(|(a, b)| a + b);
                allocated = allocated
                    .zip(bytes(response.report.qenlo_allocation_bytes))
                    .map(|(a, b)| a.max(b));
                *backend_counts
                    .entry(format!("{:?}", response.report.actual_backend))
                    .or_default() += 1;
                lock_wait_ns += response.report.lock_wait.as_nanos();
                if let Some(path) = response.report.cpu_distance_path {
                    cpu_distance_path = format!("{path:?}");
                }
                if let Some(reason) = response.report.routing_reason {
                    *routing_reasons.entry(reason).or_default() += 1;
                }
                fallback |= response.report.fallback_reason.is_some();
            }
            run_recall += recall;
            let actual_backend = if backend_counts.len() == 1 {
                backend_counts.keys().next().unwrap().as_str()
            } else {
                "mixed"
            };
            let counts = backend_counts
                .iter()
                .map(|(name, count)| format!("{name}:{count}"))
                .collect::<Vec<_>>()
                .join(";");
            let routing_reasons = routing_reasons
                .iter()
                .map(|(reason, count)| format!("{reason} ({count})"))
                .collect::<Vec<_>>()
                .join(";");
            let indices = indices
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(";");
            writeln!(
                samples,
                "{run},{batch_index},{indices},{},{},{k},{},{},{results},{eligible},{},{},{},{},{},{upload_enqueue_ns},{readback_completion_ns},{backend_execution_ns},{device_scoring_ns},{device_selection_ns},{actual_backend},{counts},{lock_wait_ns},{cpu_distance_path},{routing_reasons},{fallback},{gpu_row_preparation},{predicate_traversals},{row_materialization_ns},{materialized_rows},{row_cache_hit},{eligibility_predicate_kind},{eligibility_representation},{eligibility_generation},{corpus_rows},{eligibility_transfer_bytes},{eligible_contiguous_runs}",
                queries.len(),
                latency.as_nanos(),
                recall / queries.len() as f64,
                if k == 10 {
                    (recall / queries.len() as f64).to_string()
                } else {
                    String::new()
                },
                csv_value(upload),
                csv_value(readback),
                csv_value(allocated),
                call_allocations.count,
                call_allocations.bytes
            )?;
        }
        samples.flush()?;
        let wall = started.elapsed();
        let percentile = |p| nearest_rank_percentile(&latencies, p).unwrap();
        let p95 = percentile(0.95);
        p95s.push(p95);
        run_recall /= order.len() as f64;
        all_recall += run_recall;
        all_passed &= recall_passes(run_recall, target);
        writeln!(
            runs,
            "{run},{},{},{},{},{},{},{},{k},{},{},{}",
            latencies.len(),
            order.len(),
            percentile(0.50).as_nanos(),
            p95.as_nanos(),
            percentile(0.99).as_nanos(),
            wall.as_nanos(),
            order.len() as f64 / wall.as_secs_f64(),
            run_recall,
            if k == 10 {
                run_recall.to_string()
            } else {
                String::new()
            },
            recall_passes(run_recall, target)
        )?;
    }
    runs.flush()?;
    p95s.sort_unstable();
    let median = p95s[(p95s.len() - 1) / 2];
    let mut summary = File::create_new(output.join("summary.txt"))?;
    writeln!(
        summary,
        "status=completed\nbuild_ns={}\nreadiness_ns={}\noracle_and_tuning_ns={}\nk={}\ntuning_recall_at_k={}\nevaluation_recall_at_k={}\ntuning_recall_at_10={}\nevaluation_recall_at_10={}\nrecall_target_passed={}\nmedian_run_p95_batch_ns={}\nmedian_convention=lower-middle\nfilter_violations=0\nscale_performance_claim=none",
        build_time.as_nanos(),
        readiness_time.as_nanos(),
        oracle_time.as_nanos(),
        k,
        tuning_recall,
        all_recall / repetitions as f64,
        if k == 10 {
            tuning_recall.to_string()
        } else {
            "not-applicable".into()
        },
        if k == 10 {
            (all_recall / repetitions as f64).to_string()
        } else {
            "not-applicable".into()
        },
        all_passed,
        median.as_nanos()
    )?;
    println!(
        "completed {}: recall@{}={} target_passed={} median-run-P95-batch={}ns; no scale claim",
        output.display(),
        k,
        all_recall / repetitions as f64,
        all_passed,
        median.as_nanos()
    );
    if !all_passed && !allow_recall_miss {
        return Err("held-out recall target not met; samples retained".into());
    }
    Ok(())
}

fn validate_scores(
    records: &[OracleRecord],
    query: &[f32],
    results: &[qenlo::SearchResult],
) -> Result<()> {
    for hit in results {
        let record = records
            .get(hit.id as usize)
            .ok_or("result ID outside corpus")?;
        let expected = exact_cosine_search(
            std::slice::from_ref(record),
            query,
            OracleFilter::default(),
            1,
        )?[0]
            .distance;
        if !hit.distance.is_finite() || (f64::from(hit.distance) - expected).abs() > 1e-5 {
            return Err("returned cosine distance differs from independent f64 score (absolute tolerance 1e-5)".into());
        }
    }
    if results.windows(2).any(|pair| {
        pair[0]
            .distance
            .total_cmp(&pair[1].distance)
            .then(pair[0].id.cmp(&pair[1].id))
            .is_gt()
    }) {
        return Err("results are not ordered by distance then ID".into());
    }
    Ok(())
}

fn validate_results(
    records: &[OracleRecord],
    filter: OracleFilter,
    ids: &[u64],
    k: usize,
) -> Result<()> {
    if ids.len() > k {
        return Err("backend returned more than k results".into());
    }
    let mut unique = HashSet::new();
    for &id in ids {
        let record = records.get(id as usize).ok_or("result ID outside corpus")?;
        if record.id != id
            || record.deleted
            || !unique.insert(id)
            || filter.user_id.is_some_and(|value| record.user_id != value)
            || filter
                .timestamp_from
                .is_some_and(|value| record.timestamp_micros < value)
            || filter
                .timestamp_to
                .is_some_and(|value| record.timestamp_micros >= value)
        {
            return Err("invalid, duplicate, deleted or ineligible result ID".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skewed_eligibility_differs_and_scores_and_order_are_validated() {
        let records: Vec<_> = (0..100)
            .map(|id| OracleRecord {
                id,
                user_id: 0,
                timestamp_micros: 0,
                vector: vec![id as f32 / 100.0, ((id * 37) % 100) as f32 / 100.0 + 0.01],
                deleted: false,
            })
            .collect();
        let mut positive = records.clone();
        let mut skewed = records.clone();
        metadata(
            &mut positive,
            MetadataDistribution::PositivelyCorrelated,
            42,
        );
        metadata(&mut skewed, MetadataDistribution::Skewed, 42);
        let selected = |records: &[OracleRecord]| {
            let (filter, _) = workload_filter(records, "0.1", None, None).unwrap();
            records
                .iter()
                .filter(|row| row.timestamp_micros < filter.timestamp_to.unwrap())
                .map(|row| row.id)
                .collect::<Vec<_>>()
        };
        assert_ne!(selected(&positive), selected(&skewed));
        assert_eq!(skewed.iter().filter(|row| row.user_id == 0).count(), 90);
        let query = &[1.0, 0.0];
        let truth = exact_cosine_search(&records, query, OracleFilter::default(), 10).unwrap();
        let hits: Vec<_> = truth
            .into_iter()
            .map(|hit| qenlo::SearchResult {
                id: hit.id,
                distance: hit.distance as f32,
            })
            .collect();
        validate_scores(&records, query, &hits).unwrap();
        let mut wrong_score = hits.clone();
        wrong_score[0].distance += 0.01;
        assert!(validate_scores(&records, query, &wrong_score).is_err());
        let mut wrong_order = hits;
        wrong_order.reverse();
        assert!(validate_scores(&records, query, &wrong_order).is_err());
    }

    #[test]
    fn parser_rejects_ambiguous_options_and_cells_have_exact_cardinality() {
        assert!(recall_passes(0.9899999999999999, 0.99));
        assert!(!recall_passes(0.989999, 0.99));
        assert_eq!(
            tuning_expansions(Some("512,128,128".into()), "usearch", 32).unwrap(),
            vec![128, 512]
        );
        assert_eq!(tuning_expansions(None, "cpu", 128).unwrap(), vec![128]);
        for bad in ["", "0", "128,", "oops"] {
            assert!(tuning_expansions(Some(bad.into()), "usearch", 128).is_err());
        }
        assert!(tuning_expansions(Some("128".into()), "cpu", 128).is_err());
        assert!(parse(vec!["run".into(), "--batch".into()]).is_err());
        assert!(
            parse(
                ["run", "--batch", "1", "--batch", "8"]
                    .map(String::from)
                    .into()
            )
            .is_err()
        );
        let mut records: Vec<_> = (0..100)
            .map(|id| OracleRecord {
                id,
                user_id: 0,
                timestamp_micros: 0,
                vector: vec![id as f32, 1.0],
                deleted: false,
            })
            .collect();
        for distribution in [
            MetadataDistribution::Independent,
            MetadataDistribution::PositivelyCorrelated,
            MetadataDistribution::NegativelyCorrelated,
            MetadataDistribution::Skewed,
        ] {
            metadata(&mut records, distribution, 42);
            for fraction in ["1", "0.1", "0.01", "0.001", "0.0001", "empty", "fewer"] {
                let (filter, expected) = workload_filter(&records, fraction, None, None).unwrap();
                let actual = records
                    .iter()
                    .filter(|row| {
                        filter
                            .timestamp_to
                            .is_none_or(|to| row.timestamp_micros < to)
                    })
                    .count();
                assert_eq!(actual, expected);
                for user_id in [0, 99, u64::MAX] {
                    let (compound, expected) =
                        workload_filter(&records, fraction, Some(user_id), None).unwrap();
                    let actual = records
                        .iter()
                        .filter(|row| {
                            row.user_id == user_id
                                && compound
                                    .timestamp_from
                                    .is_none_or(|from| row.timestamp_micros >= from)
                                && compound
                                    .timestamp_to
                                    .is_none_or(|to| row.timestamp_micros < to)
                        })
                        .count();
                    assert_eq!(actual, expected);
                    assert_eq!(compound.user_id, Some(user_id));
                    if expected > 0 {
                        assert!(compound.timestamp_from.is_some());
                        assert!(compound.timestamp_to.is_some());
                    }
                }
            }
        }
        assert_eq!(
            workload_filter(&records, "1", None, Some(37)).unwrap().1,
            37
        );
        assert!(workload_filter(&records, "1", None, Some(101)).is_err());
    }
}
