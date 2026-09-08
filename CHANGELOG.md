# changelog

## 0.1.0-alpha.5 - 2026-09-08

### added

- a reproducible evidence reducer plus a preregistered held-out automatic-routing
  gate, with raw runs and processed decisions retained in the repository.
- an immutable format-v1 fixture produced by the published alpha.4 Python wheel
  and imported by the current Rust test suite.
- an explicit compatibility policy and supported-versus-preview SDK tiers.

### changed

- Rust, Python, TypeScript, Kotlin, and documentation versions advance together.
- SDK installation, result-field, distribution, and Apache-2.0 license guidance
  now matches the packages and release pipeline.
- reproducible scratch renders and superseded planning documents were removed;
  unique measurements remain in the evidence inventory.

### compatibility and limits

- the proposed `eligible_rows * dimension * batch` router fit retained
  development data but failed the held-out maximum-regret gate (235.7% versus a
  25% limit), so alpha.5 retains alpha.4 routing behavior.
- `.qn` format v1, WAL v1, query semantics, and the native ABI are unchanged.
- Rust, Python, and TypeScript are supported alpha SDKs. Go, Kotlin/JVM, and
  Swift remain preview surfaces.

## 0.1.0-alpha.4 - 2026-09-07

### added

- a repository-evidence-driven roadmap that ranks adoption, comparative
  coverage, compatibility, mobile validation, and launch work by impact,
  effort, dependencies, and proof of completion.

### changed

- documentation navigation now links directly to the prioritized roadmap.
- package manifests and SDK installation examples now use `0.1.0-alpha.4`.

### compatibility and limits

- no storage format, query semantics, native ABI, execution backend, or
  automatic network behavior changed in this release.

## 0.1.0-alpha.3 - 2026-09-06

### added

- redesigned interactive collection browser UI and local server.
- live Mermaid architecture and execution flow rendering in documentation.
- comprehensive status badges for CI workflows, multi-platform registries, and SDKs.

## 0.1.0-alpha.2 - 2026-09-05

### added

- versioned, checksummed canonical snapshots, staged publication, and a
  checksummed `HEAD` watermark; explicit create/open/flush/close operations.
- shared load/write admission limits through `StorageOptions`, row-at-a-time
  decoding, and missing acknowledged snapshot detection.
- atomic ordered add/delete batches, pre-publication rollback, and explicit
  uncertain-commit errors after publication.
- checksummed immutable WAL transactions with contiguous replay and snapshot
  compaction, removing the full-store clone and O(n) snapshot from each commit.
- atomic checksummed manifests and validated read-only memory mapping for immutable
  canonical snapshots.
- shared collections with concurrent ready CPU searches, serialized mutations
  and rebuilds, and an exclusive OS lock for durable directories.
- disposable index-readiness metadata, preparation reasons, and automatic or
  explicit rebuild policy. restart still rebuilds graphs and GPU buffers.
- runtime AVX2 and AArch64 NEON exact distance with scalar fallback, bounded top-k selection,
  independent oracle checks, and configurable USearch search expansion.
- GPU capability reporting, scratch-inclusive admission, bounded chunks,
  device-loss handling, and required-versus-automatic failure behavior.
- query-level automatic CPU/GPU routing by eligible cardinality with retained
  routing reasons in execution reports and benchmark samples.
- persistent GPU scratch arenas, true multi-query GPU batches, explicit adapter
  selection, deterministic IVF-Flat, and IVF-SQ8 with exact FP32 GPU reranking.
- a shared Linux/Windows/macOS device-lab CLI, Android and iOS native tester
  shells, retained privacy-safe reports, and authenticated telemetry ingestion/viewer.
- operation correlation, lock/commit context, CPU/ANN diagnostics, explicit
  unavailable measurements, and host-owned bounded OTLP setup.
- checksummed deterministic benchmark datasets, disjoint source-row partitions,
  workload manifests, raw samples, and nearest-rank latency summaries.
- explicit in-memory `StorageOptions`, including first-query vector admission,
  and propagation of the benchmark vector budget into collection construction.
- optional lazy PyTorch tensor indexing, bulk Python float32-buffer ingestion,
  typed native result buffers, and shared execution controls across SDK bindings.
- a failure-preserving 182-row small-collection campaign, machine-readable matrix,
  claim-to-artifact ledger, and revised research paper.

### changed

- append and live-mask GPU updates now avoid full resident rebuilds when capacity
  permits; generation checks prevent mixed canonical and derived state.
- the lane-minimum WGSL selector candidate was measured, won five and lost seven
  qualified pairs, and was reverted rather than shipped as a universal change.
- documentation now distinguishes source bindings, package verification, and
  physical mobile evidence.

### compatibility and limits

- in-memory construction and existing search/filter behavior remain available.
  shared methods accept `&self`; callers can still use mutable bindings.
- format v1 remains explicit. legacy `.tmp` recovery and snapshots without
  `HEAD` are supported; current `.pending` transactions are not promoted.
- no SQL, MVCC, replication, embedding runtime, encrypted storage, saved ANN
  graph, native vendor kernel, NPU execution, PQ, or RaBitQ was added.
- snapshot compaction/restart replay remain synchronous, memory admission is
  estimated, and Windows power-loss durability is not guaranteed.

Test outcomes belong in [the verification record](docs/verification.md), not an
unqualified release claim. The current campaign covers 1K--100K vectors; mobile,
Metal/MPS, AMD/Intel Linux, and larger-scale performance remain separate gates.
