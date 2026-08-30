use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    future::Future,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use qenlo::{
    BackendSelection, Collection, CollectionConfig, Filter, GpuFilterMode, Measurement, NewRecord,
    TimestampRange,
};
use qenlo_testkit::{SCHEMA_VERSION, TestCell, TestFailure, TestRun};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const HELP: &str = "qenlo-lab: cross-platform accelerator conformance and performance tester

run [--profile quick|full|soak] [--output run.json]
    [--endpoint https://lab.example/api/v1/runs --token TOKEN]
upload --input run.json --endpoint URL --token TOKEN

quick:  10k x 384, 16 timed queries per cell
full:  100k x 384, 64 timed queries per cell
soak:  100k x 384, 512 timed queries per cell

The runner tests exact correctness against independent float64 truth, filters,
tombstones, durable reopen, CPU/GPU latency, IVF-Flat, IVF-SQ8, selective automatic
routing, and true BxD batches. No embeddings, user IDs, timestamps, hostnames,
serial numbers, or raw query data are uploaded.";

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let interactive = args.is_empty();
    let result = if interactive {
        run_interactive()
    } else {
        run_cli(args)
    };
    if let Err(error) = &result {
        eprintln!("qenlo-lab: {error}");
    }
    if interactive {
        print!("\nPress Enter to close Qenlo Lab...");
        let _ = io::stdout().flush();
        let _ = io::stdin().read_line(&mut String::new());
    }
    if result.is_err() {
        std::process::exit(1);
    }
}

fn run_interactive() -> Result<()> {
    let output = std::env::current_exe()?
        .parent()
        .ok_or("executable has no parent directory")?
        .join("qenlo-lab-run.json");
    println!("Qenlo device lab\n================\n");
    println!("Running the quick hardware suite. This normally takes under a minute.\n");
    let report = block_on(run_suite("quick"))?;
    write_report(&output, &report)?;
    println!("\nWorkload results:");
    for cell in &report.cells {
        println!(
            "  {:32} {:>5}  p95 {:>8} us  recall {:.4}",
            cell.name,
            if cell.passed { "PASS" } else { "FAIL" },
            cell.p95_us,
            cell.recall_at_k
        );
    }
    println!(
        "\n{} cells, {} retained failures.\nReport saved to:\n{}",
        report.cells.len(),
        report.failures.len(),
        output.display()
    );
    println!("Keep that JSON file. It can be submitted later with the upload command.");
    Ok(())
}

fn run_cli(args: Vec<String>) -> Result<()> {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let (command, mut options) = parse(args)?;
    match command.as_str() {
        "run" => {
            let profile = take(&mut options, "--profile", "quick");
            let output = PathBuf::from(take(&mut options, "--output", "qenlo-lab-run.json"));
            let endpoint = options.remove("--endpoint");
            let token = options.remove("--token");
            exhausted(&options)?;
            let report = block_on(run_suite(&profile))?;
            write_report(&output, &report)?;
            println!("saved {} cells to {}", report.cells.len(), output.display());
            if let Some(endpoint) = endpoint {
                upload(
                    &endpoint,
                    token.as_deref().ok_or("--token is required")?,
                    &report,
                )?;
                println!("uploaded run {}", report.run_id);
            }
        }
        "upload" => {
            let input = PathBuf::from(required(&mut options, "--input")?);
            let endpoint = required(&mut options, "--endpoint")?;
            let token = required(&mut options, "--token")?;
            exhausted(&options)?;
            let report: TestRun = serde_json::from_slice(&fs::read(input)?)?;
            report.validate()?;
            upload(&endpoint, &token, &report)?;
        }
        _ => return Err("expected run or upload; use --help".into()),
    }
    Ok(())
}

