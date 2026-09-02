# Qenlo

<p align="center"><img src="assets/brand/logo/lockup.svg" alt="Qenlo" width="360"></p>

a fast query means very little if the collection disappears after restart.

Qenlo is an embedded vector-search library with a small durable database core.
it stores precomputed vectors, filters by user and timestamp, and returns cosine
distances. no server, SQL layer, embedding model, or replication. persistence and
atomic mutations now exist; that does not make this a production-proven database.

canonical rows decide what exists. an ANN graph or GPU buffer can be rebuilt.
neither gets to decide whether a deleted row is still alive.

## what is implemented

| crate | responsibility |
| --- | --- |
| `qenlo-core` | rows, tombstones, metadata indexes, exact eligible-set search |
| `qenlo` | durable collections, atomic batches, shared access, reports, optional backends |
| `qenlo-bench` | independent float64 oracle, checksummed datasets, workload cells, host-owned OTLP |
| `qenlo-testkit` | cross-platform conformance/performance suite and privacy-safe report schema |
| `qenlo-mobile` | C/JNI bridge used by the Android and iOS tester shells |
| `qenlo-telemetry` | authenticated ingestion, SQLite retention, and results viewer |
| `qenlo-browser` | Claude Code-style TUI and embedded Web UI collection browser |

the exact CPU path uses runtime-detected AVX2 on supported x86 CPUs and a scalar
fallback elsewhere. it evaluates every eligible live row and retains the best k
hits. computed distance ties are ordered by ID. `k` must be in `1..=64`.
vectors must have the configured dimension, finite components, and a nonzero
norm; they are normalized on insertion. duplicate IDs are rejected, including
IDs belonging to tombstones.

filters combine optional `user_id` equality with a signed `i64` timestamp range:
lower inclusive, upper exclusive, absent bounds unbounded.

## try it now

These runnable examples create a collection, insert vectors, search with a
compound user/time filter, delete a row, rebuild, and verify the result after
reopening. Use a new directory each time; the data is kept for inspection.

```powershell
cargo run -p qenlo --example quickstart -- ./demo.qenlo cpu
$env:WGPU_BACKEND = 'dx12'
cargo run -p qenlo --features gpu-wgpu --example quickstart -- ./gpu-demo.qenlo gpu
```

The GPU example prints the actual adapter, backend, dispatch count and readback
bytes. Required GPU mode errors rather than quietly substituting the CPU.
`vulkan` is also exercised on this laptop. These tiny hand-written vectors
demonstrate behavior, not speed. [How the GPU kernels work](docs/gpu-design.md)
separates our scoring and selection code from the wgpu device layer and the
external USearch CPU ANN adapter.

On this hybrid host, wgpu sees Intel UHD Graphics (integrated) and an NVIDIA
GeForce RTX 4050 Laptop GPU (discrete). The measured GPU runs requested the high
performance adapter and record `NVIDIA GeForce RTX 4050 Laptop GPU`,
`DiscreteGpu`, and DX12 in their manifests. Always inspect the reported adapter
when reproducing a result on a machine with more than one GPU.

## inspect with qenloDB browser

Just like *DB Browser for SQLite* allows developers to inspect `.sqlite` files, **QenloDB Browser** brings radical transparency to embedded vector search: inspect stored unit vectors, observe tombstones and uncompacted WAL segments, run live cosine queries, and view SIMD execution timings without black-box servers.

```powershell
# Claude Code-style interactive Terminal UI (TUI)
cargo run -p qenlo-browser -- ./demo.qenlo

# Zero-bloat local Web UI (http://127.0.0.1:3456)
cargo run -p qenlo-browser -- --web ./demo.qenlo --port 3456

# Native Desktop App (Tauri v2)
cd apps/desktop && pnpm install && cargo run -p qenlo-browser-desktop
```

See the [QenloDB Browser Guide](docs/browser.md) for full keyboard shortcuts and REST API docs.

## a small durable example

use Rust 1.98.0 from [rust-toolchain.toml](rust-toolchain.toml). this example belongs
inside an application's async startup; Qenlo does not install a runtime. `create`
needs a new or empty directory. use `open` on subsequent starts.

