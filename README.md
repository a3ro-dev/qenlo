# Qenlo

<p align="center"><img src="assets/brand/logo/lockup.svg" alt="Qenlo" width="360"></p>

<p align="center">
  <a href="https://github.com/a3ro-dev/qenlo/actions/workflows/ci.yml"><img src="https://github.com/a3ro-dev/qenlo/actions/workflows/ci.yml/badge.svg?branch=main" alt="Portable correctness"></a>
  <a href="https://github.com/a3ro-dev/qenlo/actions/workflows/sdk-ci.yml"><img src="https://github.com/a3ro-dev/qenlo/actions/workflows/sdk-ci.yml/badge.svg?branch=main" alt="SDK conformance"></a>
  <a href="https://github.com/a3ro-dev/qenlo/actions/workflows/browser-release.yml"><img src="https://github.com/a3ro-dev/qenlo/actions/workflows/browser-release.yml/badge.svg?branch=main" alt="Browser and desktop CI"></a>
  <a href="https://github.com/a3ro-dev/qenlo/actions/workflows/device-lab.yml"><img src="https://github.com/a3ro-dev/qenlo/actions/workflows/device-lab.yml/badge.svg" alt="Device lab packages"></a>
  <a href="https://github.com/a3ro-dev/qenlo/actions/workflows/pages.yml"><img src="https://github.com/a3ro-dev/qenlo/actions/workflows/pages.yml/badge.svg?branch=main" alt="GitHub Pages"></a>
</p>

<p align="center">
  <a href="https://github.com/a3ro-dev/qenlo/releases"><img src="https://img.shields.io/github/v/release/a3ro-dev/qenlo?filter=sdk-v*&amp;display_name=tag&amp;style=flat-square&amp;label=SDK%20release" alt="GitHub SDK release"></a>
  <a href="https://crates.io/crates/qenlo"><img src="https://img.shields.io/crates/v/qenlo.svg?style=flat-square&amp;label=crates.io" alt="qenlo on crates.io"></a>
  <a href="https://pypi.org/project/qenlo/"><img src="https://img.shields.io/pypi/v/qenlo.svg?style=flat-square&amp;label=PyPI" alt="qenlo on PyPI"></a>
  <a href="https://www.npmjs.com/package/@a3ro.dev/qenlo"><img src="https://img.shields.io/npm/v/%40a3ro.dev%2Fqenlo/alpha.svg?style=flat-square&amp;label=npm" alt="Qenlo on npm"></a>
  <a href="https://github.com/a3ro-dev/qenlo/actions/workflows/sdk-publish.yml"><img src="https://img.shields.io/badge/Maven_Central-not_configured-6b7280?style=flat-square" alt="Maven Central not configured"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue?style=flat-square" alt="License Apache-2.0"></a>
</p>

<p align="center">
  <a href="docs/sdks/rust.md"><img src="https://img.shields.io/badge/Rust-1.98+-black?style=flat-square&amp;logo=rust&amp;logoColor=white" alt="Rust SDK"></a>
  <a href="docs/sdks/python.md"><img src="https://img.shields.io/badge/Python-3.10+-3776AB?style=flat-square&amp;logo=python&amp;logoColor=white" alt="Python SDK"></a>
  <a href="docs/sdks/typescript.md"><img src="https://img.shields.io/badge/TypeScript-5.0+-3178C6?style=flat-square&amp;logo=typescript&amp;logoColor=white" alt="TypeScript SDK"></a>
  <a href="docs/sdks/go.md"><img src="https://img.shields.io/badge/Go-1.24+-00ADD8?style=flat-square&amp;logo=go&amp;logoColor=white" alt="Go SDK"></a>
  <a href="docs/sdks/kotlin.md"><img src="https://img.shields.io/badge/Kotlin-JVM-7F52FF?style=flat-square&amp;logo=kotlin&amp;logoColor=white" alt="Kotlin SDK"></a>
  <a href="docs/sdks/swift.md"><img src="https://img.shields.io/badge/Swift-6.0+-F05138?style=flat-square&amp;logo=swift&amp;logoColor=white" alt="Swift SDK"></a>
  <a href="docs/architecture.md"><img src="https://img.shields.io/badge/Acceleration-AVX2%20|%20NEON%20|%20WebGPU-059669?style=flat-square" alt="Acceleration"></a>
</p>

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

GitHub release assets and package registries are separate publication stages. See the [CI and release map](docs/ci.md) for triggers, gates, and outputs.

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