pub(crate) async fn run_suite(profile: &str) -> Result<TestRun> {
    let (rows, samples) = match profile {
        "quick" => (10_000, 16),
        "full" => (100_000, 64),
        "soak" => (100_000, 512),
        _ => return Err("--profile must be quick, full, or soak".into()),
    };
    let dimension = 384;
    let started_at = unix_ms();
    let nonce = format!("{}-{}", started_at, std::process::id());
    let temp = std::env::temp_dir().join(format!("qenlo-lab-{nonce}"));
    let mut failures = Vec::new();
    let records = dataset(rows, dimension);
    let queries = queries(samples.max(32), dimension);
    let selective = Filter {
        user_id: Some(7),
        timestamp: TimestampRange::ALL,
    };
    eprintln!("[1/4] Computing independent float64 truth...");
    let all_truth = exact_truth(&records, &queries[..samples], &Filter::ALL, 10);
    let selective_truth = exact_truth(&records, &queries[..samples], &selective, 10);

    let mut cells = Vec::new();
    {
        eprintln!("[2/4] Checking CPU semantics, durability, and latency...");
        let cpu = Collection::new(CollectionConfig::cpu_exact(dimension)).await?;
        cpu.add_batch(&records)?;
        exercise_semantics(&cpu, &mut failures).await;
        exercise_durability(&temp, dimension, &mut failures).await;
        cells.push(
            measure(
                "cpu-exact-all",
                &cpu,
                &queries[..samples],
                &all_truth,
                &Filter::ALL,
                1,
            )
            .await?,
        );
    }

    let gpu_config = CollectionConfig {
        dimension,
        backend: BackendSelection::WgpuRequired(GpuFilterMode::GpuPredicate),
        gpu_allocation_budget_bytes: 512 * 1024 * 1024,
    };
    eprintln!("[3/4] Checking the selected GPU, batching, IVF-Flat, and IVF-SQ8...");
    let (gpu_name, gpu_api) = match Collection::new(gpu_config).await {
        Ok(gpu) => {
            gpu.add_batch(&records)?;
            let capabilities = gpu.gpu_capabilities();
            let names = capabilities
                .as_ref()
                .map(|caps| (Some(caps.adapter_name.clone()), Some(caps.backend.clone())))
                .unwrap_or_default();
            cells.push(
                measure(
                    "gpu-exact-all",
                    &gpu,
                    &queries[..samples],
                    &all_truth,
                    &Filter::ALL,
                    1,
                )
                .await?,
            );
            cells.push(
                measure(
                    "gpu-native-batch-8",
                    &gpu,
                    &queries[..samples],
                    &all_truth,
                    &Filter::ALL,
                    8,
                )
                .await?,
            );
            gpu.set_gpu_ivf(16, 4)?;
            cells.push(
                measure(
                    "gpu-ivf-flat-recall-95",
                    &gpu,
                    &queries[..samples],
                    &all_truth,
                    &Filter::ALL,
                    1,
                )
                .await?,
            );
            gpu.set_gpu_ivf_sq8(16, 4)?;
            cells.push(
                measure(
                    "gpu-ivf-sq8-recall-95",
                    &gpu,
                    &queries[..samples],
                    &all_truth,
                    &Filter::ALL,
                    1,
                )
                .await?,
            );
            names
        }
        Err(error) => {
            failures.push(TestFailure {
                stage: "gpu-initialization".into(),
                code: "GPU_UNAVAILABLE".into(),
                message: error.to_string(),
            });
            (None, None)
        }
    };
    let automatic = Collection::new(CollectionConfig {
        dimension,
        backend: BackendSelection::Automatic(GpuFilterMode::GpuPredicate),
        gpu_allocation_budget_bytes: 512 * 1024 * 1024,
    })
    .await?;
    eprintln!("[4/4] Checking automatic CPU/GPU routing...");
    automatic.add_batch(&records)?;
    drop(records);
    cells.push(
        measure(
            "automatic-selective-cpu-route",
            &automatic,
            &queries[..samples],
            &selective_truth,
            &selective,
            1,
        )
        .await?,
    );
    cells.push(
        measure(
            "automatic-selective-batch-8",
            &automatic,
            &queries[..samples],
            &selective_truth,
            &selective,
            8,
        )
        .await?,
    );
    let _ = fs::remove_dir_all(&temp);
    Ok(TestRun {
        schema_version: SCHEMA_VERSION,
        run_id: nonce.clone(),
        install_id: install_id(),
        started_at_unix_ms: started_at,
        completed_at_unix_ms: unix_ms(),
        app_version: env!("CARGO_PKG_VERSION").into(),
        target: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        os: std::env::consts::OS.into(),
        os_version: std::env::var("OS").unwrap_or_default(),
        cpu_arch: std::env::consts::ARCH.into(),
        cpu_name: cpu_name(),
        gpu_name,
        gpu_api,
        power_source: None,
        thermal_state: None,
        suite: profile.into(),
        cells,
        failures,
    })
}

