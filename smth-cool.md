I'll start by understanding what Qenlo actually is from the code.

I've read the code, docs, benchmarks, SDKs, and the paper. Here's the review.

---

# Qenlo: senior-engineer review

## 1. What Qenlo actually is today

Stripping away the surrounding apparatus, the shipped engine is ~8,400 lines across `qenlo-core` + `qenlo` (the other 7,000 lines are bench/testkit/browser/telemetry/FFI).

**The data model** ([qenlo-core/src/lib.rs:39](crates/qenlo-core/src/lib.rs:39), [:57](crates/qenlo-core/src/lib.rs:57)) is fixed and closed:

```
Record  = { id: u64, user_id: u64, timestamp: i64, vector: Vec<f32> (unit-normalized), live: bool }
Predicate = { user_id: Option<u64> AND timestamp: [lower, upper) }
```

That is the entire schema and the entire query language. Consequences that define the product more than anything in the README:

- **No payload.** You cannot store the text, the chunk, the URL, the doc ID. Every user must run a second store keyed by `u64` and keep it in sync transactionally with Qenlo — which has no shared transaction with anything.
- **No upsert, no ID reuse.** [`ids`](crates/qenlo-core/src/lib.rs:453) retains tombstoned IDs, so `delete(7); add(7, ...)` returns `DuplicateId`. Re-embedding a changed document requires minting a new ID and rewriting your side table.
- **No vacuum, ever.** Tombstones persist in the snapshot format by design ("IDs never reused"), and nothing anywhere reclaims a dead slot. A collection under churn grows monotonically in RAM and on disk for its whole lifetime.
- **Cosine only**, `k ∈ 1..=64`, single equality partition, single time range, AND only. No OR, no IN, no arbitrary tags, no L2, no dot-product/MIPS.

**The execution model:** everything resident in RAM; single-threaded scalar/AVX2 scan ([no rayon, no threads anywhere](crates/qenlo-core/src/lib.rs:738)); one exclusive OS lock per directory — one process, *including readers*; and any mutation invalidates the derived index so the next search rebuilds it **completely** under a write lock ([lib.rs:1503](crates/qenlo/src/lib.rs:1503)). For USearch that means [constructing a brand-new HNSW graph from zero](crates/qenlo/src/usearch_backend.rs:44); for GPU it means re-uploading every vector.

**The durability core is the best part of the codebase.** Staged `.pending` → fsync → rename → checksummed `HEAD` watermark, WAL with manifest, fail-closed on checksum/version/shape, explicit `CommitUncertain` instead of a fake rollback, refusal to silently open an older snapshot. This is genuinely careful work, better than most embedded stores, and the honest caveats (Windows directory sync, CRC≠MAC, process-crash ≠ power-loss) are stated in the right places.

**The measured performance** (your own numbers, [docs/results-2026-08-28.md](docs/results-2026-08-28.md), [strict gate](benchmark-results/2026-09-02-runpod-a6000-strict-research-gate/README.md)):

| workload | Qenlo CPU | Qenlo GPU | USearch | Chroma | NumPy | FAISS GPU |
|---|---|---|---|---|---|---|
| 100k×384, all rows | 16.66 ms | 3.24 ms | 3.62 ms | **1.13 ms** | — | — |
| 100k×384, 1% eligible | **0.23 ms** | 2.88 ms | — | — | — | — |
| 1M×768, 1% eligible | — | 0.171 ms* | — | — | 0.197 ms | **0.132 ms** |

\* PyTorch prototype, not the shipped backend.

## 2. The single most important structural fact

Read that table again in the direction nobody in the repo has written down:

**Qenlo's best result by a wide margin is the selective cell (0.23 ms), and it is beaten by an off-the-shelf library in every non-selective cell.** The GPU is 12× *slower* than the CPU at 1% selectivity. Chroma — the "developer ergonomics, local prototyping" system in your own comparison matrix — is 3× faster than your best GPU path at 100k. And at the 1M scale you preregistered as the investment gate, plain NumPy is 0.197 ms.

So:

> **Qenlo is a per-partition exact retriever. It wins when every query is scoped to a tenant, user, session, or time window containing thousands of vectors — not millions.**

