# Phase 0 implementation audit

Audit date: 2026-09-04
Frozen baseline revision: `3faf32e9308e4351f02d017ba16bc12ee479cb8b`
Audit scope: the committed revision plus the pre-existing, uncommitted router/row-preparation worktree changes. The two states are distinguished below.

## Baseline preservation

No laptop benchmark was run. The committed revision was archived and executed on one Runpod secure-cloud RTX 4090 host. The first successful retained artifact is `research/artifacts/qenlo-phase0-runpod-2026-09-03.tar.gz` (SHA-256 `bdb93dfdda104eb1fe0ccfc0b52c53bb6202ad189460ea67faab04d848429c74`). It contains raw per-call samples, run summaries, environment captures, source checksums, commands, and three failed setup attempts. The input archive is `research/artifacts/phase0-baseline-head-minimal.tar.gz` (SHA-256 `989819886531103140642a7bf85ce2a6236c7bec61a16e8fda862263d5c49581`). That initial “minimal” archive omitted the desktop Cargo workspace member even though the root manifest names it. The failure is retained; the two clean files from the same commit were uploaded separately and their hashes are recorded in the successful artifact. The minimal archive is therefore an attempt input, not a sufficient clean-clone artifact.

The completed superset is `research/artifacts/qenlo-phase0-runpod-complete-2026-09-04.tar.gz` (SHA-256 `7292fd1cd0e7669c9febeaa2d49baedee747811ed65f6bc704ecbcb6e61f51ea`). Its internal checksums passed before transfer and the downloaded archive hash matches the remote hash. It retains the original four headline runs, the three setup failures, environment evidence, and 15 extension cells. The Runpod pod was then deleted; `runpodctl pod list` returned an empty list.

The successful headline cells used the retained 100k by 384 AG News prepared dataset, batch 1, k=10, 200 warmups, 5 complete runs, and 5,000 held-out calls per run. Required-GPU mode was enabled. Every measured call reported the requested backend, no fallback, recall@10=1, and no filter violation.

| E | CPU median-run P95 | WGPU-row median-run P95 | winner on this host |
|---:|---:|---:|---|
| 2,000 | 0.194723 ms | 0.263006 ms | CPU |
| 3,000 | 0.288435 ms | 0.367323 ms | CPU |

This is a new RTX 4090/Linux/Vulkan cohort, not a numerical reproduction of the retained RTX 4050/Windows crossover. In particular, it demonstrates that the 2k--3k crossover is not portable across hosts or GPU implementations.

The extension used the same host, dataset, B, k, warmups, complete-run count, and calls per run. These cells were not randomized or interleaved and have only five complete runs, so they are Phase 0 characterization rather than headline inferential evidence.

| E | CPU median-run P95 | WGPU-row median-run P95 | winner |
|---:|---:|---:|---|
| 1,000 | 0.101903 ms | 0.177678 ms | CPU |
| 4,000 | 0.401054 ms | 0.414334 ms | CPU |
| 6,000 | 1.311405 ms | 0.591601 ms | WGPU rows |
| 8,000 | 2.404601 ms | 0.593363 ms | WGPU rows |
| 10,000 | 3.754374 ms | 0.680685 ms | WGPU rows |
| 100,000 | 5.669167 ms | 1.774663 ms | WGPU rows |

At E=100,000, WGPU predicate was 0.971804 ms and WGPU mask was 1.126477 ms, both faster than WGPU rows on this host. At E=1,000, WGPU predicate was 0.881287 ms. These representation results are completed-call measurements but are not a matched full representation sweep at every E.

All extension runs passed the configured recall gate and reported zero filter violations. E below 100,000 had recall@10=1. At E=100,000 every backend reported 0.99998 against the retained independent FP64 truth, including the CPU route; this shared one-result discrepancy must be investigated as oracle/tie-policy evidence before describing the dense cells as exact ID agreement.

## Canonical and derived state

`qenlo_core::CoreStore` is the logical authority. It owns normalized FP32 row vectors, public IDs, user IDs, signed timestamps, permanent row slots, tombstones, metadata indexes, and a monotonically increasing mutation generation. Deletions remove live slots from metadata indexes but retain records and IDs. Restore validates dimensions, finiteness, nonzero vectors, normalization tolerance, uniqueness, live counts, and generation.

`Collection` holds the canonical store behind a read/write lock. CPU exact, USearch, WGPU buffers, IVF lists/codes, and `index.qidx` are derived. A mutation invalidates the prepared generation. `prepare` rebuilds USearch or all resident GPU columns from the canonical rows. `index.qidx` stores only a checksummed readiness marker; a restart still rebuilds the accelerator.

