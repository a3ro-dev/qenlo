# changelog

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