```rust
use qenlo::{Collection, CollectionConfig, Filter, NewRecord, TimestampRange};

async fn example() -> Result<(), qenlo::Error> {
    let config = CollectionConfig::cpu_exact(3);
    let collection = Collection::create("./notes.qenlo", config.clone()).await?;
    collection.add_batch(&[
        NewRecord {
            id: 1, user_id: 7, timestamp: 10,
            vector: vec![1.0, 0.0, 0.0],
        },
        NewRecord {
            id: 2, user_id: 7, timestamp: 20,
            vector: vec![0.0, 1.0, 0.0],
        },
    ])?;
    let filter = Filter::new(Some(7), TimestampRange::new(Some(0), Some(30)));
    let response = collection.search(&[1.0, 0.0, 0.0], &filter, 10).await?;
    assert_eq!(response.results[0].id, 1);
    collection.delete(1)?;
    collection.flush()?; // durable mutations are already synced
    collection.close()?;

    let reopened = Collection::open("./notes.qenlo", config).await?;
    assert_eq!(reopened.filter(&Filter::ALL), vec![2]);
    reopened.close()?;
    Ok(())
}
```

`Collection::new` remains the in-memory option. public operations include `add`,
`add_batch`, `delete`, `delete_batch`, mixed `commit(&[Mutation])`, `search`,
`search_batch`, `filter`, `prepare`, `stats`, `flush`, and `close`. a search sees
one committed generation. `search_batch` executes one native GPU batch when GPU routing is selected,
and all responses observe the same committed generation. `close` is explicit and idempotent. for compatibility, `filter`
returns an empty list after close; searches and mutations return `Error::Closed`.

## persistence and recovery

format v1 stores normalized vectors, IDs, user IDs, signed timestamps, tombstones,
and canonical generation in a checksummed snapshot. writes stage a `.pending`
file, sync it, rename it to `.qdb`, then publish a checksummed `HEAD` watermark
before acknowledging success. a missing acknowledged snapshot is an error,
not permission to silently open an older collection.

reopen validates the newest published snapshot. checksum, shape, dimension, and
format-version errors are explicit. current `.pending` files are never promoted.
valid legacy `.tmp` snapshots can be recovered; legacy files without `HEAD` are
accepted and given a watermark after validation.

before publication, a failed transaction leaves canonical memory unchanged.
after publication, a durability-confirmation failure returns
`Error::CommitUncertain` and closes the handle. reopen to resolve the outcome;
do not assume rollback. interrupted initial creation can leave only staging
files: `open` reports no committed snapshot and `create` refuses the nonempty
directory. inspect and preserve it before manually clearing confirmed uncommitted
initialization files or choosing a new location.

files are synced on the native paths. Unix also syncs directory entries, including
newly created parents. Windows currently has no directory-sync implementation
here, so full power-loss durability is not guaranteed on Windows. process-crash
tests do not prove sudden-power-loss safety. filesystem sync behavior still
matters on Unix.

the default load admission budget is 512 MiB. the same check rejects durable
writes that would be impossible to reopen under that budget. it estimates payload
plus per-row bookkeeping, not allocator usage or RSS. larger collections require
an explicit choice:

```rust
use qenlo::{Collection, CollectionConfig, StorageOptions};

async fn open_larger_collection() -> Result<Collection, qenlo::Error> {
    Collection::open_with_options(
        "./large.qenlo",
        CollectionConfig::cpu_exact(768),
        StorageOptions { max_load_bytes: 8 * 1024 * 1024 * 1024 },
    ).await
}
```

`create_with_options` accepts the same options. keep them in host configuration.
loading decodes one row at a time, but the full canonical store and metadata
indexes remain resident. canonical snapshots are decoded from validated read-only
memory maps. durable transactions validate O(batch) state and append checksummed
immutable WAL files under an atomic manifest; `flush`/`close` compact them into a
full canonical snapshot. batch input and normalized vectors coexist during validation.
compaction and restart replay remain synchronous.

## concurrency and derived indexes

share one `Arc<Collection>` across threads. ready CPU and USearch searches share
a read lock; mutations and rebuilds take a write lock. GPU queries serialize access
to a persistent, grow-on-demand scratch arena. one OS lock permits only one cooperating
handle or process to open a durable directory, including for reads.

mutations, `filter`, `stats`, `flush`, and `close` block. do not call them from a
single-thread async executor while an outstanding GPU future needs that executor
to release a lock. use a blocking thread for contended synchronous work. async
open and search can also perform synchronous I/O or CPU work; there are no hidden
collection workers.

`RebuildPolicy::OnSearch` is the default. `RebuildPolicy::Explicit` returns
`IndexNotPrepared` until `prepare().await` succeeds. mutations invalidate the
prepared generation. `index.qidx` persists only readiness metadata, not a USearch
graph or GPU buffers. restart always rebuilds. missing, corrupt, or stale derived
metadata cannot remove canonical rows.

## backends and diagnostics