Durable state uses immutable checksummed snapshots and WAL generations, a checksummed `HEAD`, and a checksummed manifest. WAL replay requires contiguous generation ranges. Corruption and missing acknowledged generations fail closed. A post-publication durability failure returns `CommitUncertain`; reopen resolves the newest validated generation. These properties must remain unchanged by later execution work.

## Completed-call execution audit

### Query validation and allocations

The public `search` and `search_batch` entry points normalize each query once for validation and discard the resulting `Vec<f32>`. The selected backend then normalizes the same query again. This causes two normalization passes and two query-vector allocations per query on every current route.

The source-visible large allocations for one CPU exact call are therefore:

1. discarded validation query vector, length D;
2. backend query vector, length D;
3. eligible-slot vector, length E;
4. binary heap backing vector, capacity k;
5. converted hit/result vectors at the core and collection boundaries.

This is a lower bound on allocation events, not a measured allocator count. The current report exposes no CPU allocation count or allocator-byte total. Driver, standard-library, tracing, and executor allocations are also not counted.

For WGPU batch execution, source-visible transient host allocations include the discarded validation vectors, one normalized vector per query, a borrowed-slice vector, an optional flattened B by D query vector, the host eligibility representation, outer/per-query merge vectors, and final response vectors. CPU-mask mode additionally creates and fills an N-entry boolean vector. Compact-row mode creates an E-entry row vector. WGPU-predicate mode avoids those representations after routing, but automatic routing still materializes a CPU filter result to learn E.

GPU buffers for vectors, IDs, users, timestamps, scores, query data, eligibility, parameters, candidates, selection, and staging are persistent after preparation. Scratch is recreated only when a larger batch exceeds its current capacity. A bind group and command encoder are nevertheless created for every chunk of every call.

### Predicate traversals

In committed automatic batch routing, a nontrivial predicate is traversed once to compute E. CPU-mask and compact-row GPU execution then calls `CoreStore::filter` again to materialize the representation. The shipped compact-row path therefore performs two complete host predicate traversals per call. `Filter::ALL` uses `live_len` for routing and needs only the materialization traversal.

The pre-existing worktree adds `LegacyTwoPass`, `OnePass`, and a one-entry generation/predicate-keyed `Cached` row list for `search_batch`. It does not change the single-query `search_inner` path, so normal `search()` calls still use the committed two-pass behavior. Its WGPU-predicate diagnostic reports one traversal for the CPU E count but does not count the shader-side predicate scan; the field's documentation says it covers traversals “before or during” the search, so the current value is incomplete. The cache is invalidated after successful mutation and also guarded by generation and exact `Filter` equality.

### CPU exact scan

CPU exact evaluates every eligible slot. The row layout is one `Vec<f32>` allocation per canonical record; the scan follows record/row indirections rather than a contiguous derived matrix. Runtime dispatch selects AVX2+FMA, AVX2, AArch64 NEON, or scalar code. All implementations convert FP32 lanes to FP64 and accumulate in FP64. There is no FP32 production scan, AVX-512/SVE path, SimSIMD route, or parallel exhaustive scan.

Top-k uses a `BinaryHeap` capped at k, so scan time is O(E times (D + log k)) and auxiliary top-k space is O(k). Ordering is computed FP32 distance followed by unsigned ID. The benchmark validates exact IDs against an independent FP64 oracle, permitting only a documented 1e-5 boundary-tie case.

### WGPU exact scan

The score shader assigns 32 lanes to each vector, reduces their FP32 partial dots in workgroup memory, and writes one global FP32 score per dispatched item and query. The score buffer therefore receives B times E values for compact rows or B times N values for mask/predicate modes.

Selection launches one 256-thread workgroup per query. For each of k output ranks, every lane scans its share of the entire score array, a tree reduction elects one candidate, and the winner's score is invalidated. Selection is O(B times rows times k). For k=10, one nonempty chunk uses two dispatches: score and select. Empty compact rows use selection only.

Per chunk, completed-call execution uploads query bytes once per call, then eligibility plus a fixed parameter block; it reads back B times k times 16 bytes. GPU-mask uploads approximately N 32-bit values per chunk, compact rows upload E 32-bit row IDs, and predicate mode uploads a one-word placeholder plus parameters. The code reports Qenlo-owned buffer capacity, not physical VRAM residency.

### Synchronization, locking, and concurrency

`Collection` holds a canonical read guard throughout search. WGPU calls additionally hold `gpu_gate`, while `GpuExact` holds its own synchronous scratch `Mutex`. Thus one collection admits only one GPU call at a time even though the queue could accept more work.

