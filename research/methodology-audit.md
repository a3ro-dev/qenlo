# Methodology audit

## Verdict

The repository harness has unusually strong oracle, predicate, failure-retention, and backend-observability checks. The retained 100k×384 Rust runs are suitable for bounded claims. The A6000 TopK and CUDA-prototype runs are context experiments and must not be described as shipped-Qenlo measurements.

## Checks performed

- `cargo test --workspace --no-default-features -- --test-threads=1`: all 84 unit/integration tests plus doctests passed on 2026-09-02.
- `cargo test -p qenlo-bench --no-default-features`: passed after adding exact eligible-count selection.
- `cargo test -p qenlo --features gpu-wgpu -- --test-threads=1`: passed (45 library, one CPU-quality, two recovery, and one doctest). Device-dependent smoke tests are conditional on adapter availability.
- `cargo test -p qenlo --features usearch -- --test-threads=1`: attempted, but the Windows MSVC compilation of the upstream C++ dependency made no progress for several minutes and was interrupted; no test result is claimed. Retained USearch benchmark artifacts remain available.
- Oracle: `PreparedOracle` exhaustively scores eligible rows using independent FP64 code and is constructed before timing.
- Result validation rejects duplicate, deleted, unknown, or predicate-ineligible IDs and checks `min(k,E)` cardinality.
- Runs write configuration before samples and only become complete when a summary exists. Failed systems remain in `failures.json` or logs.
- USearch expansion candidates are evaluated on the tuning range; the evaluation range is disjoint.
- Per-query benchmark latency is the completed batch call. WGPU reports host-observed time, not kernel time.

## Changes in this worktree

`qenlo-bench` now accepts `--eligible-count`, allowing dense crossover sampling without abusing rounded fractions. Boundary tests cover zero, fewer-than-k, exact count, and over-population rejection. The strict TopK research-gate script gained a preallocated CUDA prototype; this optimization was developed before the fresh-seed held-out synthetic run.

## Disqualifications and cautions

- The retained TopK cuVS result has recall 0.9 and is excluded from exact comparisons.
- The often-repeated 0.1196 ms number is CUDA-event kernel time from a synthetic prototype. It excludes query upload and result readback and is not the protocol's end-to-end metric.
- The real TopK strict run uses a prefiltered 10,026-row matrix. It compares equivalent exact scoring after filtering, but excludes predicate evaluation/materialization from query latency.
- WGPU initialization failed on Runpod even after requesting full driver capabilities. No performance sample was fabricated or substituted.
- The new synthetic matrix checks Qenlo/FAISS top-10 set agreement, not an independent FP64 oracle. The independently-oracled retained TopK cohort supplies the correctness evidence for the equivalent exact formulation.
- Local full-workspace formatting has unrelated pre-existing failures; package-local formatting for `qenlo-bench` passes.
