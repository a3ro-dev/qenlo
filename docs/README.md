# Qenlo documentation

Qenlo is a durable, in-process vector-search library for roughly 1k--100k vectors. Canonical Rust records define IDs, metadata, tombstones, and generations. CPU, WGPU, USearch, and PyTorch structures are derived execution choices.

## Start here

- [Quickstart](quickstart.md)
- [Architecture](architecture.md)
- [GPU design](gpu-design.md)
- [Feature and packaging status](feature-matrix.md)
- [Trade-offs](trade-offs.md)
- [Benchmark protocol](benchmark-protocol.md)
- [September 2026 implementation status](implementation-status.md)
- [QenloDB browser](browser.md)

## Platform evidence

Rust and the native ABI are exercised locally and in CI. Current performance evidence covers Windows and Linux-hosted Vulkan GPUs. Kotlin/JVM is not Android packaging evidence. Apple simulator, Apple device, Android package, and physical mobile performance checks remain separate release gates.

The core and SDKs start no background worker and make no network request. The optional telemetry collector is a separate service controlled by the host application.