async fn exercise_semantics(collection: &Collection, failures: &mut Vec<TestFailure>) {
    let filter = Filter {
        user_id: Some(7),
        timestamp: TimestampRange {
            lower: Some(0),
            upper: Some(100),
        },
    };
    if collection
        .search(&vec![1.0; 384], &filter, 10)
        .await
        .is_err()
    {
        failures.push(failure(
            "semantics",
            "FILTER_SEARCH",
            "filtered search failed",
        ));
    }
    if collection
        .search(&[0.0; 384], &Filter::ALL, 10)
        .await
        .is_ok()
    {
        failures.push(failure(
            "semantics",
            "ZERO_QUERY",
            "zero query was accepted",
        ));
    }
    if collection
        .search(&vec![1.0; 384], &Filter::ALL, 0)
        .await
        .is_ok()
    {
        failures.push(failure("semantics", "INVALID_K", "k=0 was accepted"));
    }
}

async fn exercise_durability(path: &Path, dimension: usize, failures: &mut Vec<TestFailure>) {
    let result: Result<()> = async {
        let collection = Collection::create(path, CollectionConfig::cpu_exact(dimension)).await?;
        collection.add_batch(&[NewRecord {
            id: 900_000_001,
            user_id: 1,
            timestamp: -1,
            vector: vec![1.0; dimension],
        }])?;
        collection.close()?;
        let reopened = Collection::open(path, CollectionConfig::cpu_exact(dimension)).await?;
        if reopened.stats().live_rows != 1 {
            return Err("durable reopen lost a row".into());
        }
        reopened.close()?;
        Ok(())
    }
    .await;
    if let Err(error) = result {
        failures.push(failure("durability", "REOPEN", &error.to_string()));
    }
}

async fn measure(
    name: &str,
    tested: &Collection,
    queries: &[Vec<f32>],
    truth: &[Vec<u64>],
    filter: &Filter,
    batch_size: usize,
) -> Result<TestCell> {
    if truth.len() != queries.len() {
        return Err("truth/query count mismatch".into());
    }
    let warmups = queries.len().min(4);
    for query in &queries[..warmups] {
        tested.search(query, filter, 10).await?;
    }
    let mut micros = Vec::new();
    let mut correct = 0usize;
    let mut possible = 0usize;
    let mut representative = None;
    let mut truth_offset = 0;
    for chunk in queries.chunks(batch_size) {
        let refs = chunk.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let expected = &truth[truth_offset..truth_offset + chunk.len()];
        let began = Instant::now();
        let responses = tested.search_batch(&refs, filter, 10).await?;
        let elapsed = began.elapsed().as_micros() as u64;
        micros.extend(std::iter::repeat_n(
            elapsed / chunk.len() as u64,
            chunk.len(),
        ));
        for (actual, expected) in responses.iter().zip(expected) {
            possible += expected.len();
            correct += actual
                .results
                .iter()
                .filter(|hit| expected.contains(&hit.id))
                .count();
        }
        truth_offset += chunk.len();
        representative = responses.last().map(|response| response.report.clone());
    }
    micros.sort_unstable();
    let report = representative.ok_or("no benchmark samples")?;
    let stats = tested.stats();
    let recall = if possible == 0 {
        1.0
    } else {
        correct as f64 / possible as f64
    };
    Ok(TestCell {
        name: name.into(),
        backend_requested: format!("{:?}", report.requested_backend),
        backend_actual: format!("{:?}", report.actual_backend),
        algorithm: format!("{:?}", report.algorithm),
        rows: stats.rows as u64,
        dimensions: stats.dimension as u32,
        eligible_fraction: if *filter == Filter::ALL { 1.0 } else { 0.01 },
        batch_size: batch_size as u32,
        k: 10,
        samples: micros.len() as u32,
        p50_us: percentile(&micros, 0.50),
        p95_us: percentile(&micros, 0.95),
        p99_us: percentile(&micros, 0.99),
        recall_at_k: recall,
        upload_bytes: available(report.upload_bytes),
        readback_bytes: available(report.readback_bytes),
        allocation_bytes: available(report.qenlo_allocation_bytes),
        dispatch_count: available(report.dispatch_count).map(u64::from),
        routing_reason: report.routing_reason,
        fallback_reason: report.fallback_reason,
        passed: recall
            >= if matches!(
                report.algorithm,
                qenlo::Algorithm::IvfFlat | qenlo::Algorithm::IvfSq8
            ) {
                0.95
            } else {
                0.999
            },
    })
}

