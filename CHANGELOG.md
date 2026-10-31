# changelog

## unreleased

### changed

- large exact CPU scans read a quarter of the bytes. each row now also keeps an
  int8 copy with one scale; `ExactScan::Auto` scores int8 first once E reaches
  4,096 rows, keeps every row whose deterministic error bound could put it in
  the top k, and rescores those in FP64. results are identical to the FP64
  reference (near-tie test included). `ExactScan::Int8` forces it on one
  thread. single-thread int8 was 0.31-0.51x of certified FP32 at E=100k on six
  cloud CPUs; on the laptop Auto P95 went 5.7 -> 1.9 ms at E=100k and
  1.0 -> 0.8 ms at E=10k. `cpu_distance_path` reports `Int8Avx2Certified`.
- rows live in 1,024-row chunks instead of one growing buffer, so `add` no
  longer copies the whole store when it grows: at 100k x 384 the add-built heap
  went 2,226 -> 2,047 B/row and the build peak 348 -> 196 MiB.
- costs: `add` is about 16% slower (2.36 -> 2.73 us, quantization), a store
  reopened from disk holds about 400 B/row more (the int8 copy), and certified
  FP32 at E=1k is about 3 us slower.

## 0.1.0-beta.2 - 2026-10-25

faster exact search on both sides of the CPU/GPU split. results are unchanged:
every mode is checked against an FP64 reference, including a near-tie test.

### changed

- the CPU scan used certified FP32 (score in FP32, rescore borderline rows in
  FP64) only up to 4,096 eligible rows and fell back to FP64 accumulation above
  that, on one thread. it now runs certified FP32 in one pass at any E, and
  `search` splits large queries across a persistent std-only thread pool (one
  thread per 2^18 eligible-row dimensions; small queries stay on one thread).
  the old route stays as `ExactScan::Legacy`. on a 12-thread laptop (N=100k,
  D=384, k=10, scattered rows, background load, so treat <20% as noise):
  E=4,097 0.74 -> 0.26 ms, E=6,000 1.15 -> 0.34 ms, E=30,000 6.6 -> 3.3 ms,
  E=100,000 15.2 -> 6.6 ms. the 4,096-row cliff is gone.
- the automatic router still sends E >= 4,096 to the GPU. with the faster CPU
  that threshold is probably low; it moves once it is re-measured.
- exact WGPU mask and row filters select top-k with a tiled kernel: each
  workgroup sorts 512 scores and keeps its k best, then one workgroup per query
  merges the lists. the old kernel made k passes over every score in one
  workgroup. results are unchanged (distance, then id). the old kernel stays as
  `GpuSelectKernel::KPass` (`Collection::set_gpu_select_kernel`,
  `qenlo-bench --gpu-select-kernel k-pass`), and the shader-predicate path
  still uses it. on an RTX 4050 laptop (unlocked clocks, N=100k, D=384, batch
  1, median of 3 processes), device selection at E=30k went from 1.86 ms to
  0.41 ms at k=64 and from 0.60 ms to 0.20 ms at k=10.
  end to end, calls were 0.60-0.97x of the old kernel's in 7 of 8 cells
  (E=1k-30k, k=10/64); E=3,000 at k=10 was 1.16x with a 0.83-1.65 block range.

### fixed

- the eligibility planner reported 0 upload bytes for shader-predicate plans;
  they upload one u32 of per-row liveness per query, so it now reports 4N.
- the `gpu_heater` example logs its duty cycle every 1,000 cycles, since the
  benchmark runner kills it before its final summary prints.

## 0.1.0-beta.1 - 2026-10-14

first beta. the storage format, the `qenlo` API, and the SDKs are unchanged from
alpha.11; this release is about doing the same work with less memory and less
waiting. all numbers are from one Windows laptop (12 threads, AVX2/FMA),
384-dimension vectors, `cargo run -p qenlo-core --release --example footprint`.

### changed

- every vector was held twice in memory: once per record and again in an
  aligned scan matrix that exact search builds. vectors now live once, in one
  32-byte-aligned buffer padded to eight floats, and search scans it directly.
  heap per row went from 3,374 to 2,034 bytes (-40%) at 2k rows.
- every write threw that scan matrix away, so the first search after an add
  re-copied the whole collection. that is the add-then-recall pattern of an
  agent memory. add-then-search at 2k rows went from 848 us to 65 us (13x).
