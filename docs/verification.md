# verification record

## small-collection campaign (2026-09-05)

The Runpod campaign retained 182 rows: 131 completed, 42 unavailable, seven
failed, and two invalid-harness rows. The invalid rows remain visible and are
excluded from conclusions. Corrected 100K × 768 RTX 4090 cells reached oracle
recall 1.0. WGPU P95 was 0.897 ms for batch one, k=1, and 0.896 ms for batch
eight, k=64, with 10% eligibility. PyTorch CUDA reached 0.671 ms and 0.384 ms
respectively. See the [performance report](../research/artifacts/runpod-small-2026-09-05/report/performance-report.md) for timing scopes and all losses.

The selector candidate won five and lost seven of 12 qualified frozen-revision
pairs and was therefore reverted. Mutation cells reported zero resident rebuilds;
reopen rebuilt once. All campaign pods were deleted and the final known spend was
`$0.9400778694252952`. Cloud NVIDIA results are not mobile evidence.

The final bounded local release checks used one Cargo build job, one test thread,
and at most two numerical threads. Workspace no-default-feature testing passed
95 unit/integration tests plus one doctest. The required Vulkan GPU library run
passed 52 tests on the local RTX 4050; the FFI suite passed six. Focused core,
native CPU, and benchmark runs passed 18, 28, and 10 tests respectively. Strict
Clippy passed for the changed Rust packages. Python passed 23 tests, including
Torch CPU conformance, and built a tagged wheel plus source archive. TypeScript
passed type checking and five tests and produced an inspected package tarball.
Seven workflow YAML files and all PowerShell campaign scripts parsed. A local
POSIX shell was unavailable, so the shell scripts remain covered by workflow
execution rather than a new local `bash -n` result. The final eight-page PDF was
compiled without unresolved references or overfull boxes and every rendered page
was visually inspected.

Historical verification records follow unchanged.

2026-08-28, native Windows, `x86_64-pc-windows-msvc`, Intel UHD Graphics
(integrated) plus NVIDIA GeForce RTX 4050 Laptop GPU (discrete). Rust 1.98.0
(`88d9e12ae`, LLVM 22.1.8), Cargo 1.98.0.
this is a local correctness and benchmark record, not a production-readiness or
general scale claim.

## commands and results

the starting portable suite passed 16 tests before changes. the final native
commands used these process-local settings for C++ builds:

```powershell
$env:CC = 'clang-cl'
$env:CXX = 'clang-cl'
$env:CARGO_INCREMENTAL = '0'
```

| command | final result |
| --- | --- |
| `cargo fmt --all -- --check` | passed |
| `cargo test --workspace --no-default-features -- --test-threads=1` | 47 tests + 1 doctest passed |
| `cargo test --workspace --all-features --all-targets -- --test-threads=1` | 67 tests + 1 doctest passed |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | passed, no warnings |
| `cargo doc --workspace --all-features --no-deps` | passed |
| `cargo test -p qenlo --features usearch --test cpu_quality -- --nocapture` | 2 tests passed; CPU and USearch recall@10 = 1.0 in all 9 fixture filters |
| `cargo test -p qenlo-bench --all-features --all-targets` | 8 tests passed, including the real stalled-collector test |

the final all-features total consists of 40 collection/storage/GPU unit tests, 2
CPU quality tests, 2 process-exit tests, 7 benchmark-library tests, 3 runner
tests, 1 telemetry test, and 12 core tests. the doctest compiles the durable
quickstart.
both README Rust snippets also passed a separate `rustdoc --test` compilation.

hardware acceptance was run twice, with adapter absence made fatal:

```powershell
$env:QENLO_REQUIRE_GPU = '1'
$env:WGPU_BACKEND = 'dx12'
cargo test -p qenlo --features gpu-wgpu --lib -- --nocapture --test-threads=1
$env:WGPU_BACKEND = 'vulkan'
cargo test -p qenlo --features gpu-wgpu --lib -- --nocapture --test-threads=1
```

