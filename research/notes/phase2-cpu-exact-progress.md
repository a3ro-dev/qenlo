# Phase 2 CPU exact-search progress

Status: implementation and one external baseline validated; additional hardware cohorts remain open.

## Numerical contract

Canonical `Record` vectors, metadata, tombstones, IDs, and generation remain authoritative. The
aligned scan matrix is derived state: it is rebuilt lazily, invalidated by mutation, and omitted
from clones and persistence. Exact ordering is the FP64-accumulating reference distance followed
by ascending ID for equal `f32` distances.

The selective route computes FP32 AVX2/FMA scores, retains every candidate that can cross the
current top-k boundary under a conservative floating-point error bound, and reranks retained rows
with the existing FP64 implementation. If the bound is unavailable, SIMD support is absent, or
eligible rows exceed the measured selective boundary, execution falls back to the reference path.

## Correctness evidence

On the A40 Runpod host, `cargo test --workspace --all-features -- --test-threads=1` passed all
workspace, Vulkan, recovery, FFI, browser, telemetry, integration, and doc-test groups. The core
suite includes randomized equality against the FP64 oracle across dimensions 1–384, filters,
deletions, k values 1–64, tie ordering, stale plans, mutation invalidation, and derived-matrix
alignment. After adding the measured route boundary, the focused core suite passed 18/18 tests.

## A40 CPU cohort

Hardware and software provenance are stored in the artifact. Each cell used AG News 100k,
D=384, batch=1, 200 warmups, five measured runs of 5,000 queries, completed-call latency, and the
lower-middle convention recorded by the harness. Baseline revision:
`f10f71acb67f72cadacf8a891b9db53312d54f4e`.

| Eligible rows | Baseline p95 | FP32-certified p95 | Change | Recall@10 | Filter violations |
|---:|---:|---:|---:|---:|---:|
| 1,000 | 171.501 us | 142.400 us | -17.0% | 1.00000 | 0 |
| 4,000 | 612.047 us | 440.406 us | -28.0% | 1.00000 | 0 |
| 100,000 | 15.250 ms | 16.775 ms | +10.0% | 0.99998 | 0 |

The two-pass route therefore is not a universal win. Qenlo now selects it only through 4,096
eligible rows and reports the actual route in diagnostics. A separate 100k verification cell used
the reported `Avx2Fma` reference route and measured 14.474 ms p95 with recall 0.99998 and zero
filter violations. Because that code path is materially the baseline path and the runs were
time-separated rather than interleaved, the apparent 5.1% improvement is treated as cohort drift,
not as an optimization claim.

## FAISS CPU Flat comparison

A single-thread `faiss-cpu==1.15.0` `IndexFlatIP` run used the same normalized 100,000-row corpus,
the same 5,000 evaluation queries, k=10, 200 warmups, and five repetitions. Calls were timed around
the completed Python `index.search` call, so Python binding overhead is included. FAISS used its
AVX2 runtime path. Its median-run p95 was 11.154 ms at 95.1--95.5 QPS, versus Qenlo's frozen
baseline p95 of 15.250 ms at 68.9--69.2 QPS. FAISS is therefore 26.9% lower-latency in this cell.

FAISS recall@10 against the benchmark's independent FP64 truth was 0.99984, while Qenlo measured
0.99998. Both exceed the configured 0.99 target, but they are not numerically identical. This is a
strong external baseline and currently beats Qenlo at dense single-query flat search; it is not
evidence for a Qenlo SOTA claim. The comparison covers no metadata filtering, mutation, persistence,
or end-to-end service boundary beyond each library call.

## Evidence and limitations

The complete raw samples, run summaries, configurations, source patch, checksums, FAISS environment,
logs, and host provenance are archived in
`research/artifacts/qenlo-phase2-a40-2026-09-04-final.tar.gz` (SHA-256
`ac3afda446f4943eab128cff458402f7559be53b96c3cf8ce44fcbbd7f8be4df`). Candidate source hashes
inside the final routed verification were:

- `qenlo-core/src/lib.rs`: `b9dbd5b6193eff7d032354561ff2d618ba91f9400b602aeccef685dc2110381c`
- `qenlo/src/lib.rs`: `fcda43b2d4d8995763c661896bb026a4763ea09dba7abd905c4f771bb18179a6`

No SOTA claim follows from this cohort. Phase 2 still requires additional architectures where
available, memory/build/update measurements, and an interleaved confirmation campaign before
paper-level claims are admissible.