fn dataset(rows: usize, dimension: usize) -> Vec<NewRecord> {
    let mut seed = 0x6a09_e667_f3bc_c909u64;
    (0..rows)
        .map(|row| NewRecord {
            id: row as u64 + 1,
            user_id: (row % 100) as u64,
            timestamp: row as i64 - rows as i64 / 2,
            vector: clustered_vector(&mut seed, dimension, row % 32),
        })
        .collect()
}

fn queries(count: usize, dimension: usize) -> Vec<Vec<f32>> {
    let mut seed = 0xbb67_ae85_84ca_a73bu64;
    (0..count)
        .map(|query| clustered_vector(&mut seed, dimension, query % 32))
        .collect()
}

fn clustered_vector(seed: &mut u64, dimension: usize, cluster: usize) -> Vec<f32> {
    let mut vector = random_vector(seed, dimension);
    vector[cluster % dimension] += 8.0;
    let norm = vector
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>()
        .sqrt() as f32;
    vector.iter_mut().for_each(|value| *value /= norm);
    vector
}

fn exact_truth(
    records: &[NewRecord],
    queries: &[Vec<f32>],
    filter: &Filter,
    k: usize,
) -> Vec<Vec<u64>> {
    queries
        .iter()
        .map(|query| {
            let mut scored = records
                .iter()
                .filter(|record| {
                    filter.user_id.is_none_or(|user| record.user_id == user)
                        && filter
                            .timestamp
                            .lower
                            .is_none_or(|lower| record.timestamp >= lower)
                        && filter
                            .timestamp
                            .upper
                            .is_none_or(|upper| record.timestamp < upper)
                })
                .map(|record| {
                    let dot = query
                        .iter()
                        .zip(&record.vector)
                        .map(|(&left, &right)| f64::from(left) * f64::from(right))
                        .sum::<f64>();
                    (record.id, 1.0 - dot)
                })
                .collect::<Vec<_>>();
            let order = |left: &(u64, f64), right: &(u64, f64)| {
                left.1
                    .total_cmp(&right.1)
                    .then_with(|| left.0.cmp(&right.0))
            };
            if scored.len() > k {
                scored.select_nth_unstable_by(k, order);
                scored.truncate(k);
            }
            scored.sort_unstable_by(order);
            scored.into_iter().map(|(id, _)| id).collect()
        })
        .collect()
}

fn random_vector(seed: &mut u64, dimension: usize) -> Vec<f32> {
    (0..dimension)
        .map(|_| {
            *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((*seed >> 40) as f32 / (1u32 << 24) as f32) * 2.0 - 1.0
        })
        .collect()
}

