# Canonical data, disposable indexes

A derived index is allowed to be missing. Canonical rows are not. That distinction
drives the storage protocol, locks, and error behavior.

## Ownership and format

`CoreStore` owns normalized vectors, public IDs, metadata, permanent row slots,
tombstones, and mutation generations. User and timestamp indexes are rebuilt from
those rows. Deleted IDs cannot be reused. Generation increments per successful
mutation, not per batch.

`Collection` wraps state in a read/write lock. It owns the backend, preparation
state, storage options, and the durable directory's OS lock. Use one shared
`Arc<Collection>` rather than repeatedly opening the same directory.

The little-endian v1 header contains magic, version, dimension, generation,
total rows, and live rows. Each row has an ID, user ID, signed timestamp, live flag,
reserved bytes, and normalized float32 vector bits. A trailing CRC32 covers the
header and rows. Loading preserves normalized bytes rather than normalizing them
again. Unknown versions and inconsistent shapes return explicit errors.

| File | Meaning |
| --- | --- |
| `collection.lock` | Handle under an exclusive OS file lock |
| `canonical-<20-digit-generation>.qdb` | Published canonical snapshot |
| `canonical-<generation>.pending` | Current uncommitted staging file |
| `HEAD` | 20-byte magic, acknowledged generation, and CRC32 watermark |
| `HEAD.pending` | Staged replacement watermark |
| `index.qidx` | Checksummed dimension, backend, and readiness generation (not a graph) |
| `index.pending` | Staged disposable readiness marker |

Valid newer legacy `.tmp` snapshots retain their recovery behavior. Current
writes use `.pending` so fully written but unpublished transactions are not
mistaken for commits.

## Transaction and durability order

One write lock covers the entire transaction:

1. Clone canonical state and apply the ordered batch. Invalid vectors, duplicate
   IDs, invalid deletions, and exhausted generations abort the staged copy.
2. Check the staged store's admission budget before writing.
3. Stream a complete `.pending` snapshot, append the CRC32, flush, and sync the file.
4. Rename to `.qdb`, then sync the collection directory where supported.
5. Write and sync `HEAD.pending`, rename over `HEAD`, and sync the directory.
6. Prune older snapshots while retaining the immediately preceding one, publish
   staged memory, invalidate preparation, and return success.

Validation and pre-publication I/O failures leave original memory unchanged.
Canonical rename marks publication. Subsequent failure indicates uncertainty, not
rollback: `CommitUncertain` closes the handle and releases the OS lock. Reopen
resolves the outcome. Durable single-row mutations already execute this protocol;
`flush` is normally a no-op.

The retained previous snapshot helps manual investigation. It is not permission
for automatic rollback. `HEAD` is a lower bound on acknowledged generation: if
its snapshot is missing, opening the collection returns an error. The highest published generation is checked
in full, and corruption does not cause fallback. A valid `.qdb` newer than `HEAD`
can originate from uncertain publication. Reopen repairs and syncs the watermark
before returning it. Valid legacy files without `HEAD` are upgraded similarly.

Current `.pending` contents never become visible on reopen. Interrupted initial
creation may leave no canonical snapshot at all. Open then returns an error, while create
refuses a non-empty directory. Inspect and preserve the directory before manually cleaning
confirmed uncommitted initialization files or using another location.

Unix syncs publication and newly created directory entries, including their
parents, subject to filesystem and hardware guarantees. Windows syncs files but does
not implement directory sync. Process-crash recovery is supported; complete
sudden-power-loss durability is not claimed. CRC32 is not authentication.

## Memory and write costs

The default `StorageOptions::max_load_bytes` is 512 MiB. Both read and durable
write admission check snapshot size and the estimate
`rows * (32 + 4 * dimension + 64 * ceil(dimension / 16) + 512)` with checked
arithmetic. The aligned term accounts for the disposable exact-CPU scan matrix
that may appear on the first query; the final 512 bytes per row allow for metadata
indexes and bookkeeping. This estimate includes tombstones, does not measure resident set size (RSS), and does
not pre-reserve memory. The host supplies options on create or open; they are not a
persisted machine policy.

`CollectionConfig::gpu_allocation_budget_bytes` is a separate device-allocation
scope. Exact GPU preparation rejects resident vector, ID, and metadata buffers plus the
admitted scratch arena when their checked total exceeds this cap. Scratch includes
query, eligibility, score, selected-candidate, and readback buffers, growing to the
largest admitted batch. Host canonical memory, preparation copies, driver-private
allocations, and physical residency fall outside this number. Automatic mode reports
an exact-CPU fallback after a GPU budget failure; required-GPU mode returns the
failure. Hosts requiring a process-wide limit must account for both Qenlo scopes and
their own inputs.

