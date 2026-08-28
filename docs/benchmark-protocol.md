# reproducible benchmark protocol

`qenlo-bench` runs one explicitly selected workload cell at a time. the defaults
are a small synthetic correctness smoke, not a performance or scale result.
the independent oracle still uses exhaustive f64 cosine scoring and never calls
qenlo's scoring or eligibility implementation.

## smoke

run these from the repository root with fresh output paths:

```powershell
cargo run -p qenlo-bench -- --help
cargo run -p qenlo-bench -- prepare --dataset smoke.qnb
cargo run -p qenlo-bench -- run --dataset smoke.qnb --output smoke-cpu --fraction 0.1 --batch 8
```

preparation defaults to seed 42, 256 corpus vectors, 8 tuning vectors, 32 held-out
evaluation vectors, and dimension 16. runs default to 8 tuning-query warmups,
3 shuffled repetitions, k=10, and a 0.95 recall target. output creation is exclusive:
existing datasets and run directories are never overwritten. incomplete preparation
can leave a partial file, which loading rejects. an incomplete run has no completed
`summary.txt`; preserve it for diagnosis, and use a fresh directory to retry.

## data and provenance

the prepared format is little-endian, versioned `QNLOB001`: an 8-byte magic;
six u64 fields for dimensions, corpus rows, tuning rows, evaluation rows, seed,
and source provenance; all row-major f32 vector values; then CRC32 over the header
and vector payload. the provenance field is zero for generation, or bit 32 plus
the imported source CRC32. source CRC is checked both before and during import.
load rejects unsupported versions, inconsistent lengths, dimension mismatch,
non-finite values, zero norms, and checksum mismatch.

generated vectors use the fixed SplitMix64 sequence and uniform components in
[-1,1). row identity ranges are disjoint: corpus `[0,N)`, tuning `[N,N+T)`, and
evaluation `[N+T,N+T+E)`. imported data uses the same disjoint source-row ranges;
this is not a promise that an external source contains no duplicate content.
remove duplicates upstream if that property matters for your dataset.

an external source must be row-major raw little-endian f32 with exactly
`(N+T+E)*dimensions*4` bytes. prepare a deterministic source subset and retain its
selection recipe and publisher SHA-256 separately. CRC32 detects accidental
corruption; it does not authenticate a download. there is no built-in network
download or dataset-license acceptance.

```powershell
cargo run --release -p qenlo-bench -- prepare --dataset ag-news-100k-384.qnb --input selected.f32 --expect-crc32 YOUR_HEX_CRC32 --rows 100k --dimensions 384 --tuning 1000 --evaluation 5000 --seed 42
```

`--rows 100k` and `--rows 1m`, and `--dimensions 384` and `--dimensions 768`, are
supported sizes, not claims that those workloads were run. omitting `--input`
creates a labelled synthetic dataset, never a substitute for the real embedding
dataset in a published comparison. canonical source provenance and split sizes
are stored in the prepared header and copied into each run manifest.

## workload cells

run every backend and both recall targets against the exact same prepared data:

| parameter | values |
| --- | --- |
| corpus size | 100k, 1m |
| dimensions | 384, 768 |
| requested eligible fraction | 1, 0.1, 0.01, 0.001, 0.0001, empty, fewer |
| synthetic metadata | independent, positive, negative, skewed |
| batch size | 1, 8, 32 |
| recall@10 target | 0.95, 0.99 |
| backend | cpu, usearch, gpu-mask, gpu-rows, gpu-predicate, automatic |

each cell uses a shared upper-exclusive timestamp filter. exact cardinality is
floor(N*fraction); `empty` has zero rows and `fewer` has min(N,5). actual count and
fraction are always recorded, including small smoke fractions that round to zero.
metadata is independent of vectors after deterministic permutation in `independent`.
`positive` and `negative` correlate timestamp rank and synthetic user bucket with
the first vector component, in opposite directions. `skewed` ranks by
`0.2*x[0] + x[1]^3`, uses cubic timestamp spacing, and places 90% of users in one
synthetic bucket. this changes eligible IDs as well as metadata frequencies;
a regression test distinguishes it from positive rank. all metadata is synthetic, not the original corpus's
user or publication data. these are controlled projection correlations, not a
claim to model every real-world relationship to semantic distance.