For each chunk, the backend submits commands, maps the staging range, calls device polling with wait semantics, parses the mapping, and unmaps it before submitting the next chunk. Compute, readback, and CPU merge are not pipelined across chunks. There is one completion synchronization per chunk. Kernel timestamp queries are not used, so current phase timing is host-observed and cannot isolate queue wait, scoring, selection, copy, or map latency.

### Automatic routing

The committed production rule routes automatic batch calls to CPU when `E < 4096` and B < 8; B >= 8 bypasses the CPU threshold. At or above 4,096 it requests the configured WGPU representation. A GPU error in automatic mode falls back to exact CPU and records a reason; required-GPU mode returns the error.

The pre-existing worktree can replace the threshold with one `RouterProfile` only when adapter name, D, B, WGPU filter mode, and cache state match exactly. Otherwise it silently returns to the 4,096 rule, though the routing reason identifies “static fallback.” It is not yet a general policy abstraction: always-CPU/WGPU are backend selections rather than comparable policies, oracle-best is absent, k/N/selectivity/fragmentation/transfers/queue state are absent, profiles are not persisted, confidence and predicted latency are absent, and there is no calibration command or held-out regret evaluator.

## Observability gaps to close before optimization claims

- CPU allocation counts and allocated bytes are unavailable.
- Host allocation counts for WGPU transient representations are unavailable.
- Validation/query normalization, routing, predicate counting, materialization, queue wait, score kernel, selection kernel, copy, mapping, and host merge do not have mutually exclusive phase timings.
- “lock wait” combines collection-lock and GPU-gate waits; scratch-mutex wait is not separately reported.
- WGPU-predicate candidate count is unavailable under basic diagnostics; detailed diagnostics adds another CPU scan and changes the measured boundary.
- GPU timestamp support is not queried or recorded.
- Queue depth and scratch availability are not observable to the router.
- The benchmark records completed-call latency correctly, but does not interleave different backends within one run block.

## Product/documentation contradictions found during the audit

- `PRODUCT.md`, `docs/trade-offs.md`, `docs/architecture.md`, and the Rust library describe explicit host-owned telemetry, local operation, and no installed exporter. In contrast, the root README and the Python/TypeScript SDKs declare mandatory telemetry with no opt-out. Python sends a background HTTPS request at import time; TypeScript contains the same fixed endpoint. This is not merely stale copy and violates the requested local-first privacy boundary.
- The root README says MIT-or-Apache-2.0 and refers to `LICENSE-MIT`, while the workspace package metadata and repository license are Apache-2.0. Release metadata is internally inconsistent.
- `docs/implementation-status.md` labels the query-level router “implemented,” but the committed router is only the fixed 4,096/B>=8 rule and has no measured regret.
- `paper/REPRODUCE.md` names a repository SHA plus worktree changes. No immutable revision contains that state.
- The Python and TypeScript SDK documentation uses “telemetry” both for local execution reports and remote collection, obscuring a security-significant distinction.

## Phase 0 validation status

After the committed baseline was frozen, the worktree benchmark gained two diagnostic-only measurements: gross process-wide allocation calls/bytes around each completed API call, and host-side queue-write enqueue/readback-completion durations. The latter are explicitly not GPU kernel timestamps. These additions do not change route selection, scoring, storage, or the timing boundary.

- `cargo test -p qenlo --features gpu-wgpu -- --test-threads=1`: pass.
- `cargo test --workspace -- --test-threads=1`: pass.
- `CC=clang-cl CXX=clang-cl cargo test --workspace --all-features -- --test-threads=1`: pass.
- Python script unit tests: 13/13 pass.
- `cargo fmt --all -- --check`: fail; numerous pre-existing formatting differences, including the browser crate and the dirty benchmark/router changes.
- `cargo clippy --workspace --all-targets -- -D warnings`: fail in the pre-existing worktree changes (`derivable_impls`, `await_holding_lock`, and `too_many_arguments`).
- A later default-MSVC all-feature clippy attempt failed earlier in the external `numkong` C build with an MSVC internal compiler error. The earlier `clang-cl` all-feature test remains the valid successful Windows build record; no source warning conclusion is drawn from the failed MSVC invocation.
- Paper build: direct `pdflatex` succeeds at 10 pages with no unresolved references/citations; `latexmk` itself is unavailable because the local MiKTeX installation has no Perl engine. Page-by-page rendering found no clipping or missing glyphs, but Figure 5's Qenlo CPU and FAISS Flat labels overlap and the architecture figure is visually crowded. These are retained layout defects, not silently edited during baseline collection.

The baseline is now frozen. No performance optimization is claimed by this audit.
