# Qenlo documentation

Qenlo is a research-grade embedded vector store for durable, exact, metadata-filtered retrieval. It runs inside your application and stores canonical records locally. Optional CPU, WGPU, ANN, and tensor structures determine how a query runs, not which records exist.

## Start here

- [Quickstart](quickstart.md): Create, mutate, search, close, and reopen a collection.
- [Concepts](concepts.md): Canonical state, eligibility, exactness, and routing.
- [Use cases](use-cases.md): Where Qenlo fits and where it does not.
- [Trade-offs](trade-offs.md): Operational and performance costs.
- [Architecture](architecture.md): Storage and derived execution structures.
- [Feature matrix](feature-matrix.md): Implemented, optional, and unverified capabilities.

## Operate and integrate

- [Recovery policy](recovery-policy.md)
- [Portable `.qn` format](qn-format-v1.md)
- [GPU design](gpu-design.md)
- [QenloDB browser](browser.md)
- [Rust SDK](sdks/rust.md)
- [Python SDK](sdks/python.md)
- [TypeScript SDK](sdks/typescript.md)
- [Go SDK](sdks/go.md)
- [Kotlin SDK](sdks/kotlin.md)
- [Swift SDK](sdks/swift.md)

## Evidence and project status

- [Verification](verification.md)
- [Benchmark protocol](benchmark-protocol.md)
- [Implementation status](implementation-status.md)
- [Research paper](../paper/output/pdf/qenlo-final-research-paper.pdf)

Benchmark results apply only to their recorded data, hardware, source revision, runtime, and timing boundary. They do not establish a universal CPU or GPU threshold, nor do they claim production readiness.

Rust and the native ABI run in CI across Linux, Windows, and macOS. Passing a target build does not prove physical device validation. The core engine and SDKs start no background workers and make no network requests. Telemetry is an entirely separate, host-controlled service.
