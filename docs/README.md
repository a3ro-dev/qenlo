# Qenlo

> **Embedded vector search that explains its work.**

Qenlo is a lightweight, embeddable vector database built in Rust for native applications and edge devices. It keeps canonical vectors and metadata local, applies relational/temporal filters before ranking, and returns an execution report with every search.

---

## Key Guarantees

* **Exact Filtered Search**: Hard scalar metadata filtering (user ID, tenant, visibility, time ranges) is evaluated *prior* to vector scoring, preventing recall collapse.
* **Zero Background Servers**: Runs embedded directly inside your Python, Node.js, Go, Rust, Kotlin, or Swift process without running daemon services.
* **Portable `.qn` Format**: Single-file, zero-copy memory-mappable snapshot format for vector distribution and offline models.
* **Crash-Safe Durability**: Transactional write-ahead log (WAL) and atomic snapshots with checksum validation.
* **Transparent Execution Reports**: Every search yields timing, scanned candidates, filter rejections, and hardware acceleration path details.

---

## Architecture at a Glance

| Component | Responsibility | Performance Target |
| :--- | :--- | :--- |
| **Core Engine** | Memory layout, SIMD cosine/dot distance, bitmask filtering | < 5 ms for 100k vectors |
| **Storage Subsystem** | WAL replay, snapshot generation, `.qn` serialization | Crash-safe atomic commits |
| **FFI Boundary** | C-ABI with zero panics across dynamic libraries | Zero overhead foreign calls |
| **SDKs** | Python, TypeScript, Go, Kotlin, Swift idiomatic bindings | Type-safe ergonomics |

---

## Supported Platforms

* **Linux**: `x86_64` (glibc / musl) & `aarch64` (ARM64)
* **macOS**: Apple Silicon (`arm64`) & Intel (`x86_64`)
* **Windows**: `x86_64` & `ARM64`
* **Android**: `arm64-v8a`, `armeabi-v7a`, `x86_64`
* **iOS**: `arm64`, simulator `x86_64`

---

## Quick Navigation

* [Quickstart Guide](quickstart.md) — Get running in 5 minutes
* [Core Concepts](concepts.md) — Data model, filters, and records
* [Python SDK](sdks/python.md) — Prebuilt wheels and API reference
* [TypeScript SDK](sdks/typescript.md) — Node.js and Electron bindings
* [Architecture Specification](architecture.md) — Deep dive into engine internals