By default the runner exercises timestamp-only, shared-filter batches. Adding
`--user-id 0` joins user equality with bounded timestamps; in that case fraction
is relative to the selected user's population and actual corpus eligibility is
recorded. Independent metadata with `--user-id 0 --fraction 1` selects 1% of a
100k corpus. Distinct-filter batches remain to be added. Batches are
sequential API batches, not an assertion of parallel device execution. the final
batch may be partial and its query count is recorded.

```powershell
cargo run --release -p qenlo-bench -- run --dataset ag-news-100k-384.qnb --output cpu-01 --dimensions 384 --backend cpu --fraction 0.01 --distribution positive --batch 1 --recall-target 0.99 --warmups 200 --repetitions 5

$env:CC = 'clang-cl'
$env:CXX = 'clang-cl'
cargo run --release -p qenlo-bench --features usearch -- run --dataset ag-news-100k-384.qnb --output ann-01 --dimensions 384 --backend usearch --expansion-search 128 --fraction 0.01 --recall-target 0.99 --warmups 200 --repetitions 5

cargo run --release -p qenlo-bench --features gpu-wgpu -- run --dataset ag-news-100k-384.qnb --output gpu-01 --dimensions 384 --backend gpu-predicate --fraction 0.01 --warmups 200 --repetitions 5
```

ANN expansion is explicit and fixed for measured queries. For USearch,
`--tune-expansion-search 128,256,512,1024,2048,4096,8192` tests the supplied values
on tuning queries first and records every attempt in `tuning.csv`. It selects the
smallest supplied passing value before accessing held-out queries; no passing
value means an error with no evaluation run. The effective value is recorded in
the manifest. Without the grid, only `--expansion-search` is tried. Do not choose
parameters by repeatedly chasing evaluation recall.

## timing and correctness

ground truth is `TopK({x in corpus where filter(x)}, query)`, never a filtered
unfiltered top-k. compute it before timing. `PreparedOracle` validates the corpus
and caches f64 norms and eligibility once; each query still scores every eligible
row independently of the library implementation. retain corpus originals for f64 scoring;
the collection normalizes its own copy. result IDs are checked for membership,
deletion, duplication, and filter eligibility after every measured call.
recall is unique-ID overlap divided by min(10,eligible count), with empty truth
defined as recall 1. every returned score is independently recomputed in f64 and
must be within absolute distance tolerance 1e-5. results must be sorted by
computed f32 distance, then ID. exact backends must achieve the recall target and
strict score, filter, count, and uniqueness checks; if the only ID difference is
an f64 boundary tie within 1e-5, the tie checker accepts it and records the raw
recall rather than hiding the difference.

`samples.csv` contains every completed batch's wall-clock latency, evaluation
indices, query count, recall, result/eligible counts, actual backend, and available
transfer/allocation counts, lock-wait sum, CPU distance path and fallback flag.
per-backend query counts are retained; automatic batches crossing a fallback are
marked `mixed`, never labelled only by the last query's backend.
latency begins immediately before `search_batch`
and ends only when results/readback complete; it includes API locking, filtering,
execution, transfers, synchronization and selection. backend initialization,
ingestion, explicit preparation, oracle work and warmups are outside this window.

`runs.csv` retains each repetition's P50/P95/P99 using nearest rank
`ceil(p*sample_count)`, run wall time, measured queries/second and mean recall.
QPS includes driver validation and buffered CSV writing, unlike batch-call latency.
`summary.txt` reports build, readiness, oracle/tuning times, tuning/evaluation
recall, pass/fail and the lower-middle median across run P95 values. reference
runs use five repetitions, so the median is unambiguous. a failed recall target
returns a nonzero exit code but retains samples. filter violations fail immediately.
publish every cell, including failures, rather than only a favorable aggregate.
The completed 2026-08-28 100k × 384 real-data cells are retained in the
[result record](results-2026-08-28.md); the preregistered 1m × 768 gate remains
untested.

