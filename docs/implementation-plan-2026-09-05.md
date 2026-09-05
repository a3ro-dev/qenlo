# Qenlo implementation plan: small collections, useful acceleration

Status: proposed. Implementation paused at the user's request on September 5, 2026.

## Direction

Make Qenlo simple to use and fast for **1K–100K vectors**, with resource use treated as part of performance. Keep one durable Rust core and two deployment profiles:

| Profile | Platforms | Priorities | Optional capabilities |
| --- | --- | --- | --- |
| Embedded | Android, iOS, Linux on phones | Low resident memory, bounded scratch, fast single queries, predictable mutation cost, small binaries | Portable GPU acceleration when it improves completed-call performance within the memory budget |
| Accelerated | Windows, macOS, desktop Linux | Fast single and batch queries, convenient ingestion, good diagnostics and packaging | Portable GPU, PyTorch tensor integration, existing desktop inspection tools |

These are build and configuration choices, not separate database implementations. Phones do not inherit desktop dependencies. Desktop conveniences must use the same canonical records, filtering, deletion and persistence semantics.

“Best performance per resource” means measuring latency and throughput alongside peak RAM, accelerator allocations, scratch, transfer volume, build cost and mutation cost. There is no power-measurement project and no single invented efficiency score.

GPU investment remains in scope. The earlier million-vector research gate is historical evidence, not a veto on the new small-collection objective. Beating Chroma is a concrete benchmark target, not a result to promise in advance.

## 1. Implement improvements

### 1.1 Establish resource and correctness contracts

- Preserve canonical generations, fail-closed stale-plan checks, atomic writes and tombstones.
- Keep cosine distance, existing ID semantics and distance-then-ID tie ordering consistent across SDKs.
- Separate exhaustive FP32 search from numerical agreement with an FP64 oracle. Do not weaken accuracy silently to improve a chart.
- Audit collection load admission against the allocations triggered by the first query: canonical vectors, aligned scan storage, metadata indexes, resident GPU buffers and scratch. Reject over-budget preparation explicitly.
- Keep detailed diagnostics available, but remove avoidable allocation from the normal search path after profiling it.
- Define explicit CPU, automatic and required-GPU behavior. Required GPU must fail clearly; automatic mode must report its actual route and any fallback.

Deliverable: documented resource accounting and focused regression checks for admission, fallback and stale state.

### 1.2 Improve the portable GPU hot path first

Work in the existing Rust/wgpu/WGSL backend so improvements can reach Vulkan, Metal and DX12 without maintaining three kernel implementations.

1. Finish and validate the lane-minimum top-k optimization already drafted. Each lane retains its minimum; only the lane whose candidate won rescans its stripe. Preserve global distance/ID ordering, empty eligibility, partial workgroups and maximum k.
2. Measure scoring, selection, eligibility preparation, dispatch and completed readback separately. Use phase timings to explain performance; use completed calls to compare engines.
3. Reuse bind groups and scratch where their lifetimes permit it. Check actual allocation and submission counts before adding caches.
4. If selection still dominates at 10K–100K, implement bounded hierarchical selection: parallel tile top-k, then merge candidates. Account for candidate scratch and dispatch overhead. Keep the simpler selector where it wins.
5. Evaluate query tiling for batches, reusing vector loads across queries. Keep a direct batch-one path so throughput work does not penalize interactive search.

Acceptance: no result-semantic regressions; benchmark each change against the frozen pre-change revision; retain a more complex kernel only when its measured benefit justifies its memory and maintenance cost.

### 1.3 Reduce work after mutations

The current full rebuild/upload after a mutation can outweigh warm-query gains.

- Trace invalidation from add/delete through preparation before changing it.
- Add append and live-mask updates where existing GPU capacity permits them, with bounded growth. Keep full rebuild as the fallback.
- Publish derived updates against a canonical generation; concurrent searches must never mix generations or return deleted rows.
- Measure add-one-then-search, delete-then-search and small write batches alongside static-corpus queries.

Acceptance: immediate deletion correctness and lower first-query-after-mutation cost without an unbounded delta log or an additional source of truth.

### 1.4 Make PyTorch a supported optional desktop feature

The current draft is an immutable tensor index. It is not yet a durable collection backend.

