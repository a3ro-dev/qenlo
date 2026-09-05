# Qenlo

<p align="center"><img src="assets/brand/logo/lockup.svg" alt="Qenlo" width="360"></p>

Qenlo is a local, embedded vector store for applications that need durable records, metadata filtering, and exact cosine search without operating a separate database service.

It is best suited to small and medium collections owned by one application: desktop search, offline RAG, local document or code retrieval, and application memory. Qenlo is alpha software. It is not a distributed vector database, an embedding service, or a fastest-in-class search engine.

## Why Qenlo exists

Vector indexes are disposable; application data is not. Qenlo keeps IDs, normalized FP32 vectors, metadata, tombstones, and generation state in one canonical store. CPU, WGPU, USearch, and PyTorch structures are derived execution paths that can be rebuilt without changing which records exist.

This gives an embedded application:

- durable local collections with atomic mutation batches;
- exact search over rows selected by user and timestamp filters;
- deterministic distance-then-ID result ordering;
- optional portable-GPU, ANN, and tensor execution;
- explicit backend, fallback, preparation, and allocation diagnostics; and
- no daemon or automatic network traffic.

## Good fit

Use Qenlo when one process owns the collection, exact results are useful, metadata filters can substantially reduce the candidate set, and local persistence matters. Typical examples are desktop semantic search, offline retrieval, per-user document collections, local agent memory, and device-level search research.

Use something else when you need relational joins, multi-node replication, hosted ingestion, high write concurrency, billion-scale ANN, built-in encryption, or a production service-level agreement. See [use cases](docs/use-cases.md) and [trade-offs](docs/trade-offs.md).

## Status

Qenlo is research-grade alpha software. The repository has strong correctness, recovery, and evidence-preservation tests, but it does not establish production readiness:

- the automatic router has not been validated on held-out workloads;
- current CPU performance is not a competitive bound on optimized CPU libraries;
- WGPU availability and performance depend on the adapter, driver, and backend;
- concurrency, sustained mutation churn, crash schedules, and energy use need broader evaluation; and
- mobile packaging and current-revision physical-device validation remain incomplete.

The research paper reports observations for named hardware and source revisions. It does not claim a universal CPU/GPU threshold. Read [the paper](paper/output/pdf/qenlo-final-research-paper.pdf) or [verification notes](docs/verification.md) before quoting benchmark numbers.

## Quickstart

Qenlo uses the Rust toolchain pinned in `rust-toolchain.toml`.

```powershell
cargo run -p qenlo --example quickstart -- ./demo.qenlo cpu
```

To require WGPU on Windows:

```powershell
$env:WGPU_BACKEND = 'dx12'
cargo run -p qenlo --features gpu-wgpu --example quickstart -- ./gpu-demo.qenlo gpu
```

Required-GPU mode returns an error if initialization or execution fails; it does not silently substitute CPU execution. The example prints the adapter, graphics backend, dispatch count, and transfer sizes.

## Rust example

```rust
use qenlo::{Collection, CollectionConfig, Filter, NewRecord, TimestampRange};

async fn example() -> Result<(), qenlo::Error> {
    let config = CollectionConfig::cpu_exact(3);
    let collection = Collection::create("./notes.qenlo", config.clone()).await?;

    collection.add_batch(&[
        NewRecord { id: 1, user_id: 7, timestamp: 10, vector: vec![1.0, 0.0, 0.0] },
        NewRecord { id: 2, user_id: 7, timestamp: 20, vector: vec![0.0, 1.0, 0.0] },
    ])?;

    let filter = Filter::new(Some(7), TimestampRange::new(Some(0), Some(30)));
    let response = collection.search(&[1.0, 0.0, 0.0], &filter, 10).await?;
    assert_eq!(response.results[0].id, 1);

    collection.delete(1)?;
    collection.close()?;

    let reopened = Collection::open("./notes.qenlo", config).await?;
    assert_eq!(reopened.filter(&Filter::ALL), vec![2]);
    reopened.close()?;
    Ok(())
}
```

`Collection::new` creates an in-memory collection. Durable collections use `create` once and `open` on later starts. Public operations include atomic mixed commits, add/delete batches, exact and batch search, filtering, preparation, statistics, flush, and close.

## Execution paths

| Path | Role | Important limit |
| --- | --- | --- |
| CPU exact | Default exhaustive search | Qenlo measurements do not bound optimized CPU libraries |
| WGPU exact | Optional exhaustive acceleration | Adapter and driver support vary; fixed costs can dominate small eligible sets |
| USearch HNSW | Optional approximate search | Recall must be measured for the actual data and filters |
| PyTorch tensor | Optional Python exhaustive snapshot | Derived, not durable; CUDA and CPU measured, MPS unverified |

Automatic mode reports which route actually ran and why. Profiles are hardware-bound. Without a matching profile, the built-in threshold is a fallback policy, not a transferable performance law.

## Persistence model

Qenlo stores checksummed snapshots, a write-ahead log, and a published generation watermark. Mutations are validated before publication, and failed transactions do not partially update the canonical store. Reopen validates shape, dimension, checksum, and format version.

These mechanisms have automated recovery and corruption tests. They are not proof against every filesystem, power-loss, or concurrent crash schedule. See [recovery policy](docs/recovery-policy.md), [architecture](docs/architecture.md), and the [portable `.qn` format](docs/qn-format-v1.md).

## Inspect a collection

```powershell
# Terminal UI
cargo run -p qenlo-browser -- ./demo.qenlo

# Local Web UI at http://127.0.0.1:3456
cargo run -p qenlo-browser -- --web ./demo.qenlo --port 3456
```

See the [browser guide](docs/browser.md).

## Verify the workspace

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --all-features --no-deps
```

Benchmark commands and evidence requirements are documented in the [benchmark protocol](docs/benchmark-protocol.md). Smoke commands are not performance evidence.

## Documentation

- [Documentation home](docs/README.md)
- [Quickstart](docs/quickstart.md)
- [Concepts](docs/concepts.md)
- [Use cases](docs/use-cases.md)
- [Trade-offs](docs/trade-offs.md)
- [Feature matrix](docs/feature-matrix.md)
- [Implementation status](docs/implementation-status.md)
- [Contributing](CONTRIBUTING.md)
- [Security model](SECURITY.md)

## License

Licensed under Apache-2.0. See [LICENSE](LICENSE).