`configuration.txt` retains split ranges, seed, source and prepared CRC32, requested
backend, metadata distribution, fraction/count, batch, target, ANN expansion,
budgets, platform/architecture, package version, git revision and metric conventions.
also archive your compiler version, feature flags, build profile, dirty diff,
GPU driver/adapter, power settings and machine specifications with a published run.
On the measured hybrid host, wgpu enumerated Intel UHD Graphics (integrated) and
NVIDIA GeForce RTX 4050 Laptop GPU (discrete). The high-performance request
selected the NVIDIA adapter; `gpu_adapter`, `gpu_device_type`, and `gpu_api` in
the run manifest are the source of truth for reproductions on multi-GPU hosts.
the runner does not infer those hardware details or claim a clean checkout.

## memory and reporting limits

preparation streams one vector at a time. loading estimates source vector payload
plus the normalized collection copy and rejects it above `--vector-budget-mib`
(default 512). that is not an RSS limit: metadata, allocator overhead, indexes,
oracle scratch and runtime allocations are additional. source and collection
vectors are RAM-resident. a 1m x 768 run needs an explicit larger vector budget
(for example `--vector-budget-mib 8192`) and enough real host memory. do not run
that command merely to force an out-of-memory result on a constrained laptop.

The runner itself has no portable process sampler or instrumented allocator.
For native Windows, `scripts/measure_command.py` wraps the benchmark executable,
recording sampled host working set and the OS peak working set separately from
latency reports. It measures that process only, not descendants, allocator totals
or GPU VRAM. Do not wrap a virtualenv Python launcher and mistake its memory for
the child workload. Empty transfer/allocation
CSV cells mean the backend did not report them. Qenlo-owned GPU allocations are
not physical VRAM residency. `--gpu-budget-mib` separately controls the backend
budget, including its supported scratch accounting. required GPU errors remain
errors; only explicit `automatic` may fall back and actual backend is retained.

## instrumentation and OTLP checks

```powershell
cargo test -p qenlo-bench --features otlp --all-targets
cargo run -p qenlo-bench --features otlp --example otlp
```

the example performs a real search under a scoped subscriber and exports traces
and bounded-category metrics. query spans finish before shutdown. the helper caps
the trace queue at 32 spans and drops overflow; requests have explicit timeouts.
the test compares instrumented search results to uninstrumented results, floods
the queue, checks captured telemetry for sensitive input, connects to a collector
that accepts but never responds, and bounds shutdown. exporter failure is never
converted into a search error.

`--diagnostics disabled|basic|detailed` selects the collection's diagnostics mode
for workload runs (default basic). the runner installs no subscriber; these runs
measure report/diagnostic work without OTLP export. compare that overhead with
identical cells and retain all samples. the failure test separately covers actual
export, and checks that disabled diagnostics emit no operation spans. the host
uses the blocking HTTP client because the synchronous SDK processors export on
dedicated threads without a Tokio reactor; provider construction/shutdown is
isolated with `spawn_blocking` when called from an async host.

SDK 0.32.1 ignores the meter provider's `shutdown_with_timeout` parameter; the
periodic reader has a fixed five-second wait. trace shutdown uses two seconds;
HTTP requests use 100ms in the failure test and 500ms in the example. the helper
registers no observable callbacks, which could independently block collection.
this limitation was verified against the pinned SDK source, not inferred from
the method name. the test allows an eight-second combined shutdown ceiling.
see [OpenTelemetry Rust batch processing](https://github.com/open-telemetry/opentelemetry-rust/blob/main/opentelemetry-sdk/src/trace/span_processor.rs)
and the checked-in lockfile for the exact dependency version.

## the investment gate remains untested

the preregistered gate is 1m x 768, 1% eligible, batch 1, k=10: at least 2x lower
P95 than the best validated CPU path at the recall target, no invalid results,
five shuffled runs with uncertainty reported, and complete transfer/selection
costs with basic diagnostics. this runner enables measurements; it does not
establish that gate. [Native Chroma replay](../scripts/chroma-replay.md) now
validates the same vectors, canonical synthetic metadata, compound filter,
independent truth and shuffled query order. `scripts/compare_runs.py` checks
workload compatibility and recall gates before reporting latency ratios and
seeded whole-run bootstrap intervals. Five repetitions give coarse uncertainty;
neither bootstrap nor one laptop establishes general performance. Other competitor
adapters and pinned service environments remain absent. Native Windows library
measurements must not be presented as directly comparable to Linux/WSL HTTP
service latency. if memory or toolchains prevent the run, report it as untested.