both runs passed 37 tests and printed the RTX 4050 with the requested backend.
The Intel adapter was present on the host but was not the reported device: the
high-performance request selected the discrete NVIDIA adapter. The retained
manifests record the actual adapter, device type, API, and negotiated limits.
coverage includes optional predicate combinations and signed extremes, tombstones,
bounded chunk/readback accounting, real device destruction, required errors,
automatic fallback, explicit recovery, unavailable adapters and allocation validation.

device destruction is `Device::destroy()`, not an injected driver reset. allocation
failure uses a real invalid allocation request and budget rejection, not deliberate
physical VRAM exhaustion. process-exit tests bypass destructors and leave partial
staging files; they do not simulate power removal. Unix, Metal, mobile, browser,
network filesystems and sudden-power-loss behavior were not exercised.

## benchmark smoke and lock observations

the prepared smoke has 256 corpus rows, dimension 16, 8 tuning queries and 32
held-out evaluation queries, seed 42. its prepared CRC32 is `c256dfdd`.

```powershell
cargo run -p qenlo-bench -- --help
cargo run -p qenlo-bench -- prepare --dataset target/bench-smoke-v1.qnb
cargo run -p qenlo-bench -- run --dataset target/bench-smoke-v1.qnb --output target/bench-smoke-cpu-v1 --fraction 0.1 --batch 8 --diagnostics detailed

foreach ($mode in @('disabled','basic','detailed')) {
    cargo run -p qenlo-bench -- run --dataset target/bench-smoke-v1.qnb --output "target/verification-lock-$mode" --fraction 0.1 --batch 1 --repetitions 5 --diagnostics $mode
}
```

all passed recall@10 = 1.0. the latter runs retained 160 samples each, 25 eligible
rows, AVX2, debug profile and no subscriber. measured observations:

| diagnostics | median of run P95 batch latency | mean reported lock wait per query |
| --- | --- | --- |
| disabled | 24,400 ns | 85.62 ns |
| basic | 23,000 ns | 53.12 ns |
| detailed | 30,100 ns | 70.62 ns |

these are small, sequential, uncontended smoke observations on a busy laptop.
lock wait includes clock/acquisition overhead, not an isolated estimate of all
locking costs. noise is visible: basic happened to be lower than disabled.
do not interpret this as a speed ranking or extrapolate it to scale.
raw samples and manifests remain in the named ignored `target/` directories;
`cargo clean` will remove them. repeat with fresh output paths.

additional corrected-runner smokes used CPU, USearch, GPU mask, GPU eligible rows,
GPU predicate and automatic mode, plus empty/fewer-than-k and batch-32 cases.
all passed recall 1.0 and strict score/order validation. their raw outputs are
`target/bench-smoke-*-v2/`. exact commands are reproducible from their manifests
and [the protocol](benchmark-protocol.md).

the larger independent correctness fixture has 2,048 rows, dimension 32 and 16
disjoint queries across 9 filters. both CPU and USearch measured recall 1.0.
The follow-up [real-data result record](results-2026-08-28.md) now retains
100k × 384 CPU, RTX 4050 GPU, USearch, and native Chroma cells with five runs,
independent truth, recall gates, and seeded whole-run intervals. The exact GPU
predicate path measured 5.14× lower P95 than Qenlo exact CPU when all rows were
eligible, while the 1% selective GPU predicate cell was slower than CPU. The
predeclared 1m × 768 investment gate remains untested because this host lacks
the required memory. host RSS/allocator totals and GPU kernel timestamps remain
explicitly unavailable for the library reports.

## device-lab verification (2026-08-30)

The release `quick` profile ran on the NVIDIA RTX 4050 through Vulkan with a
10,000 × 384 deterministic clustered corpus and 16 timed queries. Every cell
passed with Recall@10 1.0:

| cell | P95 per query | upload bytes |
| --- | ---: | ---: |
| CPU exact | 1,099 µs | 0 |
| GPU exact | 464 µs | 41,600 |
| native GPU batch-8 | 99 µs | 52,352 |
| IVF-Flat | 356 µs | 15,352 |
| IVF-SQ8 | 2,153 µs | 2,880 |
| automatic selective CPU | 28 µs | 0 |
| automatic selective batch-8 GPU | 414 µs | 52,352 |