- Support explicit CPU, CUDA and macOS MPS devices, with independent validation on each available platform.
- Accept contiguous tensors/arrays and return tensors without mandatory JSON conversion. Batch queries through matrix operations.
- Handle tied cutoffs deterministically; test identical vectors, near ties, extreme finite inputs, empty indexes and unsigned-ID compatibility with the native core.
- Keep tensor/native imports lazy and PyTorch an optional dependency.
- Add a canonical collection-to-index integration before describing it as a database backend. Bind snapshots to generation and filter; reject or rebuild stale snapshots after mutations.
- Specify ownership, lifetime, memory limits, normalization and precision settings. Avoid changing application-global PyTorch settings inside the library.
- Reuse an application's existing embedding runtime when possible. Bundling PyTorch solely for a small search workload must remain optional.

Acceptance: a documented usable tensor API, canonical integration tests, and honest platform/semantic limitations. A fast detached matrix alone does not complete this milestone.

### 1.5 Improve SDK access without adding phone overhead

- Extend the shared native ABI with explicit backend configuration, allocation budget and a typed result-buffer path; preserve existing entry points.
- Finish Python bulk float32-buffer ingestion to avoid per-component Python assignment. Validate shape, contiguity, lifetimes and atomic rejection.
- Expose the same supported execution controls through Python, TypeScript, Swift, Kotlin and Go, using each SDK's existing conventions.
- Verify actual Android packaging and bridge support separately from Kotlin/JVM support. Verify Apple device and simulator artifacts separately.
- Keep desktop inspection and richer integration as optional packages/features. Audit background work, including existing telemetry, before claiming an idle or offline embedded footprint; document and make any retained network behavior explicit.

Acceptance: runnable examples, ABI conformance and platform-appropriate packages. No SDK advertises acceleration that its native artifact cannot execute.

## 2. Run bounded local checks

Do this after the implementation work, before paid GPU tests.

- Limit local builds to one job and numerical checks to one or two CPU threads.
- Use small deterministic correctness fixtures, not the full performance matrix.
- Run Rust core/storage tests, portable shader correctness tests, FFI checks and affected SDK tests.
- Exercise k=1/10/64, dimensions around SIMD/workgroup boundaries, ties, sparse/empty filters, tombstones, generation changes, allocation failures and device fallback.
- Run formatting, lint and package checks appropriate to changed components. Move expensive cross-platform builds to CI or the remote reference host.
- Check available RAM and disk before builds; stop rather than allowing build caches or test corpora to crowd out normal laptop use.

Gate: functional checks pass before provisioning. Local correctness checks do not count as a GPU performance result.

## 3. Run the Runpod matrix within $5 for the day

### Campaign controls

1. Check billing since midnight in the user's Asia/Calcutta timezone, active resources and live prices. Subtract today's existing spend from $5.
2. Reserve at least $1 of the remaining allowance for startup, storage, billing lag and cleanup. Target at most $4 total daily exposure before that reserve; reduce the campaign if prior spend exists.
3. Choose approximately ten configurations from current availability. Prefer a spread of inexpensive GPU architectures and memory tiers over ten identical large accelerators. Candidate families include RTX 2000/4000 Ada, L4, A40, RTX 3090/4090 and A6000; exact choices depend on live quotes.
4. Run a short pilot first to measure installation time, benchmark duration and cost. Expand only after the pilot produces valid artifacts.
5. Reuse one versioned workload bundle and pinned dependencies. Use small waves and short deadlines; calculate worst-case exposure before each creation.
6. Record each created pod ID immediately. Start an independent deletion watchdog, monitor actual rates and retain a persistent cost ledger. Do not launch if monitoring or cleanup is unhealthy.
7. Download results, delete each campaign pod, verify deletion and reconcile final billing. Do not retain disks or volumes. Never delete unrelated resources.

Ten configurations are a coverage target. The $5 ceiling takes precedence over pod count. Failed and unavailable configurations remain in the report, with their cost and reason.

### Workloads

| Dimension | Primary sweep | Additional checks on reference hosts |
| --- | --- | --- |
| Collection size | 1K, 10K, 100K | 5K, 25K, 50K around observed changes |
| Vector dimension | 128, 384 | 768 if the budget permits |
| Query batch | 1, 16 | 8, 64 |
| k | 10 | 1, 64 |
| Eligible fraction | Unfiltered and 1% | 10%, compound user/time filters |
| Data | Fixed synthetic corpus and disjoint queries | Retained real embeddings, with provenance |
| Lifecycle | Warm resident queries | Cold preparation, first query after add/delete, reopen |

Do not run the full Cartesian product on every pod. Use a common primary sweep everywhere and deeper tests on one or two reference configurations.

### Comparisons and timing