- the FP32 distance kernels keep four independent accumulators instead of one,
  so the FMA latency chain no longer serializes the scan. user-filtered search
  at 2k rows: 23 us -> 19 us.
- the timestamp index is one ordered (timestamp, slot) set instead of a set per
  timestamp, which cost about 100 bytes per row when timestamps are unique.
- snapshots and WAL files checksum a whole vector per update instead of four
  bytes at a time. opening a 100k x 384 snapshot went from 453 ms to 165 ms;
  writing one from 708 ms to 422 ms, most of which is now fsync.
- memory admission (`StorageOptions::max_load_bytes`) budgets the one in-memory
  copy, so the default 512 MiB admits about twice as many rows as before.
- the automatic CPU/GPU router counts eligible rows without materializing them.
- npm releases are published under their channel tag (`beta` now) and `latest`.

### security

- the web browser API (`qenlo-browser --web`, desktop app) rejects requests
  whose Host or Origin is not localhost, a loopback address, or the host it was
  explicitly bound to. it can create, mutate and export collections at
  caller-chosen paths, and a DNS-rebinding page could otherwise drive it.
- rustls 0.23.46 and quinn-proto 0.11.19 (seven RustSec advisories, reached
  through the telemetry exporter), and ratatui 0.30, which drops the unsound
  lru 0.12. `cargo audit` is clean.
- the C ABI rejects slice lengths above `isize::MAX` bytes as errors instead of
  passing them to `slice::from_raw_parts`.

### breaking (Rust, `qenlo-core` only)

- `CoreStore::record`, `get`, and `records` return a borrowed `RecordRef`
  instead of `&Record`. `RecordRef::to_record()` copies a row out.
  `CoreStore::derived_scan_matrix_bytes` is replaced by `vector_bytes`. the
  `qenlo` crate API is unchanged.

## 0.1.0-alpha.11 - 2026-09-26

### fixed

- `flush()` never compacted a durable collection. It returned early whenever the
  durable generation matched the current one, and every WAL commit makes them
  match, so the snapshot-and-prune path was unreachable. Each write left one
  WAL file behind and every `open` replayed all of them. `flush()` now writes a
  snapshot and deletes the WAL files it covers whenever the newest snapshot is
  behind. `close()` still only publishes state that isn't durable yet, so it
  does not get slower. Found while building qenlo-memory, which writes one
  memory per call.
- the Python README listed Linux aarch64 and Intel macOS wheels that were never
  published, and documented `requested_backend` values that the FFI does not
  emit. Both now match the release.

## 0.1.0-alpha.10 - 2026-09-26

### added

- the web UI (`qenlo-browser --web`) and the desktop app, which embeds the same
  server, now have a Functions tab: the 33 public `Collection` methods grouped
  by task, with signature, summary, and example, filtered as you type (`/`
  focuses search). `GET /api/functions` serves the same catalog the terminal
  browser uses, so one drift test covers all three surfaces.

### changed

- the README now opens with registry install commands and short Rust, Python,
  and TypeScript examples, followed by a research table that gives each finding
  its number, its boundary, and a link to the retained evidence, failed bets
  included.
- the landing page headline numbers now come from shipped code and retained
  evidence: 4.6x light-path device-time variation across five RTX 4090 hosts,
  235.7% held-out regret for the reverted routing rule, and 1,833 hashed
  evidence files. The previous numbers came from a PyTorch CUDA prototype that
  is not shipped and that lost to FAISS in its own gate.
- the quickstart points to registry installs and the docs.rs API reference;
  it no longer says published packages are unavailable.
- `bump_alpha.py` also updates the README install commands.

### fixed

- the npm package could not load its native library on Windows. The release
  workflow packed the DLL under `native/windows-x64/`, but the loader looks in
  `native/${process.platform}-${process.arch}`, which is `win32-x64` in Node.
  Found by installing alpha.8 from npm into an empty project. The workflow now
  packs `win32-x64` and fails the release if the tarball lacks any platform's
  library. For `@a3ro.dev/qenlo` alpha.9 and earlier on Windows, set
  `QENLO_LIBRARY_PATH` to the packaged `native/windows-x64/qenlo_ffi.dll`.
- the three landing-page cards rendered about 2px wide. `.cards` is a size
  container whose cards are sized from `100cqh`, and the section added below it
  in alpha.8 squeezed its height to zero; it now has a definite flex basis.
