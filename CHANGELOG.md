# changelog

## unreleased

### added

- versioned, checksummed canonical snapshots, staged publication, and a
  checksummed `HEAD` watermark; explicit create/open/flush/close operations.
- shared load/write admission limits through `StorageOptions`, row-at-a-time
  decoding, and missing acknowledged snapshot detection.
- atomic ordered add/delete batches, pre-publication rollback, and explicit
  uncertain-commit errors after publication.
- shared collections with concurrent ready CPU searches, serialized mutations
  and rebuilds, and an exclusive OS lock for durable directories.
- disposable index-readiness metadata, preparation reasons, and automatic or
  explicit rebuild policy. restart still rebuilds graphs and GPU buffers.
- runtime AVX2 exact distance with scalar fallback, bounded top-k selection,
  independent oracle checks, and configurable USearch search expansion.
- GPU capability reporting, scratch-inclusive admission, bounded chunks,
  device-loss handling, and required-versus-automatic failure behavior.
- operation correlation, lock/commit context, CPU/ANN diagnostics, explicit
  unavailable measurements, and host-owned bounded OTLP setup.
- checksummed deterministic benchmark datasets, disjoint source-row partitions,
  workload manifests, raw samples, and nearest-rank latency summaries.

### compatibility and limits

- in-memory construction and existing search/filter behavior remain available.
  shared methods accept `&self`; callers can still use mutable bindings.
- format v1 remains explicit. legacy `.tmp` recovery and snapshots without
  `HEAD` are supported; current `.pending` transactions are not promoted.
- no SQL, WAL, MVCC, replication, embedding runtime, encrypted storage, saved ANN
  graph, or custom GPU ANN was added.
- full-snapshot transactions remain O(n), memory admission is estimated, and
  Windows power-loss durability is not guaranteed.

test outcomes belong in [the verification record](docs/verification.md), not an
unqualified release claim. large-workload performance remains to be measured.