`qenlo` has no default features. `usearch` enables the C++ HNSW adapter;
`gpu-wgpu` enables portable GPU exact, IVF-Flat, and IVF-SQ8 search. the benchmark crate forwards both and
separately provides `otlp` for its host-owned exporter example.

USearch applies canonical eligibility during graph traversal. initial parameters
are connectivity 16, insertion expansion 128, and search expansion 128.
`set_ann_search_expansion` changes the last value for the handle and subsequent
rebuilds. results remain approximate, even though returned ties are sorted.

GPU modes are CPU mask, CPU eligible rows, and GPU predicate. allocation admission
includes scratch, execution is chunked, and candidate readback is bounded.
required mode reports failures; automatic mode routes selective searches below
the initial 4,096-row crossover to CPU and reports routing/fallback reasons.
true batches contain up to 128 queries in one GPU workload. `set_gpu_ivf` enables
deterministic IVF-Flat candidate generation; `set_gpu_ivf_sq8` adds an SQ8 coarse
stage. both retain canonical FP32 vectors and perform an exact FP32 GPU rerank.
capability reporting and device-loss handling exist. kernel
timestamps are not measured; host-observed execution durations are not isolated
kernel timings.

reports include operation IDs, actual backend, generation, preparation reason,
lock wait, CPU path, ANN parameters, transfer counts, and commit context. missing
measurements carry reasons. `Diagnostics::Disabled` suppresses operation spans
while retaining reports; `Detailed` adds an eligibility-count scan. the library
installs no global tracing subscriber and excludes vectors, credentials, raw
user IDs, timestamps, and predicates from default spans. hosts own exporters,
bounded queues, shutdown, and any additional fields.

## run it, then make claims

```powershell
cargo test --workspace --no-default-features
cargo run -p qenlo-bench -- prepare --dataset smoke.qds --rows 256 --dimensions 16
cargo run -p qenlo-bench -- run --dataset smoke.qds --output smoke-results --dimensions 16 --backend cpu
```

use fresh dataset/output paths. the [benchmark protocol](docs/benchmark-protocol.md)
defines the 100k/1m, 384/768-dimensional workload matrix, disjoint query sets,
selectivity, recall gates, raw samples, and nearest-rank percentiles. these
commands are smoke workloads, not evidence of scale performance.

one deterministic correctness fixture measured recall@10 of 1.0 for exact CPU
and USearch across nine filters, 16 queries each, and 2,048 vectors at 32
dimensions, including empty and fewer-than-k cases. this does not establish
0.95 or 0.99 recall on arbitrary embeddings or the larger matrix. The retained
[2026-08-28 real-data results](docs/results-2026-08-28.md) include a 100k × 384
all-row cell where the RTX 4050 exact GPU predicate path is 5.14× lower P95 than
Qenlo exact CPU, plus a selective 1% cell where the GPU is slower. The
predeclared 1m × 768 gate remains untested.

the Windows setup previously hit an MSVC 14.29 compiler crash in USearch's native
dependency. the local workaround is explicit rather than forced on consumers:

```powershell
$env:CC = 'clang-cl'
$env:CXX = 'clang-cl'
cargo test -p qenlo --features usearch
```

see [contributing](CONTRIBUTING.md) for full commands,
[the device lab](docs/device-lab.md) for the Linux, Windows, macOS, Android,
iOS, and telemetry packages, [roadmap status](docs/implementation-status.md) for
implemented versus research-only milestones, and [verification](docs/verification.md)
for recorded results and platform limits.
[Measured 2026-08-28 results](docs/results-2026-08-28.md) contains the real-data
CPU, RTX 4050 GPU, USearch, and native Chroma cells with their recall gates and
limits.
[Native Chroma replay](scripts/chroma-replay.md) uses identical prepared vectors,
synthetic metadata, oracle results and query order, with recall validation.
[architecture](docs/architecture.md) explains the storage/index boundary;
[security](SECURITY.md) describes the local trust model.

## anonymous telemetry policy

Qenlo collects anonymous installation, execution, and hardware environment telemetry (operating system, CPU architecture, SDK version, and search duration metrics) transmitted securely to `https://api.gobitsnbytes.org/qenlo/telemetry`. This telemetry is strictly anonymous, privacy-preserving, and mandatory across all SDK installations and language bindings (there is no opt-out) in order to monitor release stability, diagnose GPU driver regressions, and optimize embedded vector routing algorithms.

## license

the workspace declares MIT or Apache-2.0, at your option. the included MIT text
is in [LICENSE-MIT](LICENSE-MIT).