- the landing page's dot-matrix digits only defined 0, 1, 5, 6, and 7, so any
  other digit drew as 0. All digits now render.

### compatibility and limits

- storage, query semantics, SDK APIs, and the native ABI are unchanged from
  alpha.9. The browser server adds one read-only route, `GET /api/functions`.
## 0.1.0-alpha.9 - 2026-09-26

### added

- the terminal browser's `? Functions` tab now covers all 33 public Rust
  `Collection` methods, grouped by task, each with its real signature, a
  one-line summary, and a short example. Press `/` to filter by name, group, or
  summary; `Esc` clears. A test re-parses `crates/qenlo/src/lib.rs` and fails if
  the catalog misses or invents a method, and a render test keeps the summary
  and example visible on an 80x24 terminal.
- the Python SDK emits `ResourceWarning` for a `Collection` that is garbage
  collected without `close()`, like an unclosed file, and includes any native
  close error in the message.

### fixed

- `Collection::get_record` and `Collection::scan_records` returned stale data
  after `close()` released the directory lock; they now return empty results,
  like `filter`.
- `qenlo_collection_free` ran `close()` outside the panic guard and discarded
  its error. It now uses the same guard as every other native entry point, and
  `qenlo_last_error()` reports a close or flush failure after free.
- a TypeScript `Collection` that was never closed held its native handle and
  durable directory lock for the life of the process. A `FinalizationRegistry`
  now frees it after garbage collection; `close()` remains the reliable path.
- release `SHA256SUMS` files listed a hash of themselves taken mid-write, so
  `sha256sum -c SHA256SUMS` always reported one failure. The file now covers
  only the release assets.

### compatibility and limits

- `.qn` format v1, WAL v1, and the native ABI signatures are unchanged.
- code that read records after `close()` now receives empty results.
## 0.1.0-alpha.8 - 2026-09-26

### added

- a selectable `? Functions` view in the terminal browser with ten common Rust
  `Collection` operations, concise signatures, and keyboard navigation.
- browser documentation for finding the function view and its shortcuts.
- a task-first documentation home that routes to the quickstart, browser,
  concepts, architecture, status, and each SDK guide.

### fixed

- the TypeScript SDK now rejects out-of-range `bigint` IDs, user IDs, and
  timestamps with `RangeError`. Previously `-1n` or `2n ** 63n` wrapped silently
  inside `BigUint64Array`/`BigInt64Array` and was stored as a different value.
- the `research-evidence` CI gate, red since alpha.6, passes again: the evidence
  inventory now covers the power-state archives, and `bump_alpha.py` refreshes
  it so a release cannot ship with a stale inventory.

### compatibility and limits

- the catalog is a quick reference; the Rust SDK guide remains the source for
  complete types and error behavior. Storage and query semantics are unchanged.
- `.qn` format v1, WAL v1, and the native ABI are unchanged.

## 0.1.0-alpha.7 - 2026-09-26

### added

- a research evidence index that connects the held-out router rejection and the
  GPU power-state study to protocols, raw archives, reductions, and limits.
- a sequential alpha-version bump command that updates package manifests,
  active install examples, lock entries, and generated documentation together.

### changed

- the Python install examples and lock entry now match the package version.
- the main README exposes both successful and failed research bets in a compact
  claim-to-evidence map.

### compatibility and limits

- the current branch also includes the post-alpha.6 filter, benchmark-accounting,
  and experimental GPU-path work described in its source history. No new
  automatic router or cross-host speedup guarantee is claimed.
- `.qn` format v1, WAL v1, native ABI, and SDK API contracts are unchanged.

## 0.1.0-alpha.6 - 2026-09-19

### added

- an archive-wide reanalysis that verifies four retained research archives,
  records duplicate handling, and keeps incompatible cohorts separate.
- scoped routing evidence showing that the preregistered alpha.5 scalar rule
  failed its held-out regret gate; no replacement automatic router is claimed.

### changed

- documentation now opens as a direct, content-first reference instead of a
  separate promotional landing page. Deep links retain the normal docs shell.
- Rust, Python, TypeScript, Kotlin, and documentation versions advance together.

### compatibility and limits

- `.qn` format v1, WAL v1, query semantics, native ABI, and the retained
  alpha.4 routing behavior are unchanged.

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
