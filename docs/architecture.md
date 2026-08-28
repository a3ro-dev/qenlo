# canonical data, disposable indexes

a derived index is allowed to be missing. canonical rows are not. that distinction
drives the storage protocol, locks, and error behavior.

## ownership and format

`CoreStore` owns normalized vectors, public IDs, metadata, permanent row slots,
tombstones, and mutation generation. user and timestamp indexes are rebuilt from
those rows. deleted IDs cannot be reused. generation increments per successful
mutation, not per batch.

`Collection` wraps state in a read/write lock. it owns the backend, preparation
state, storage options, and durable directory's OS lock. use one shared
`Arc<Collection>` rather than repeatedly opening the same directory.

the little-endian v1 header contains magic, version, dimension, generation,
total rows, and live rows. each row has ID, user ID, signed timestamp, live flag
and reserved bytes, then normalized float32 vector bits. a trailing CRC32 covers
header and rows. loading preserves normalized bytes rather than normalizing them
again. unknown versions and inconsistent shapes are explicit errors.

| file | meaning |
| --- | --- |
| `collection.lock` | handle under an exclusive OS file lock |
| `canonical-<20-digit-generation>.qdb` | published canonical snapshot |
| `canonical-<generation>.pending` | current uncommitted staging file |
| `HEAD` | 20-byte magic, acknowledged generation, and CRC32 watermark |
| `HEAD.pending` | staged replacement watermark |
| `index.qidx` | checksummed dimension/backend/readiness generation, not a graph |
| `index.pending` | staged disposable readiness marker |

valid newer legacy `.tmp` snapshots retain their recovery behavior. current
writes use `.pending` so fully written but unpublished transactions are not
mistaken for commits.

## transaction and durability order

one write lock covers the entire transaction:

1. clone canonical state and apply the ordered batch. invalid vectors, duplicate
   IDs, invalid deletions, and exhausted generation abort the staged copy.
2. check the staged store's admission budget before writing.
3. stream a complete `.pending` snapshot, append CRC32, flush, and sync the file.
4. rename to `.qdb`, then sync the collection directory where supported.
5. write and sync `HEAD.pending`, rename over `HEAD`, and sync the directory.
6. prune older snapshots while retaining the immediately previous one, publish
   staged memory, invalidate preparation, and return success.

validation and pre-publication I/O failures leave original memory unchanged.
canonical rename is publication. subsequent failure means uncertainty, not
rollback: `CommitUncertain` closes the handle and releases the OS lock. reopen
resolves the outcome. durable single-row mutations already execute this protocol;
`flush` is normally a no-op.

the retained previous snapshot helps manual investigation. it is not permission
for automatic rollback. `HEAD` is a lower bound on acknowledged generation: if
its snapshot is missing, open errors. the highest published generation is checked
in full, and corruption does not cause fallback. a valid `.qdb` newer than `HEAD`
can come from uncertain publication. reopen repairs and syncs the watermark
before returning it. valid legacy files without `HEAD` are upgraded similarly.

current `.pending` contents never become visible on reopen. interrupted initial
creation may leave no canonical snapshot at all. open then errors, while create
refuses the nonempty directory. inspect and preserve it before manually cleaning
confirmed uncommitted initialization files or using another location.

Unix syncs publication and newly created directory entries, including their
parents, subject to filesystem/hardware guarantees. Windows syncs files but does
not implement directory sync. process-crash recovery is supported; complete
sudden-power-loss durability is not claimed. CRC32 is not authentication.

## memory and write costs

the default `StorageOptions::max_load_bytes` is 512 MiB. both read and durable
write admission check snapshot size and the estimate
`rows * (32 + 4 * dimension + 512)` with checked arithmetic. the final 512 bytes
per row allow for bookkeeping. this includes tombstones, is not measured RSS,
and does not reserve memory. the host supplies options on create/open; they are
not a persisted machine policy.

loading keeps one decoded row outside the growing canonical store. directory
selection and pruning retain only the generation candidates they need. no second
full decoded snapshot is staged, but canonical rows and all metadata indexes
remain resident. allocation can still fail.

transactions pay for a full clone and full snapshot. a one-row durable mutation
is O(n) in collection size. input and normalized batch vectors may coexist with
old and staged stores. staging and retained snapshots need disk space too. the
admission estimate is not a transaction peak-memory bound. use batches, measure
the workload, and add a WAL when these costs justify its recovery complexity.

## visibility and locks

ready CPU and USearch searches hold a shared read lock for the entire query.
mutations and rebuilds take the write lock, so a query sees a complete committed
generation. search-triggered preparation drops its initial read lock, obtains a
write lock, prepares the then-current generation, and downgrades before search.
no stale graph is served during that transition.

GPU queries also take a per-collection gate so simultaneous query and selection
scratch allocations cannot multiply the budget. mutation waits for readers. there are no collection background
workers or speculative MVCC versions. `search_batch` is an ordered convenience
loop, not a multi-query snapshot.

synchronous methods block. do not block a single-thread executor on mutation
while an earlier GPU future needs that executor to finish. put contended
synchronous work on a blocking thread. async signatures do not make snapshot I/O
or CPU distance calculation nonblocking.

## preparation and backend boundaries

`index.qidx` stores only readiness metadata. no ANN graph, metadata tree, or GPU
buffer is serialized. restart always rebuilds, even when its generation matches.
missing, corrupt, backend-mismatched, and stale markers change the preparation
reason, never canonical membership. marker-save failure is reported separately
from canonical commit.

`RebuildPolicy::OnSearch` prepares lazily. `Explicit` requires `prepare` and
otherwise returns `IndexNotPrepared`. policies and ANN search expansion belong
to the handle; the host should reapply them after reopen.

exact CPU search evaluates the whole eligible subset, selecting AVX2 at runtime
or falling back to scalar. its heap holds at most k hits. float64 accumulation
reduces numerical error; returned float32 distances still have finite precision.
ordering uses computed distance then ID.

USearch performs approximate filtered HNSW with canonical live-ID eligibility.
the adapter sorts returned ties, but cannot promise globally smallest IDs among
equal-distance candidates the graph did not visit. recall must be measured for
each workload and parameter claim.

wgpu remains an exact-search experiment: CPU-mask, eligible-row, or GPU-predicate
filtering; signed timestamps; bounded chunks and candidate readback; resident and
scratch admission; capability and device-loss reporting. required failures and
automatic fallback are explicit. there is no custom GPU ANN graph or established
scale-performance advantage. On a hybrid machine, the high-performance adapter
request is observable in the returned capabilities and benchmark manifest; the
2026-08-28 Windows measurements used the discrete NVIDIA GeForce RTX 4050 rather
than the integrated Intel UHD adapter. Callers should treat the reported actual
adapter as part of the performance result.

## reports and evidence

operation IDs are process-local correlation, not durable transaction IDs. reports
name the actual backend, preparation reason, lock wait, commit context, CPU path,
ANN parameters, transfer counts, and unavailable measurements. GPU execution
timing is host-observed, not isolated kernel timestamp telemetry. detailed
eligibility diagnostics add a scan and affect overhead.

the library installs no global subscriber. default spans omit vectors,
credentials, raw user IDs, timestamps, and predicates. hosts own telemetry
workers, queue bounds, exporter timeouts, and shutdown. exporter failure may lose
observations but must not alter search results.

see [the benchmark protocol](benchmark-protocol.md) for evidence requirements and
[the verification record](verification.md) for actual commands and results.
