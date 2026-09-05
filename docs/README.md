# Qenlo documentation

Qenlo is a research-grade embedded vector store for durable, exact, metadata-filtered retrieval. It runs inside an application and keeps canonical records locally; optional CPU, WGPU, ANN, and tensor structures decide how a query executes, not what data exists.

## Start here

- [Quickstart](quickstart.md): create, mutate, search, close, and reopen a collection.
- [Concepts](concepts.md): canonical state, eligibility, exactness, and routing.
- [Use cases](use-cases.md): where Qenlo fits and where it does not.
- [Trade-offs](trade-offs.md): operational and performance costs.
- [Architecture](architecture.md): storage and derived execution structures.
- [Feature matrix](feature-matrix.md): implemented, optional, and unverified capabilities.

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

Benchmark results apply only to their recorded data, hardware, source revision, runtime, and timing boundary. They do not establish a universal CPU/GPU threshold or production-readiness claim.

Rust and the native ABI run in CI across Linux, Windows, and macOS. A successful target build is not physical-device validation. The core and SDKs start no background worker and make no network request; telemetry is a separately deployed, host-controlled component.
