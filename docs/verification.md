# verification record

2026-08-28, native Windows, `x86_64-pc-windows-msvc`, NVIDIA GeForce RTX 4050
Laptop GPU. Rust 1.98.0 (`88d9e12ae`, LLVM 22.1.8), Cargo 1.98.0.
this is a local correctness record, not a production-readiness or scale claim.

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
| `cargo test --workspace --no-default-features` | 42 tests + 1 doctest passed |
| `cargo test --workspace --all-features` | 61 tests + 1 doctest passed |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | passed, no warnings |
| `cargo doc --workspace --all-features --no-deps` | passed |
| `cargo test -p qenlo --features usearch --test cpu_quality -- --nocapture` | 2 tests passed; CPU and USearch recall@10 = 1.0 in all 9 fixture filters |
| `cargo test -p qenlo-bench --all-features --all-targets` | 8 tests passed, including the real stalled-collector test |

the all-features total consists of 39 collection/storage/GPU unit tests, 2 CPU
quality tests, 2 process-exit tests, 5 benchmark-library tests, 2 runner tests,
1 telemetry test, and 10 core tests. the doctest compiles the durable quickstart.
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
neither fixture establishes recall on 100k/1m real embeddings. those scale runs,
competitor comparisons, confidence intervals and the 2x P95 investment gate
remain untested. host RSS/allocator totals and GPU kernel timestamps remain
explicitly unavailable. the runner currently uses shared timestamp filters;
distinct-filter and compound-filter performance cells remain to be implemented.

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
- commits clone the canonical store and write full snapshots. O(n) writes and
  peak transaction memory are deliberate limits, not a high-throughput claim.
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

the documentation-only commit containing this record follows those commits.
the pre-existing `index.html` modification was preserved and excluded from every
commit; the working tree is therefore intentionally not clean.