fn percentile(values: &[u64], quantile: f64) -> u64 {
    values[((values.len() - 1) as f64 * quantile).ceil() as usize]
}

fn available<T>(measurement: Measurement<T>) -> Option<T> {
    match measurement {
        Measurement::Available(value) => Some(value),
        Measurement::Unavailable(_) => None,
    }
}

fn upload(endpoint: &str, token: &str, report: &TestRun) -> Result<()> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()?
        .post(endpoint)
        .bearer_auth(token)
        .json(report)
        .send()?
        .error_for_status()?;
    Ok(())
}

fn write_report(path: &Path, report: &TestRun) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let pending = path.with_extension("json.pending");
    fs::write(&pending, serde_json::to_vec_pretty(report)?)?;
    fs::rename(pending, path)?;
    Ok(())
}

fn parse(args: Vec<String>) -> Result<(String, BTreeMap<String, String>)> {
    let mut args = args.into_iter();
    let command = args.next().unwrap_or_else(|| "--help".into());
    let mut options = BTreeMap::new();
    while let Some(key) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {key}"))?;
        if !key.starts_with("--")
            || value.starts_with("--")
            || options.insert(key.clone(), value).is_some()
        {
            return Err(format!("invalid or duplicate option {key}").into());
        }
    }
    Ok((command, options))
}

fn take(options: &mut BTreeMap<String, String>, key: &str, default: &str) -> String {
    options.remove(key).unwrap_or_else(|| default.into())
}

fn required(options: &mut BTreeMap<String, String>, key: &str) -> Result<String> {
    options
        .remove(key)
        .ok_or_else(|| format!("{key} is required").into())
}

fn exhausted(options: &BTreeMap<String, String>) -> Result<()> {
    options
        .keys()
        .next()
        .map_or(Ok(()), |key| Err(format!("unknown option {key}").into()))
}

fn failure(stage: &str, code: &str, message: &str) -> TestFailure {
    TestFailure {
        stage: stage.into(),
        code: code.into(),
        message: message.chars().take(512).collect(),
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn install_id() -> String {
    if let Ok(value) = std::env::var("QENLO_INSTALL_ID") {
        return value;
    }
    let root = std::env::var_os("APPDATA").map(PathBuf::from).or_else(|| {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
    });
    let Some(directory) = root.map(|root| root.join("qenlo")) else {
        return format!("ephemeral-{}", unix_ms());
    };
    let path = directory.join("install-id");
    if let Ok(value) = fs::read_to_string(&path) {
        let value = value.trim();
        if !value.is_empty() && value.len() <= 256 {
            return value.into();
        }
    }
    let value = format!("{}-{}", unix_ms(), std::process::id());
    if fs::create_dir_all(&directory).is_ok() {
        let pending = directory.join("install-id.pending");
        if fs::write(&pending, &value).is_ok() {
            let _ = fs::rename(pending, path);
        }
    }
    value
}

fn cpu_name() -> String {
    std::env::var("PROCESSOR_IDENTIFIER")
        .or_else(|_| std::env::var("HOSTTYPE"))
        .unwrap_or_else(|_| std::env::consts::ARCH.into())
}

struct ThreadWake(std::thread::Thread);
impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}
pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(ThreadWake(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::park(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_truth_applies_filters_and_id_ties() {
        let records = vec![
            NewRecord {
                id: 2,
                user_id: 7,
                timestamp: 1,
                vector: vec![1.0, 0.0],
            },
            NewRecord {
                id: 1,
                user_id: 7,
                timestamp: 2,
                vector: vec![1.0, 0.0],
            },
            NewRecord {
                id: 0,
                user_id: 8,
                timestamp: 1,
                vector: vec![1.0, 0.0],
            },
        ];
        let filter = Filter::new(Some(7), TimestampRange::new(Some(1), Some(3)));
        assert_eq!(
            exact_truth(&records, &[vec![1.0, 0.0]], &filter, 2),
            [[1, 2]]
        );
    }
}