Everything downstream follows from this, and most of it contradicts the current roadmap. The entire GPU/routing/eligibility-compilation research program optimizes the regime (large E) where Qenlo is structurally *weakest* and where FAISS, cuVS, and a BLAS call are all better. The routing-regret result — 17.4% at E=3,000 — is 17.4% of a sub-millisecond query. It is ~70 µs. Meanwhile the Python SDK serializes every search result to JSON and parses it back ([qenlo-ffi/src/lib.rs](crates/qenlo-ffi/src/lib.rs), [sdk/python/src/qenlo/__init__.py:280](sdk/python/src/qenlo/__init__.py:280)), which is almost certainly more than 70 µs and has never been measured.

The paper is intellectually honest about its scope. The *product* has quietly inherited the paper's priorities, and they are the wrong priorities.

## 3. Where Qenlo genuinely fits

**Strong fit — real structural advantage:**

1. **Multi-tenant per-user retrieval inside an application process.** `users: BTreeMap<u64, BTreeSet<u32>>` gives eligibility in O(rows for that tenant), then an exact scan. No recall cliff, no cross-tenant traversal, deterministic ordering. HNSW systems degrade badly here and everyone knows it.
2. **Session- or window-scoped agent/episodic memory.** The timestamp BTree is exactly the right index for "the last 6 hours of this agent's task." E stays in the hundreds-to-thousands. Exact is not just adequate, it's cheaper than maintaining a graph.
3. **A sealed, shipped corpus artifact.** `.qn` is checksummed, fixed-layout, version-checked, byte-identical across platforms. Dropping a validated 50k-vector index into an app bundle or container image is a real need that SQLite-VSS and Chroma serve badly.
4. **A correctness oracle / differential-testing rig for other vector systems.** `qenlo-bench` with its independent FP64 oracle, checksummed datasets, disjoint tuning/eval splits, retained failures, and replay harness is genuinely better than what most vector DBs ship. Nobody else gives you this.
5. **Auditable deletion.** Canonical tombstone + fail-closed eligibility revalidation ([`InvalidEligibleRow`](crates/qenlo-core/src/lib.rs:770)) means a stale derived index cannot resurrect a deleted row — it errors instead. That's a defensible compliance property, and most competitors' soft-delete bitsets are not.

**Weak fit — use something else:**

| If you need | Use |
|---|---|
| Vectors alongside relational data, joins, real transactions | **pgvector** |
| Analytics over embeddings, columnar scans, joins to Parquet | **DuckDB** |
| Local multimodal store with payloads, versioning, S3-backed | **LanceDB** |
| Fastest local ANN over a static corpus | **USearch / hnswlib / FAISS** — Qenlo just wraps USearch and makes it slower |
| Rich metadata filters, hybrid search, a server | **Qdrant** |
| Prototyping with documents + metadata, batteries included | **Chroma** — and it beat you 3× on your own benchmark |
| ≤10k vectors, one process, occasional queries | **A `numpy` array and a pickle.** Say this out loud; it is true and it builds credibility. |

## 4. What to build

### 4.1 Parallel + blocked CPU scan
1. **Problem.** 16.66 ms for 100k×384 is one core doing 38M FLOPs. The machine has 12.
2. **Structural advantage.** The eligible set is already a sorted slot list and vectors already live in a 64-byte-aligned row-major matrix ([`AlignedScanMatrix`](crates/qenlo-core/src/lib.rs:261)). The hard part is done.
3. **Missing.** Rayon over row chunks with per-chunk top-k merge; query-block × row-block tiling in `search_batch` so a batch of 128 is one blocked GEMM instead of [128 independent scans](crates/qenlo/src/lib.rs:1657).
4. **Smallest implementation.** `par_chunks` over the eligible slice, thread-local `BinaryHeap<RankedHit>` of size k, merge. ~80 lines in `qenlo-core`, behind an optional `parallel` feature so `qenlo-core` keeps its zero-dependency property.
5. **Failure mode.** At E=1,000 thread spawn overhead exceeds the work — needs a size threshold, which is a *measured* constant, not another router. And it will likely show the GPU backend has no remaining justification, which is politically inconvenient but correct.

