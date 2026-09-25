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
- In the native compact-row sweep, the outer timer encloses all route work. Each call first traverses the predicate to count eligible rows in `search_batch_inner`, then traverses it again in the WGPU path to materialize a fresh row-ID list. The query and row IDs are uploaded per call; vectors and scratch allocations remain resident. No cached eligible list is measured.

## Changes in this worktree

`qenlo-bench` now accepts `--eligible-count`, allowing dense crossover sampling without abusing rounded fractions. Boundary tests cover zero, fewer-than-k, exact count, and over-population rejection. The strict TopK research-gate script gained a preallocated CUDA prototype; this optimization was developed before the fresh-seed held-out synthetic run.

## Disqualifications and cautions

- The retained TopK cuVS result has recall 0.9 and is excluded from exact comparisons.
- The often-repeated 0.1196 ms number is CUDA-event kernel time from a synthetic prototype. It excludes query upload and result readback and is not the protocol's end-to-end metric.
- The real TopK strict run uses a prefiltered 10,026-row matrix. It compares equivalent exact scoring after filtering, but excludes predicate evaluation/materialization from query latency.
- The native compact-row bracket includes a duplicated predicate traversal in the tested revision. Removing the route-count pass or caching row IDs may move the boundary, so the paper does not present 2k--3k as an optimized compact-row frontier.
- WGPU initialization failed on Runpod even after requesting full driver capabilities. No performance sample was fabricated or substituted.
- The new synthetic matrix checks Qenlo/FAISS top-10 set agreement, not an independent FP64 oracle. The independently-oracled retained TopK cohort supplies the correctness evidence for the equivalent exact formulation.
- Local full-workspace formatting has unrelated pre-existing failures; package-local formatting for `qenlo-bench` passes.

## Addendum 2026-09-24: batch counter defect and E0/E2 audit

- `qenlo-bench` summed `upload_bytes`, `readback_bytes`, and `lock_wait` over the
  responses of one native batch. `ExecutionReport` documents these as batch
  totals repeated on every response, so every batch-B GPU row overstated them
  B-fold. Evidence: E2 B=16 upload 6,794,240 = 16 × (24,576 + 400,000 + 64);
  S2 batch-8 upload 517,120 = 8 × 64,640 and readback 65,792 = 8 × 8,224. The
  harness now records each batch total once and errors if responses disagree
  (run format `qenlo-bench-run-v4`). Latency, recall, allocation, and phase
  timings were not affected. Retained raw fields are unchanged; read B>1
  transfer fields from earlier formats as B × per-call.
- The E0/E2 archive was audited independently: SHA-256, coverage (227/260),
  exit codes, recall values, and all 92 cell medians and 132 paired ratios
  were recomputed from `runs.csv`. Its runner halts on a nonzero exit but
  skipped destinations that already had summaries on resume, which is how the
  exit-101 block entered the data. The runner now validates the sibling run
  record, zero exit code, completed status, and recall gate before resuming past
  a summary. The retained block remains included and explicitly flagged.
- Earlier 25% regret gates are below the same-cell A/A variation now
  measured (E0 ~2×; R1 A/A >25% in 9/16 cells). Future gates need block
  replication and a margin calibrated to measured A/A variation.
- Required shader-predicate batches formerly built the same sorted host row list
  as row and mask modes, although `gpu.rs` consumes no prepared rows in predicate
  mode. They now call `CoreStore::filter_count` and expose row materialization and
  contiguous-run diagnostics as unavailable. Automatic predicate execution still
  retains rows for a possible CPU route or GPU-failure fallback. This was verified
  with count/materialization equivalence tests and a live WGPU diagnostic test;
  no campaign-host speedup is claimed.