These are compatibility/profile observations, not a production embedding claim.
SQ8 reduced transfer relative to IVF-Flat but did not reduce latency on this cell.
The same runner also completed on Intel UHD through Vulkan. AMD, Linux, Metal,
Android, and iOS still require physical tester devices.

`cargo test --workspace --features gpu-wgpu --no-fail-fast` passed 74 unit and
integration tests plus doctests. The all-features run with the documented
`clang-cl` workaround passed 78 unit/integration tests plus doctests, including
USearch and the bounded OTLP test. Strict workspace Clippy, format, diff, and
workflow YAML validation passed. The packaged telemetry executable accepted the
seven-cell report, listed it, and returned its detail with zero retained failures.
The dashboard was visually checked at 1280 × 720 and 390 × 844 with no document
overflow; empty, loaded, detail, pass, and no-failure states were exercised.

## Intel Arc device-lab evidence (2026-09-01)

Three manually supplied Windows/Vulkan reports are retained under
`benchmarks/2026-08-31/device-lab/intel-arc/`: two reports whose embedded suite
is `quick`, and one `soak` report with 100,000 × 384 rows and 512 samples per
cell. All 21 cells passed with Recall@10 = 1.0 and no reported fallback.

On the soak run, exact GPU P95 was 4,444 µs versus exact CPU P95 16,486 µs
(3.71× lower). IVF-Flat P95 was 2,704 µs (6.10× lower than exact CPU), while
IVF-SQ8 P95 was 22,797 µs and therefore slower than exact CPU on this cell.
The report supplied under a “full” label identifies itself as `quick`; the
retained record and public summary use the embedded value. These observations
are evidence for one Intel Arc adapter and do not close the 1M × 768 gate.

## toolchain and remaining release limits

- the existing MSVC 14.29 native-dependency crash was not repaired or re-tested;
  this work used the documented `clang-cl` workaround successfully.
- early builds printed Windows incremental-cache finalization `Access denied`.
  final verification disabled incremental compilation without changing user config.
- the OTLP test initially exposed a real runtime panic with an async HTTP client
  on synchronous exporter workers. switching the host to the blocking client
  fixed it. the final stalled-collector/privacy/overflow/shutdown check passed.
- full Windows power-loss durability is not guaranteed: files are synced, but
  directory publication is not synced with a Windows-specific primitive.
- commits append checksummed immutable WAL transactions after O(batch) atomic
  validation. reopen replays contiguous generations; flush/close synchronously
  compact to a full snapshot. background compaction is not claimed.
- only derived readiness metadata is persisted; ANN graphs and resident GPU
  buffers rebuild after restart. no persisted graph cache is claimed.
- interrupted initial creation may require manual handling of confirmed staging
  files. preserve evidence before cleanup; never guess at a committed outcome.
- the repository includes `LICENSE-MIT`; metadata also declares Apache-2.0 as an
  option, but a separate Apache license text is not included. no licensing terms
  were changed in this implementation.

## implementation commits

starting point: `4c40187`. no push or remote repository creation was performed.

```text
458d783 feat(cpu): add SIMD eligible-set baseline and streaming restore
a5d8fc5 feat(storage): lock writers and bound crash recovery loading
a51e285 feat(tx): add atomic durable mutation batches
08154d6 feat(cpu): validate filtered ANN recall and expose search parameters
a2c9e8a feat(concurrency): serialize writes and allow concurrent searches
09eb5fa fix(storage): detect missing commits and enforce reopen budgets
8891d19 feat(gpu): harden wgpu lifecycle and failure handling
cb82bd6 feat(index): integrate recovery policies and correlated diagnostics
28b8084 test(storage): exercise process exit and interrupted staging
ebd7d5f fix(obs): account for resident uploads and failed GPU preparation
36f2e11 feat(bench): add reproducible workload runner and bounded OTLP checks
f70b5d6 docs(api): add checked quickstart and clarify recovery diagnostics
```

the documentation-only commit containing this historical record follows those
commits. Later result, site, and adapter documentation is tracked in the current
repository history. The pre-existing `index.html` modification was preserved
until the site update requested after this record; the current site file now
includes the measured adapter and result status.
