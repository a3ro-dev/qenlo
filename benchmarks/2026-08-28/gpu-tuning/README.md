# GPU kernel tuning: native Windows RTX 4050

These are **small synthetic tuning runs**, not the full research protocol, a
competitor comparison, or the 1m × 768 investment gate. All timed searches are
completed, batch-one, exact top-10 queries with basic diagnostics, including
filtering, transfers, selection and readback. No CPU fallback occurred.

## Measured results

100,000 × 384 float32 vectors, synthetic independent metadata, seed 42. There are
only **20 distinct evaluation queries** and four tuning queries. Full-scan runs
use four warmups and two shuffled repetitions; 1% runs use eight warmups and
three repetitions. Each P95 is the nearest-rank 19th of 20 observations. The
reported aggregate is the lower-middle median of run P95s, matching the harness.
This is a noisy tuning sample, not a reliable production tail estimate or a
confidence interval. See each `runs.csv` for variation between repetitions.

| Eligible | Backend / kernel | Median run P95 (ms) | Run P95s (ms) |
|---|---|---:|---|
| 100% | CPU exact, runtime SIMD | 34.1576 | 37.8663, 34.1576 |
| 100% | GPU predicate, original serial selector | 131.6643 | 146.7037, 131.6643 |
| 100% | GPU predicate, parallel selector, 16,384-row chunks | 7.0770 | 11.5687, 7.0770 |
| 100% | GPU predicate, parallel selector, 131,072-row cap | 3.0908 | 3.1873, 3.0908 |
| 1% | CPU exact, runtime SIMD | 0.2369 | 0.2379, 0.1361, 0.2369 |
| 1% | GPU CPU-generated row list | 0.8070 | 0.8284, 0.8040, 0.8070 |
| 1% | GPU CPU-generated mask | 2.2499 | 2.2968, 2.2349, 2.2499 |
| 1% | GPU predicate | 2.3796 | 2.6843, 2.3796, 2.3520 |

All eight cells returned recall@10 = 1 against the independent float64 oracle,
with zero reported filter violations. The optimized broad scan's P95 was 11.05×
lower than the measured SIMD CPU path and 42.60× lower than the original GPU
path. **At 1% eligibility the CPU won against every GPU mode.** These results
support a broad exact-scan use case; they do not establish universal GPU gains,
GPU ANN, or the scale gate. The configurations explicitly select a GPU backend;
there is no latency-based backend planner.

## What changed

`7d160fd` changes only the GPU Rust adapter and its WGSL shader:

- Eight vectors per 256-invocation workgroup; 32 adjacent lanes load adjacent
  dimensions, then reduce their dot products in workgroup memory.
- Exact selection uses 256 lanes to scan disjoint row subsets and a tree
  reduction to elect each winner. Work remains O(rows × k), but is parallel.
- Eligible-row mode scores and selects the compact row list, without clearing
  or scanning the full score array. Other modes write excluded scores directly.
- A larger chunk cap removes repeated host synchronization for this workload.
  Adapter binding/dispatch limits and the configured memory budget can still
  force smaller chunks.

No vendor ANN kernel or subgroup extension is used. Selection preserves full
64-bit ID tie ordering. Every chunk returns only k candidates; the final full
scan read back 160 bytes/query instead of a complete distance array. Its report
counted 401,584 upload bytes and 157,203,488 peak Qenlo-owned allocation bytes.
These allocation counters are **not physical VRAM residency measurements**.

## Runtime and correctness

- Windows x86_64, Intel i5-13420H, NVIDIA GeForce RTX 4050 Laptop GPU, driver
  616.56, driver-reported 6,141 MiB total GPU memory.
- All performance cells explicitly used `WGPU_BACKEND=dx12` (irrelevant to CPU).
- Required-hardware GPU tests passed **14/14 on DX12 and 14/14 on Vulkan**.
  The retained transcripts name the actual RTX 4050 adapter. They were run with
  `QENLO_REQUIRE_GPU=1`, so unavailable hardware could not silently skip tests.
