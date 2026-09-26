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

Qenlo is a local, embedded vector store: one process owns a canonical durable record store, applies metadata filters before ranking, and runs exact search on CPU by default, with optional WGPU, USearch, and PyTorch execution paths. It is built for engineers embedding vector search directly into an application and for researchers who want to check the evidence behind any number before relying on it. Qenlo is not a distributed vector database, a hosted service, or a fastest-in-class ANN engine.

## Install

| Language | Command |
| --- | --- |
| Rust | `qenlo = "0.1.0-alpha.11"` in `Cargo.toml` |
| Python | `pip install qenlo==0.1.0a11` |
| TypeScript | `npm install @a3ro.dev/qenlo@alpha` |

Go, Kotlin, and Swift are preview SDKs — see [docs/sdks/go.md](docs/sdks/go.md), [docs/sdks/kotlin.md](docs/sdks/kotlin.md), and [docs/sdks/swift.md](docs/sdks/swift.md). Maven Central is not published; releases are tagged `sdk-v0.1.0-alpha.N` on [GitHub Releases](https://github.com/a3ro-dev/qenlo/releases).

## Quick example

Rust:

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

Python:

```python
from qenlo import Collection, Filter, Record

with Collection.memory(dimension=3) as db:
    db.add(Record(id=1, user_id=7, timestamp=10, vector=(1.0, 0.0, 0.0)))
    db.add(Record(id=2, user_id=7, timestamp=20, vector=(0.0, 1.0, 0.0)))

    response = db.search(query=(1.0, 0.0, 0.0), filter=Filter(user_id=7), k=10)
    print(f"Matched ID: {response.results[0].id}")
```

TypeScript:

```typescript
import { Collection } from "@a3ro.dev/qenlo";

using db = Collection.memory(3);
db.add({ id: 1n, userId: 7n, timestamp: 10n, vector: [1.0, 0.0, 0.0] });
db.add({ id: 2n, userId: 7n, timestamp: 20n, vector: [0.0, 1.0, 0.0] });

const response = db.search([1.0, 0.0, 0.0], { userId: 7n }, 10);
console.log(`Matched ID: ${response.results[0]?.id}`);
```

`Collection::new` / `Collection.memory` open an in-memory collection; `create` / `open` are for durable, reopenable collections. Public operations cover atomic mixed commits, add/delete batches, exact and batch search, filtering, preparation, statistics, flush, and close.

## Research: what we measured

| Finding | Number | Boundary | Evidence |
| --- | --- | --- | --- |
| The compact-row GPU path ran in lower power states, and its device time varied across fresh processes and hosts | up to 4.6x device-time variation across five RTX 4090 pod hosts (4.2x on a fresh laptop process); the heavier path stayed within 1-7% | Device timestamps for specific query cells, not end-to-end latency; a small extra GPU load cut the light path's slow tail, but the predicted >=10% end-to-end gain failed on every GPU | [research/README.md](research/README.md) · [paper/v2/](paper/v2/) |
| A fitted CPU/GPU routing rule failed its preregistered held-out gate | 235.7% maximum regret vs a 25% limit (rule fit on 31 development pairs) | Reverted; alpha.4's static routing shipped instead | [held-out gate report](research/data/processed/alpha5-router-heldout/report.md) |
| A faster kernel does not imply a faster search call | qualitative | Host filtering, transfers, dispatch, and readback also cost time; Qenlo reports call latency and execution diagnostics separately so comparisons keep both in view | [benchmark protocol](docs/benchmark-protocol.md) · [execution reports](docs/concepts.md) |
| Every retained evidence file is hashed and checked in CI | 1,833 git-tracked files | SHA-256 identities regenerated and diffed by the `research-evidence` CI job on every push and pull request | [research/README.md](research/README.md) · [CI map](docs/ci.md) |

Read [QENLO-RESEARCH-PAPER.pdf](QENLO-RESEARCH-PAPER.pdf) — *The Efficient Kernel Runs Slow* — or the [verification notes](docs/verification.md) before quoting any number above; each row links to its boundaries, not just its headline.

## Why Qenlo exists

Vector indexes are disposable; application data is not. Qenlo keeps IDs, normalized FP32 vectors, metadata, tombstones, and generation state in one canonical store. CPU, WGPU, USearch, and PyTorch structures are derived execution paths that can be rebuilt without changing which records exist.

## Good fit

Use Qenlo when one process owns the collection, exact results are useful, metadata filters can substantially reduce the candidate set, and local persistence matters: desktop semantic search, offline retrieval, per-user document collections, local agent memory, device-level search research. Use something else for relational joins, multi-node replication, hosted ingestion, high write concurrency, billion-scale ANN, built-in encryption, or a production SLA. See [use cases](docs/use-cases.md) and [trade-offs](docs/trade-offs.md).

## Status and limits

Qenlo is research-grade alpha software with strong correctness, recovery, and evidence-preservation tests, but it does not establish production readiness:

- no automatic CPU/GPU router has passed a held-out gate (see the table above);
- current CPU performance is not a competitive bound on optimized CPU libraries;
- WGPU availability and performance depend on the adapter, driver, and backend;
- concurrency, sustained mutation churn, crash schedules, and energy use need broader evaluation; and
- mobile packaging and current-revision physical-device validation remain incomplete.

## Execution paths

| Path | Role | Important limit |
| --- | --- | --- |
| CPU exact | Default exhaustive search | Qenlo measurements do not bound optimized CPU libraries |
| WGPU exact | Optional exhaustive acceleration | Adapter and driver support vary; fixed costs can dominate small eligible sets |
| USearch HNSW | Optional approximate search | Recall must be measured for the actual data and filters |
| PyTorch tensor | Optional Python exhaustive snapshot | Derived, not durable; CUDA and CPU measured, MPS unverified |

Automatic mode reports which route actually ran and why. Without a matching hardware profile, the built-in threshold is a fallback policy, not a transferable performance law.

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

The terminal UI's `?` tab is a function browser covering all 33 public `Collection` methods, with `/` to type-to-filter by name, group, or summary; an automated test fails if the catalog drifts from the real API. See the [browser guide](docs/browser.md).

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
- [Rust API reference on docs.rs](https://docs.rs/qenlo/latest/qenlo/)
- [Concepts](docs/concepts.md)
- [Use cases](docs/use-cases.md)
- [Trade-offs](docs/trade-offs.md)
- [Feature matrix](docs/feature-matrix.md)
- [Prioritized roadmap](docs/prioritized-roadmap.md)
- [Implementation status](docs/implementation-status.md)
- [Contributing](CONTRIBUTING.md)
- [Security model](SECURITY.md)

## License

Licensed under Apache-2.0. See [LICENSE](LICENSE).
