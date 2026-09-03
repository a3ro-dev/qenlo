# Phase 1 eligibility-plan report

Measurement date: 2026-09-04
Base revision: `3faf32e9308e4351f02d017ba16bc12ee479cb8b` plus retained `source.patch`
Raw artifact: `research/artifacts/qenlo-phase1-eligibility-2026-09-04.tar.gz`
Archive SHA-256: `26e0ccc057c59c0fb4687f8d85a923e88b7bb25257e4e551e22cffe13c1b821f`

## Implementation result

`EligibilityPlan` compiles the canonical generation, E, N, predicate shape, execution representation, retained sorted row IDs, estimated eligibility transfer bytes, contiguous-run count, materialization time, traversal count, and cache status. Single-query and batch WGPU paths now use the same plan. Automatic CPU routing and GPU failure fallback consume the plan's rows through a fail-closed exact CPU API rather than traversing the canonical predicate again.

The production default performs one canonical predicate traversal. `LegacyTwoPass` remains available only as an explicit benchmark ablation. The one-entry cache is keyed by exact predicate and canonical generation; mutations cannot reuse a stale entry. `CoreStore::search_rows` revalidates row order, liveness, and every predicate clause before scoring, so malformed or stale derived rows return an error.

Implemented representations are empty, tiny sorted rows, sorted rows, a dense 32-bit mask, and shader-side predicate execution. The latter still retains host rows because the current automatic router needs E before dispatch; it does not upload those rows. Roaring, packed resident bitsets, and persistent device representations remain unimplemented.

## Correctness evidence

- A deterministic randomized property test checks plan rows/count/run statistics against `CoreStore::filter` over 200 inserted rows, randomized users/ranges, tombstones, and a subsequent mutation.
- Cache miss, hit, and post-mutation invalidation are asserted.
- A core test establishes result equivalence between canonical filter search and compiled-row search, then verifies that deletion makes the old row list fail closed.
- `cargo test -p qenlo --features gpu-wgpu -- --test-threads=1`: 47 library tests, CPU oracle integration, recovery tests, and doctest passed.
- `cargo test -p qenlo-bench --bin qenlo-bench --features gpu-wgpu`: 3 tests passed.
- `cargo clippy -p qenlo --features gpu-wgpu -- -D warnings`: passed.
- Runpod compatibility test and GPU smoke cell passed with recall@10=1.

## Matched Runpod ablation

Hardware cohort: Runpod secure cloud, NVIDIA GeForce RTX 4090, Linux/Vulkan, driver 570.211.01. This is not the Phase 0 physical host/driver and is not pooled with it. Workload: retained AG News 100k x 384, independent predicate distribution, B=1, k=10, 200 warmups, five complete runs, 5,000 held-out calls per run. Modes were executed sequentially rather than interleaved, so thermal/time-order bias remains possible.

| E | legacy two-pass P95 | one-pass P95 | cached P95 | one-pass vs legacy | cached vs legacy |
|---:|---:|---:|---:|---:|---:|
| 1,000 | 0.131736 ms | 0.117870 ms | 0.099747 ms | -10.5% | -24.3% |
| 4,000 | 0.305281 ms | 0.208009 ms | 0.143778 ms | -31.9% | -52.9% |
| 6,000 | 0.401130 ms | 0.271618 ms | 0.171330 ms | -32.3% | -57.3% |
| 100,000 | 1.786170 ms | 1.720476 ms | 1.596024 ms | -3.7% | -10.6% |

Values are lower-middle medians of five complete-run P95 values. All cells reported recall@10=1 and no filter violation. This campaign does not justify confidence intervals or a new crossover claim: it was designed as an implementation ablation, used only five complete runs, and did not interleave the three modes.

Median per-call plan materialization was 0 ns for warmed cached calls. For one-pass it was 12.714 microseconds at E=1,000, 55.473 microseconds at E=4,000, 84.528 microseconds at E=6,000, and 64.851 microseconds at E=100,000. The nonmonotonic dense value is retained as measured; E alone does not determine indexed predicate materialization cost.

## Retained failures and limits

The first bootstrap attempt failed because the image lacked `/usr/share/vulkan/icd.d`; its log is retained under `artifacts/failures/bootstrap-missing-icd`. The explicit repair created the directory and NVIDIA ICD manifest, after which Vulkan validation passed. Pod creation also initially omitted the published SSH port; it was updated in place before any benchmark ran.

The cache is deliberately bounded to one predicate/generation entry. There is no resident GPU eligibility cache yet. The current contiguous-run statistic is descriptive and not used for routing. The materialization timer includes canonical filtering and conversion into the shared row slice, while completed-call latency remains the comparison boundary.