- Tests cover signed timestamp extremes, optional predicates, deletion,
  duplicate eligible rows, cross-chunk ties, allocation failure, device loss,
  and an independent float64 dot-product check over dimensions
  1/7/31/32/33/128/384/768, k 1/10/64, partial workgroups and sparse/empty filters.

No hardware other than this Windows RTX 4050 was exercised. GPU dot products use
a different float32 accumulation order from CPU; the cross-check permits
absolute distance error below 2e-6, and exact IDs agreed on these samples.
Near-boundary numerical ties in arbitrary datasets remain subject to
floating-point tolerance.

## Source provenance and limitations

Original GPU source: `55d089615a54285308c5488587ad66128a25a346`.
Final GPU source: `7d160fd` (full revision in `source-provenance.txt`). The
intermediate parallel run used the final production shader with a 16,384-row
cap; the final version uses 131,072. Tests were added after the first timings.

The repository had concurrent benchmark development during these tuning runs.
Each untouched `configuration.txt` retains the HEAD visible at run time; that
HEAD alone does **not** identify uncommitted GPU changes or the exact compiled
binary. GPU algorithm provenance is given above. The optimized runs also picked
up the prepared-oracle/replay-export changes in the benchmark driver; these are
outside the timed search call but can affect surrounding cache state and QPS.
Original executable hashes were not captured at execution time. Do not describe
this series as an isolated binary-only A/B experiment. A larger fixed-revision
rerun is needed before strong comparative claims.

## Reproduction

PowerShell; default release profile, no custom `RUSTFLAGS`, two build jobs:

```powershell
$env:CC = 'clang-cl'
$env:CXX = 'clang-cl'
$env:CARGO_INCREMENTAL = '0'
cargo build -p qenlo-bench --release --features 'usearch,gpu-wgpu' -j 2
$env:WGPU_BACKEND = 'dx12'
target/release/qenlo-bench.exe prepare --dataset target/gpu-synthetic-100k384.qnb --rows 100k --dimensions 384 --tuning 4 --evaluation 20 --seed 42
target/release/qenlo-bench.exe run --dataset target/gpu-synthetic-100k384.qnb --dimensions 384 --backend gpu-predicate --fraction 1 --warmups 4 --repetitions 2 --output target/reproduce-all
target/release/qenlo-bench.exe run --dataset target/gpu-synthetic-100k384.qnb --dimensions 384 --backend gpu-rows --fraction 0.01 --warmups 8 --repetitions 3 --output target/reproduce-onepct
$env:QENLO_REQUIRE_GPU = '1'
cargo test -p qenlo --release --features 'usearch,gpu-wgpu' gpu::tests -- --test-threads=1 --nocapture
$env:WGPU_BACKEND = 'vulkan'
cargo test -p qenlo --release --features 'usearch,gpu-wgpu' gpu::tests -- --test-threads=1 --nocapture
python benchmarks/2026-08-28/gpu-tuning/verify.py
```

Use backend `cpu`, `gpu-mask`, or `gpu-predicate` to reproduce the other cells.
Output paths must not already exist. To reconstruct the original kernel, use
the original source revision in a separate checkout; do not overwrite an
active working tree. The synthetic dataset payload CRC32 is `8b058757`; the
complete prepared file's SHA256 is
`f69a08a149fb60cb1eb62b00d28f69a1fe8ef86772735588ff92ac78230c0a4f`.
The 154 MB `.qnb` file is intentionally not checked in. Raw timing samples,
per-run summaries, configurations and available truth IDs are retained here.
The generated 100k-row metadata exports are omitted to avoid duplicate files;
the preparation command deterministically recreates the workload.

`SHA256SUMS` covers all retained evidence, this report, source provenance and
the verifier. The verifier checks every hash and recomputes per-run percentiles
and median P95 from raw samples, plus recall and fallback assertions.
