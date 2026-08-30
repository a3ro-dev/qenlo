# changelog

## unreleased

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

### compatibility and limits

- in-memory construction and existing search/filter behavior remain available.
  shared methods accept `&self`; callers can still use mutable bindings.
- format v1 remains explicit. legacy `.tmp` recovery and snapshots without
  `HEAD` are supported; current `.pending` transactions are not promoted.
- no SQL, MVCC, replication, embedding runtime, encrypted storage, saved ANN
  graph, native vendor kernel, NPU execution, PQ, or RaBitQ was added.
- snapshot compaction/restart replay remain synchronous, memory admission is
  estimated, and Windows power-loss durability is not guaranteed.

test outcomes belong in [the verification record](docs/verification.md), not an
unqualified release claim. large-workload performance remains to be measured.