### 4.2 Opaque payload column + `replace()`
1. **Problem.** Nobody can use Qenlo without a second database. That's not a feature gap, it's the adoption blocker.
2. **Structural advantage.** The WAL/snapshot protocol is already atomic and checksummed. A variable-length blob per row rides along inside the *same* transaction, which is precisely what a side SQLite table cannot give you.
3. **Missing.** Variable-length records in format v2; `replace(id, vector, payload)` that tombstones + re-adds atomically while keeping the public ID stable; a real compaction that reclaims tombstoned slots.
4. **Smallest implementation.** Format v2 = v1 rows + `payload_len: u32` + bytes; `get_record` returns `&[u8]`; `replace` as a WAL primitive. Keep v1 readable. Fix `ids` to permit reuse only via `replace`.
5. **Failure mode.** Payloads inflate the resident set (everything is in RAM) and the load budget. Cap payload size hard (say 4 KiB), or store payloads in a separate segment that isn't mmapped into the scan path.

### 4.3 Read-only, multi-process open of a sealed collection
1. **Problem.** [One exclusive OS lock, even for reads](crates/qenlo/src/storage.rs:113). One process. Your CLI browser cannot inspect a collection your app has open. A web server cannot fan out readers.
2. **Structural advantage.** A published snapshot is immutable and already mmapped and validated. Concurrent readers over an immutable file need no coordination at all.
3. **Missing.** `Collection::open_readonly(path)` taking a shared lock, refusing all mutations, and mapping the newest `HEAD` generation. Plus a zero-copy scan that reads `f32` directly from the map instead of decoding each row into a fresh `Vec<f32>`.
4. **Smallest implementation.** Shared `flock`/`LockFileEx` shared mode + a `ReadOnly` state variant. The mmap path already exists.
5. **Failure mode.** A writer compacting and pruning snapshots out from under a reader. Reference-count generations, or simply never prune a generation while any `HEAD`-referenced map is live.

### 4.4 Remove the mandatory telemetry
1. **Problem.** [`sdk/python/src/qenlo/__init__.py:60`](sdk/python/src/qenlo/__init__.py:60) fires a network request to `api.gobitsnbytes.org` **at module import**, with the README stating "there is no opt-out." This is disqualifying for the healthcare/financial/air-gapped users your own docs court, it is a GDPR problem regardless of the word "anonymous," and it flatly contradicts [docs/trade-offs.md](docs/trade-offs.md) ("Air-Gapped Ready: Requires zero internet connection or telemetry to operate").
2. **Structural advantage.** N/A — this is subtraction.
3. **Missing.** Deletion.
4. **Smallest implementation.** Delete `_send_telemetry` and its siblings in every SDK; make telemetry opt-*in* via an explicit `qenlo.enable_telemetry()` call or an env var that defaults off; update the README section.
5. **Failure mode.** You lose install analytics. At alpha with no users, you are losing nothing and buying back your entire trust position.

### 4.5 Reconcile the documentation with the code
1. **Problem.** Three docs describe a different product than the one in the repo. Details in §7. The quickstart's first Python example raises `TypeError`.
2. **Structural advantage.** The README, `architecture.md`, `implementation-status.md`, and `claim-artifact-ledger.md` are *excellent* and are the reason anyone would trust this project. The marketing layer actively destroys that.
3. **Missing.** Deletion of `docs/use-cases.md`, `docs/trade-offs.md`, `docs/feature-matrix.md` in their current form.
4. **Smallest implementation.** Delete all three. Replace with one page: "Qenlo is a per-partition exact retriever" + the honest fit/no-fit table + the measured numbers including the ones where you lose.
5. **Failure mode.** The page looks less impressive than competitors'. It will convert better with the engineers who'd actually adopt an alpha embedded database.

## 5. What NOT to build