Loading keeps one decoded row outside the growing canonical store. Directory
selection and pruning retain only the generation candidates they need. No second
full decoded snapshot is staged, though canonical rows and all metadata indexes
remain resident. Allocation can still fail.

Transactions validate the complete ordered batch before publishing a checksummed
immutable WAL file and atomic manifest, then apply it to the resident store. Commit
work is O(batch), not O(collection). Reopen maps and validates the latest immutable
canonical snapshot, then replays contiguous WAL generations. `flush` and `close`
synchronously compact the current store into a new full snapshot and prune covered
WAL files. Background compaction and zero-copy row ownership are not implemented yet.

## Visibility and locks

Ready CPU and USearch searches hold a shared read lock for the entire query.
Mutations and rebuilds take the write lock, so a query sees a complete committed
generation. Search-triggered preparation drops its initial read lock, acquires a
write lock, prepares the current generation, and downgrades before search.
No stale graph is served during that transition.

GPU queries also take a per-collection gate so simultaneous use of the persistent
scratch arena cannot multiply the budget. Mutations wait for readers. There are no collection background
workers or speculative MVCC versions. `search_batch` holds one canonical generation
and forms a single GPU workload when GPU routing is selected.

For prepared exact-GPU state, deletion updates the owning chunk's host live mask;
the next query uploads that mask through the existing bounded eligibility arena.
Appends upload suffix rows as at most eight additional chunks. The ninth append
wave consolidates through the full preparation path. Budget failure, device error,
and IVF configuration also trigger full preparation. The collection write lock
publishes the canonical mutation and its derived update as one visible generation,
preventing readers from mixing old vectors with new tombstones.

Synchronous methods block. Do not block a single-thread executor on mutations
while an earlier GPU future needs that executor to finish. Place contended
synchronous work on a blocking thread. Async signatures do not make snapshot I/O
or CPU distance calculations nonblocking.

## Preparation and backend boundaries

`index.qidx` stores only readiness metadata. No ANN graph, metadata tree, or GPU
buffer is serialized. Restart always rebuilds, even when its generation matches.
Missing, corrupt, backend-mismatched, or stale markers change the preparation
reason, never canonical membership. Marker-save failures are reported separately
from canonical commits.

`RebuildPolicy::OnSearch` prepares lazily. `Explicit` requires calling `prepare` and
otherwise returns `IndexNotPrepared`. Policies and ANN search expansion settings belong
to the handle; the host should reapply them after reopen.

Exact CPU search evaluates the entire eligible subset, selecting AVX2 at runtime
or falling back to scalar code. Its heap holds at most k hits. Float64 accumulation
reduces numerical error, though returned float32 distances still have finite precision.
Ordering uses computed distance, then ID.

USearch performs approximate filtered HNSW with canonical live-ID eligibility.
The adapter sorts returned ties, but cannot guarantee globally smallest IDs among
equal-distance candidates that the graph did not visit. Recall must be measured for
each workload and parameter claim.

The wgpu backend supplies exact search plus deterministic IVF-Flat and IVF-SQ8 candidate
generation with exact FP32 GPU reranking: CPU-mask, eligible-row, or GPU-predicate
filtering; signed timestamps; bounded chunks and candidate readback; persistent
scratch admission; true query batches; and capability and device-loss reporting.
Required failures and automatic fallback are explicit. IVF configuration is derived
state and is rebuilt after mutations; there is no custom GPU ANN graph. Exact-GPU
state uses bounded suffix chunks and live-mask updates between consolidating rebuilds.
On a hybrid machine, the high-performance adapter
request is observable in the returned capabilities and benchmark manifest; the
2026-08-28 Windows measurements used the discrete NVIDIA GeForce RTX 4050 rather
than the integrated Intel UHD adapter. Callers should treat the reported actual
adapter as part of the performance result.

## Reports and evidence

Operation IDs provide process-local correlation, not durable transaction IDs. Reports
record the actual backend, preparation reason, lock wait, commit context, CPU path,
ANN parameters, transfer counts, and unavailable measurements. GPU completed-call
timing is host-observed. Adapters with wgpu timestamp-query support also report
isolated scoring and selection device time. Detailed
eligibility diagnostics add a scan and increase overhead.

The library installs no global subscriber. Default spans omit vectors,
credentials, raw user IDs, timestamps, and predicates. Hosts own telemetry
workers, queue bounds, exporter timeouts, and shutdown. Exporter failures may lose
observations but must not alter search results.

See [the benchmark protocol](benchmark-protocol.md) for evidence requirements and
[the verification record](verification.md) for actual commands and results.