- Compare the frozen Qenlo revision, improved native CPU, improved native portable GPU and the optional tensor implementation as distinct engines.
- Include Chroma HNSW, FAISS Flat and NumPy. Add USearch and other existing harness adapters on the reference host if time permits.
- Report ANN recall with latency. An approximate Chroma result is not automatically equivalent to exhaustive Qenlo search.
- Time host query input through host-visible IDs and distances for completed-call comparisons. Report device-resident tensor timing separately.
- Keep identical corpora, queries, filters and k. If using a prefiltered corpus, label it and measure filter preparation separately; also retain an end-to-end filtered comparison.
- Record build time, peak RSS, attributable device allocations, scratch, transfers and post-mutation preparation. Distinguish process-wide memory high-water marks from per-engine allocations.
- Warm up consistently, rotate engine order, collect repeated runs and retain raw samples, versions, seeds, hashes, actual adapter and failures.
- Compute independent FP64-oracle recall. Predeclare correctness qualification and retain disqualified rows visibly.

Runpod establishes accelerator behavior on those machines. Android, iOS and phone Linux performance needs a separate physical-device run of the native implementation; cloud NVIDIA results cannot establish mobile performance.

## 4. Publish the performance matrix

Generate CSV, machine-readable summaries and a readable report from the raw artifacts.

Each row identifies revision, device, backend, workload, timing scope, P50/P95/P99, throughput, recall, build time, memory scope, transfer volume and status. Include sample counts and uncertainty where the repetition count supports it.

Show paired before/after Qenlo results and Qenlo/Chroma results at comparable correctness. Highlight wins and losses. Record unavailable native GPU drivers and missing mobile evidence rather than substituting tensor results under the native backend's name.

Select defaults from measured latency/resource tradeoffs. Avoid a learned router or a large configuration surface unless simpler fixed choices demonstrably fail.

## 5. Rewrite and finish the research paper

Only after the matrix and artifact audit:

- Reframe the manuscript around small-collection execution and the embedded/accelerated deployment distinction.
- Preserve earlier crossover and negative results as revision-specific historical evidence. Do not relabel old measurements as results of the new implementation.
- Explain the selector change, its correctness invariant, memory cost, tensor integration and remaining platform gaps.
- Separate native portable GPU, PyTorch CUDA/MPS, CPU and mobile cohorts. Include failures and competitor wins.
- State which hypotheses the new experiments support and which remain untested. “Fastest database” stays a goal unless evidence supports a precisely scoped claim.
- Apply research-engineer for factual rigor, humanizer-zh for removing inflated/repetitive prose, and akshat-voice for direct ownership and clear explanations while preserving mathematical notation and technical names.
- Update citations and the claim-to-artifact ledger. Compile the PDF and visually check every page, table, figure and reference.

Deliverable: revised manuscript, verified PDF, reproducible build instructions and evidence links. Venue formatting waits for a specified venue.

## 6. Documentation, version and release

- Update README, quickstart, architecture, GPU design, SDK guides and implementation status around the two profiles.
- Replace unsupported API/performance claims with tested examples and qualified results. Explain installation size, precision, persistence and fallback behavior.
- Write the changelog from the final diff and measured results. Choose the next unused prerelease version and synchronize package manifests.
- Complete release CI, package-content inspection, native ABI checks and supported-platform artifact verification.
- Push the reviewed implementation and version, then publish through the existing release workflow when its gates pass. Do not claim all registries published merely because a Git tag exists.
- Verify clean consumer installation and a small add/filter/search/reopen example from published artifacts.

## 7. Cleanup and handoff

Verify zero campaign resources remain and report the final known cost, including any billing delay. Retain raw evidence and release artifacts; remove only task-created disposable files. Preserve user work and the original proposal. Finish with release links, matrix, paper, tests and explicit remaining platform limitations.

## Existing uncommitted work at the pause

- Portable WGSL lane-minimum selector draft.
- Optional `TorchIndex`, lazy Python imports and the `torch` package extra.
- Python `add_buffer` ingestion draft.
- Two small tensor conformance tests and a first portable benchmark script.
- Completed checks: 18 Rust core tests, 47 native collection/GPU library tests, and two tensor CPU tests passed. These are correctness results, not performance claims.
- The native FFI debug build completed; ABI/SDK conformance and release validation remain pending.
- No Runpod pods were created; no paid benchmark results or performance matrix exist for this revision.
- No version bump, commit, push, publication or paper rewrite has been performed.

The drafts need the integration, resource accounting and benchmark-methodology work above before release. In particular, the first benchmark script is a prefiltered synthetic sweep; it does not yet fulfill the full campaign plan.
