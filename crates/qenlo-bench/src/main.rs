//! One explicit, reproducible workload cell per invocation; no benchmark framework.

use std::{
    collections::{BTreeMap, HashSet},
    error::Error,
    fs::File,
    future::Future,
    io::{BufWriter, Write},
    path::PathBuf,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    time::Instant,
};

use qenlo::{BackendSelection, Collection, CollectionConfig, Filter, Measurement, TimestampRange};
use qenlo_bench::{
    MetadataDistribution, OracleFilter, OracleRecord, PreparedOracle,
    dataset::{self, DatasetSpec},
    exact_cosine_search, nearest_rank_percentile, recall_at_k,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const HELP: &str = "qenlo-bench: deterministic preparation and one workload cell per run

prepare --dataset PATH [--rows 256|100k|1m] [--dimensions 16|384|768]
        [--tuning 8] [--evaluation 32] [--seed 42]
        [--input RAW_F32_LE --expect-crc32 HEX]
run     --dataset PATH --output NEW_DIRECTORY [--dimensions 16|384|768]
        [--backend cpu|usearch|gpu-mask|gpu-rows|gpu-predicate|automatic]
        [--distribution independent|positive|negative|skewed]
        [--fraction 1|0.1|0.01|0.001|0.0001|empty|fewer]
        [--user-id U64]
        [--batch 1|8|32] [--warmups 8] [--repetitions 3]
        [--recall-target 0.95|0.99] [--expansion-search 128]
        [--diagnostics disabled|basic|detailed]
        [--vector-budget-mib 512] [--gpu-budget-mib 512]

Defaults are small synthetic smoke workloads, NOT scale measurements.
Reference protocol: --rows 100k or 1m, --dimensions 384 or 768,
--tuning 1000 --evaluation 5000; run --warmups 200 --repetitions 5.
All vectors come from disjoint source-row intervals; imported content may repeat.
Each batch uses one shared filter. --user-id adds equality AND a bounded timestamp
range; --fraction then applies within that user's population. k=10.
Raw CSV samples, timing/recall summaries and configuration.txt are retained.
Vector budget is a payload estimate, not an RSS/allocator limit. External dataset
downloads, distinct-filter batches and automatic ANN tuning
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
        _ => Err("expected prepare or run; use --help".into()),
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
) -> Result<(OracleFilter, usize)> {
    let mut timestamps: Vec<_> = records
        .iter()
        .filter(|row| user_id.is_none_or(|user| row.user_id == user))
        .map(|row| row.timestamp_micros)
        .collect();
    timestamps.sort_unstable();
    let population = timestamps.len();
    let count = match fraction {
        "1" => population,
        "0.1" => population / 10,
        "0.01" => population / 100,
        "0.001" => population / 1_000,
        "0.0001" => population / 10_000,
        "empty" => 0,
        "fewer" => population.min(5),
        _ => return Err("unsupported --fraction".into()),
    };
    let upper = timestamps
        .get(count)
        .copied()
        .or_else(|| user_id.and_then(|_| timestamps.last()?.checked_add(1)));
    Ok((
        OracleFilter {
            user_id,
            timestamp_from: user_id.and_then(|_| timestamps.first().copied()),
            timestamp_to: upper,
            ..OracleFilter::default()
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

fn csv_value(value: Option<u64>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}

async fn run_cell(
    path: PathBuf,
    dimension: usize,
    mut options: BTreeMap<String, String>,
) -> Result<()> {
    let output = PathBuf::from(options.remove("--output").ok_or("--output is required")?);
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
    let user_id = options
        .remove("--user-id")
        .map(|value| value.parse::<u64>())
        .transpose()?;
    let batch = count(&take(&mut options, "--batch", "1"))?;
    if ![1, 8, 32].contains(&batch) {
        return Err("--batch must be 1, 8 or 32".into());
    }
    let warmups = count(&take(&mut options, "--warmups", "8"))?;
    let repetitions = count(&take(&mut options, "--repetitions", "3"))?;
    if repetitions == 0 {
        return Err("--repetitions must be nonzero".into());
    }
    let target: f64 = take(&mut options, "--recall-target", "0.95").parse()?;
    if target != 0.95 && target != 0.99 {
        return Err("--recall-target must be 0.95 or 0.99".into());
    }
    let expansion = count(&take(&mut options, "--expansion-search", "128"))?;
    if expansion == 0 {
        return Err("--expansion-search must be nonzero".into());
    }
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
    let (oracle_filter, eligible) = workload_filter(&data.corpus, &fraction, user_id)?;
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
    let git_revision = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .unwrap_or_else(|| "unavailable".into());
    writeln!(
        manifest,
        "format=qenlo-bench-run-v1\nstatus=incomplete-until-summary-exists\ndataset={}\ndataset_crc32={:08x}\nsource=prepared-f32-rows-see-preparation-record\nseed={}\nrows={}\ndimensions={}\ncorpus_range=0..{}\ntuning_range={}..{}\nevaluation_range={}..{}\nbackend={}\nmetadata={}\nfraction_requested={}\neligible_count={}\neligible_fraction_actual={}\nbatch={}\nfilter_mode=shared\nk=10\nwarmup_queries={}\nrepetitions={}\nrecall_target={}\nexpansion_search={}\nvector_budget_bytes={}\ngpu_budget_bytes={}\nplatform={}-{}\npackage_version={}\ngit_revision={}\npercentile=nearest-rank\nquery_latency=batch-call-completion\nqps_window=includes-driver-validation-and-csv\nhost_rss_bytes=unavailable:no-portable-process-measurement\nhost_allocator_bytes=unavailable:no-instrumented-allocator\nvector_budget_scope=source-plus-normalized-corpus-payload-only\ngpu_allocation_scope=qenlo-owned-not-physical-vram\nmissing_csv_measurement=empty:not-reported-by-backend\nscale_gate=untested-by-this-single-cell\nload_ns={}",
        path.display(),
        data.checksum,
        data.spec.seed,
        data.spec.corpus,
        dimension,
        data.spec.corpus,
        data.spec.corpus,
        data.spec.corpus + data.spec.tuning,
        data.spec.corpus + data.spec.tuning,
        data.spec.corpus + data.spec.tuning + data.spec.evaluation,
        backend_name,
        distribution.label(),
        fraction,
        eligible,
        eligible as f64 / data.spec.corpus as f64,
        batch,
        warmups,
        repetitions,
        target,
        expansion,
        vector_budget,
        gpu_budget,
        std::env::consts::OS,
        std::env::consts::ARCH,
        env!("CARGO_PKG_VERSION"),
        git_revision,
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
    let dirty = std::process::Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=no"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| (!out.stdout.is_empty()).to_string())
        .unwrap_or_else(|| "unavailable".into());
    writeln!(manifest, "git_worktree_dirty={dirty}")?;
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
    let collection = Collection::new(CollectionConfig {
        dimension,
        backend: requested,
        gpu_allocation_budget_bytes: gpu_budget,
    })
    .await?;
    collection.set_diagnostics(diagnostics);
    #[cfg(feature = "usearch")]
    if backend_name == "usearch" {
        collection.set_ann_search_expansion(expansion)?;
    }
    for row in &data.corpus {
        collection.add(row.id, row.user_id, row.timestamp_micros, &row.vector)?;
    }
    let build_time = build_started.elapsed();
    let readiness = Instant::now();
    collection.prepare().await?;
    let readiness_time = readiness.elapsed();
    // Ground truth exhausts the eligible subset. It is outside every measured query window.
    let oracle_started = Instant::now();
    let oracle = PreparedOracle::new(&data.corpus, dimension, oracle_filter)?;
    let truth: Vec<Vec<u64>> = data
        .evaluation
        .iter()
        .map(|query| {
            oracle
                .search(query, 10)
                .map(|hits| hits.into_iter().map(|hit| hit.id).collect())
        })
        .collect::<std::result::Result<_, _>>()?;
    let mut tuning_recall = 0.0;
    let mut replay_truth = BufWriter::new(File::create_new(output.join("truth.csv"))?);
    writeln!(replay_truth, "split,query_index,ids")?;
    for (index, ids) in truth.iter().enumerate() {
        writeln!(
            replay_truth,
            "evaluation,{index},{}",
            ids.iter().map(u64::to_string).collect::<Vec<_>>().join(";")
        )?;
    }
    for (index, query) in data.tuning.iter().enumerate() {
        let expected: Vec<_> = oracle
            .search(query, 10)?
            .into_iter()
            .map(|hit| hit.id)
            .collect();
        writeln!(
            replay_truth,
            "tuning,{index},{}",
            expected
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(";")
        )?;
        let response = collection.search(query, &filter, 10).await?;
        validate_scores(&data.corpus, query, &response.results)?;
        let actual: Vec<_> = response.results.iter().map(|hit| hit.id).collect();
        validate_results(&data.corpus, oracle_filter, &actual)?;
        tuning_recall += recall_at_k(&expected, &actual, 10)?;
    }
    tuning_recall /= data.tuning.len() as f64;
    replay_truth.flush()?;
    let oracle_time = oracle_started.elapsed();
    for i in 0..warmups {
        collection
            .search(&data.tuning[i % data.tuning.len()], &filter, 10)
            .await?;
    }
    let mut samples = BufWriter::new(File::create_new(output.join("samples.csv"))?);
    writeln!(
        samples,
        "run,batch_index,query_indices,query_count,batch_latency_ns,recall_at_10,result_count,eligible_count,upload_bytes,readback_bytes,max_qenlo_allocation_bytes,actual_backend,backend_counts,lock_wait_ns,cpu_distance_path,fallback"
    )?;
    let mut runs = BufWriter::new(File::create_new(output.join("runs.csv"))?);
    writeln!(
        runs,
        "run,batches,queries,p50_batch_ns,p95_batch_ns,p99_batch_ns,wall_ns,qps,recall_at_10,recall_target_passed"
    )?;
    let mut p95s = Vec::new();
    let mut all_recall = 0.0;
    let mut all_passed = true;
    for run in 0..repetitions {
        let mut order: Vec<_> = (0..data.evaluation.len()).collect();
        shuffle(&mut order, data.spec.seed.wrapping_add(run as u64));
        let started = Instant::now();
        let mut latencies = Vec::new();
        let mut run_recall = 0.0;
        for (batch_index, indices) in order.chunks(batch).enumerate() {
            let queries: Vec<_> = indices
                .iter()
                .map(|&i| data.evaluation[i].as_slice())
                .collect();
            let call_started = Instant::now();
            let responses = collection.search_batch(&queries, &filter, 10).await?;
            let latency = call_started.elapsed();
            if responses.len() != queries.len() {
                return Err("backend returned wrong response count for batch".into());
            }
            latencies.push(latency);
            let mut recall = 0.0;
            let mut results = 0;
            let mut upload = Some(0_u64);
            let mut readback = Some(0_u64);
            let mut allocated = Some(0_u64);
            let mut backend_counts = BTreeMap::<String, usize>::new();
            let mut lock_wait_ns = 0;
            let mut cpu_distance_path = String::from("not-applicable");
            let mut fallback = false;
            for (&index, response) in indices.iter().zip(responses) {
                let ids: Vec<_> = response.results.iter().map(|hit| hit.id).collect();
                validate_results(&data.corpus, oracle_filter, &ids)?;
                validate_scores(&data.corpus, &data.evaluation[index], &response.results)?;
                let query_recall = recall_at_k(&truth[index], &ids, 10)?;
                if response.report.actual_backend != qenlo::BackendKind::Usearch
                    && query_recall != 1.0
                {
                    return Err("exact backend differs from independent oracle IDs (including boundary ties)".into());
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
            let indices = indices
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(";");
            writeln!(
                samples,
                "{run},{batch_index},{indices},{},{},{},{results},{eligible},{},{},{},{actual_backend},{counts},{lock_wait_ns},{cpu_distance_path},{fallback}",
                queries.len(),
                latency.as_nanos(),
                recall / queries.len() as f64,
                csv_value(upload),
                csv_value(readback),
                csv_value(allocated)
            )?;
        }
        samples.flush()?;
        let wall = started.elapsed();
        let percentile = |p| nearest_rank_percentile(&latencies, p).unwrap();
        let p95 = percentile(0.95);
        p95s.push(p95);
        run_recall /= order.len() as f64;
        all_recall += run_recall;
        all_passed &= run_recall >= target;
        writeln!(
            runs,
            "{run},{},{},{},{},{},{},{},{},{}",
            latencies.len(),
            order.len(),
            percentile(0.50).as_nanos(),
            p95.as_nanos(),
            percentile(0.99).as_nanos(),
            wall.as_nanos(),
            order.len() as f64 / wall.as_secs_f64(),
            run_recall,
            run_recall >= target
        )?;
    }
    runs.flush()?;
    p95s.sort_unstable();
    let median = p95s[(p95s.len() - 1) / 2];
    let mut summary = File::create_new(output.join("summary.txt"))?;
    writeln!(
        summary,
        "status=completed\nbuild_ns={}\nreadiness_ns={}\noracle_and_tuning_ns={}\ntuning_recall_at_10={}\nevaluation_recall_at_10={}\nrecall_target_passed={}\nmedian_run_p95_batch_ns={}\nmedian_convention=lower-middle\nfilter_violations=0\nscale_performance_claim=none",
        build_time.as_nanos(),
        readiness_time.as_nanos(),
        oracle_time.as_nanos(),
        tuning_recall,
        all_recall / repetitions as f64,
        all_passed,
        median.as_nanos()
    )?;
    println!(
        "completed {}: recall@10={} target_passed={} median-run-P95-batch={}ns; no scale claim",
        output.display(),
        all_recall / repetitions as f64,
        all_passed,
        median.as_nanos()
    );
    if !all_passed {
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

fn validate_results(records: &[OracleRecord], filter: OracleFilter, ids: &[u64]) -> Result<()> {
    if ids.len() > 10 {
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
            let (filter, _) = workload_filter(records, "0.1", None).unwrap();
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
                let (filter, expected) = workload_filter(&records, fraction, None).unwrap();
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
                        workload_filter(&records, fraction, Some(user_id)).unwrap();
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
    }
}