- **CUDA / HIP / native Metal / direct Vulkan kernels.** Your own strict gate: the CUDA prototype is 1.29× slower than FAISS and 1.16× faster than a NumPy loop — below your preregistered 2× gate. You wrote the go/no-go and it said no. Honor it.
- **NPU / ANE / QNN stages.** Vendor SDKs, physical devices, signing, and a research gate you have explicitly not executed. This is a multi-quarter detour for a system whose queries are already sub-millisecond.
- **A custom GPU ANN graph, IVF-PQ, RaBitQ, FP16.** In the small-E regime that Qenlo actually wins, approximate indexes are strictly worse than the exact scan they'd replace.
- **A live single-file `.qn` write container.** [`qn-format-v1.md`](docs/qn-format-v1.md) correctly enumerates what this needs — dual superblocks, torn-write handling, locking, recovery. That's a rewrite of your best-tested subsystem to win a cosmetic comparison with SQLite. The directory format works.
- **An adaptive / learned / calibrated router.** You are proposing to learn a policy that saves tens of microseconds, over a decision surface that changes with every driver update, using a profile keyed on adapter name string. Delete `RouterProfile` from the public API.
- **Replication, multi-GPU, encryption-at-rest.** Correctly marked "not implemented"; keep them there.
- **The Tauri desktop app, the Android/iOS tester shells, and `qenlo-telemetry`** as *product* surface. As a research device lab they were justified. As things a user of an alpha library needs, they are not, and they're now ~1,000 lines of maintenance plus a Node/pnpm/Gradle/Xcode toolchain in a Rust repo.

## 6. Engineering for engineering's sake

**Real user value:** the storage protocol; the canonical/derived boundary and fail-closed eligibility; the FP64 oracle and `cpu_quality` differential test; the `.qn` format spec; `architecture.md`; the claim-artifact ledger.

**Engineering for its own sake:**

- **`ExecutionReport` has 35 fields**, returned on *every* search. `eligibility_transfer_bytes`, `eligible_contiguous_runs`, `eligibility_cacheable`, `eligibility_resident`, `predicate_traversals`, `gpu_row_preparation`, `row_cache_hit` exist to serve the paper, not a caller. Worse, they cost on the hot path: [`PhaseTimings::cpu`](crates/qenlo/src/lib.rs:255) heap-allocates five `String`s per CPU search to record *reasons things weren't measured*. On a 0.23 ms query that's pure diagnostic tax. Split this: a 6-field `SearchReport` by default, the full struct behind `Diagnostics::Detailed`.
- **`GpuRowPreparation::LegacyTwoPass`** is a deliberately retained slower path, shipped in the public API, for one ablation in one paper. That belongs in the bench crate.
- **Three `GpuFilterMode` variants** are three implementations of the same semantics kept alive for a plot.
- **`qenlo-telemetry`** — a full authenticated SQLite ingestion server with retention and a dashboard, for a library with no users.
- **`RouterProfile`** keyed on `adapter_name: String` — a per-device tuning artifact with no persistence, no calibration, and, per your own ledger, no held-out evaluation.

## 7. Claims not supported by the evidence

These are concrete and checkable:

| Claim | Where | Reality |
|---|---|---|
| `flags_all_set` / `flags_any_set` / `flags_none_set` bitmask filtering, shown in three code samples | use-cases.md, trade-offs.md, feature-matrix.md | **Does not exist.** Zero occurrences in the codebase. `Predicate` has `user_id` + timestamp range only. |
| "Your entire vector database, WAL log, and metadata live in a single `.qn` file" | trade-offs.md | [qn-format-v1.md](docs/qn-format-v1.md): "a portable snapshot, not the live write-ahead-log container." Live DB is a directory. `import_qn` returns an **in-memory** collection. |
| "Memory-mapped indexes open in `< 5ms` with zero index warm-up latency" | use-cases.md | README: "restart always rebuilds." Load decodes row-by-row into resident memory. |
| "configurable vector quantization (f32, fp16, int8)" | use-cases.md | FP16 is "not implemented" in implementation-status.md. SQ8 is GPU-IVF-internal, not a storage option. |
| "Air-Gapped Ready: Requires zero internet connection or telemetry" | trade-offs.md | README + every SDK: mandatory telemetry, no opt-out, fired at import. |
| "Portable WGPU + **CUDA kernels**" | feature-matrix.md, trade-offs.md | Not implemented. Paper: "exploratory PyTorch code, not a shipped Qenlo backend." |
| "**100% Deterministic Recall**" | feature-matrix.md, trade-offs.md | Measured evaluation recall 0.99998; paper explicitly avoids "exact recall" as a numerical claim. |
| Chroma correctness authority: "Basic unit test assertions" | feature-matrix.md | Unsupported disparagement of a system that beat you 3× in your own benchmark. |
| "Memory footprint ~10-50MB base; zero-copy mmap decoded row slices" | feature-matrix.md | Not zero-copy. Each row is decoded into a `Vec<f32>`, **and** [`AlignedScanMatrix`](crates/qenlo-core/src/lib.rs:261) makes a second padded copy of every row including tombstones. Resident vector memory is ≥2× payload. |
| `qenlo.open("user_notes.qn", dim=384)`; `Collection.memory(dim=3)`; `db.search(..., top_k=5)`; `match.score` | use-cases.md, quickstart.md | Real signatures are `memory(dimension=)`, `search(..., k=)`, `SearchResult.distance`. **The quickstart's first example raises `TypeError`.** |
| `pnpm add @qenlo/qenlo` | quickstart.md | Published name is `@a3ro.dev/qenlo`. |
| `pub struct Filter { user_id, timestamp_min, timestamp_max }` | concepts.md | Real type is `Predicate { user_id, timestamp: TimestampRange { lower, upper } }`. |

Two more that are code, not docs:

- **The 5.14× GPU-over-CPU headline compares a GPU against a single-threaded CPU baseline.** With 12 cores the CPU cell would plausibly land at 1.5–3 ms — at or below the GPU. The comparison isn't wrong, but the framing implies a hardware conclusion that a `par_chunks` call could erase. This should be stated in `results-2026-08-28.md`.
- **The load admission budget under-counts by ~6× at D=768.** [`check_admission`](crates/qenlo/src/storage.rs:731) allows `32 + 4D + 512` bytes/row, but the scan matrix alone adds `ceil(D/16)*16*4` = 3,072 bytes/row at D=768. It's allocated lazily on first search, so `open()` succeeds and the *first query* OOMs — for 1M rows that's ~2.5 GB unaccounted. A real, reachable failure mode.

## 8. Operational and adoption blockers

1. **No vacuum.** Tombstones are never reclaimed. Combined with no upsert, any application that re-embeds changed content grows without bound in RAM and on disk, permanently.
2. **Full index rebuild on every mutation.** Insert one row into a 100k collection → the next query rebuilds the entire HNSW graph or re-uploads every vector to the GPU, under a write lock. The ANN and GPU backends are unusable for anything but a static corpus.
3. **Single process, including readers.** Blocks the browser, blocks server fan-out, blocks sidecar inspection.
4. **JSON per search across the FFI**, plus [a Python-level float-by-float copy loop in `add_batch`](sdk/python/src/qenlo/__init__.py:218) — 38.4M interpreter-level assignments to ingest 100k×384. No numpy path. Every non-Rust user hits this before they hit any of your fast paths.
5. **`eligible_ids` builds a `HashSet<u64>` of the entire eligible set on every USearch query** ([lib.rs:2045](crates/qenlo/src/lib.rs:2045)) — 100k allocations per query for `Filter::ALL`.
6. **CI never builds `usearch` or `gpu-wgpu`.** [ci.yml](.github/workflows/ci.yml) runs `cargo test --workspace` with default features = none. The 2,036-line `gpu.rs` and the USearch adapter have zero automated coverage.
7. **The browser's web server has no auth, no CSRF token, and no `Host` validation** ([server.rs:78](crates/qenlo-browser/src/server.rs:78)), exposing `/api/open`, `/api/mutate`, and `/api/export` with arbitrary filesystem paths. Same-origin policy blocks the naive cross-site attack, but DNS rebinding is not blocked, and `--host 0.0.0.0` is a supported flag. Add a Host/Origin allowlist and a startup token.
8. **~85 tests, no fuzzing, no property-based testing beyond one randomized loop, no concurrency stress test, no crash-injection matrix** — for a system whose central claim is durability.

---

# Qenlo reality check

### Strongest use cases
1. **Per-tenant / per-user exact retrieval in-process** — small E, no recall cliff, no cross-tenant traversal. The one place the architecture is genuinely right.
2. **Session- and window-scoped agent memory** — the timestamp index is the correct structure and E stays small.
3. **Sealed, checksummed, shippable corpus artifacts** (`.qn` in an app bundle, mobile asset, or container image).
4. **A correctness oracle and differential-testing harness for other vector systems** — `qenlo-bench` is better than what the incumbents ship and is currently invisible.
5. **Auditable deletion** where "a deleted row must provably never be returned" is a requirement.

### Build these
1. **Parallel + blocked CPU scan** (rayon + query-block tiling). Largest available win; likely obsoletes the GPU backend, which is the point.
2. **Payload column + `replace()` + real tombstone compaction.** Removes the two-database problem and the unbounded-growth problem in one format revision.
3. **Read-only multi-process open** with a shared lock and true zero-copy mmap scan.
4. **Delete the mandatory telemetry** from every SDK; make it opt-in, default off.
5. **Delete `use-cases.md`, `trade-offs.md`, `feature-matrix.md`; fix `quickstart.md` so the first example runs.**

### Stop / out of scope
1. CUDA, HIP, native Metal, direct Vulkan — your own preregistered 2× gate said no.
2. NPU/ANE/QNN stages.
3. Adaptive or calibrated routing; delete `RouterProfile` and `GpuRowPreparation::LegacyTwoPass` from the public API.
4. Live single-file `.qn` write container.
5. Tauri desktop app, mobile tester shells, and `qenlo-telemetry` as product surface — move them to a `research/` archive or a separate repo.

### Biggest risks
- **Product risk:** the fabricated marketing docs. This repo's asset is its credibility, and three pages currently invent an API that does not exist, promise "zero telemetry" next to mandatory telemetry, and disparage a competitor that beat it 3× in its own benchmark. One skeptical reader finds this in ten minutes and the honest 90% of the repo dies with it.
- **Technical risk:** the strategy optimizes the wrong regime. Every hardware investment targets large E, where FAISS, cuVS, USearch, and a BLAS call all win. The small-E regime where Qenlo is genuinely differentiated has received no optimization work at all — it isn't even multithreaded.
- **Adoption risk:** no payload, no upsert, no vacuum, single-process, JSON FFI, telemetry you cannot disable. Any one of these ends an evaluation in the first hour.
- **Correctness risk:** the two most complex subsystems have zero CI coverage, and the memory admission budget under-counts peak usage by ~6× at production dimensions.
- **Sustainability risk:** eight crates, five SDKs, six CI workflows, a Tauri app, two mobile shells, and a telemetry service — for 8,400 lines of actual engine, maintained by one person.

### Proposed near-term direction

**Reposition Qenlo as "SQLite for per-partition exact vector retrieval" and cut everything that isn't that.**

A concrete next milestone, roughly in order:

1. **Week 1 — integrity.** Delete the three fabricated docs. Remove mandatory telemetry. Make the quickstart run. Publish one honest positioning page that includes the cells where Qenlo loses. This costs nothing and is the highest-value change available.
2. **Weeks 2–4 — the regime you actually win.** Parallel + blocked CPU scan. Trim `ExecutionReport` to six default fields and stop allocating strings per query. Re-run the 100k×384 all-rows cell and publish the multithreaded CPU number honestly, including if it removes the GPU's advantage.
3. **Weeks 5–8 — format v2.** Payload column, `replace()`, tombstone-reclaiming compaction, read-only shared open. This is the difference between a benchmark and something a person can build on.
4. **Ongoing — freeze the accelerators.** Move `gpu-wgpu`, `usearch`, the device lab, mobile shells, and the telemetry service behind a `research` designation. Keep them compiling; add them to CI; stop investing in them.
5. **Publish the paper as-is.** The research is honest, well-audited, and worth having. Just stop letting it set the product roadmap — it answered a question about GPU routing, and the answer was "the crossover is device-specific and the stakes are microseconds." That's a legitimate negative result. Ship it, then go build the database.