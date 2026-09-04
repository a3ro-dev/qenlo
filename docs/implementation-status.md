# Heterogeneous roadmap status

This ledger separates implemented code from research directions and distribution prerequisites. “Built” means source and automated packaging exist; “physically validated” means the test ran on hardware available during implementation.

| Milestone | Status | Evidence or remaining gate |
|---|---|---|
| correctness/oracle hardening | implemented | deterministic semantic checks and independent exact truth in `qenlo-lab` |
| storage v2 WAL/manifest/mmap | implemented | atomic batch, recovery, corruption, reopen, and fail-closed tests |
| query-level router | partial | eligibility plans and hardware-bound threshold profiles are implemented; persisted calibration, online adaptation, and held-out regret evaluation remain open |
| ARM NEON fallback | implemented | target compilation; physical ARM run comes from macOS/mobile packages |
| portable exact GPU | implemented | persistent arenas, chunked bounded readback, true B×D batches |
| portable IVF-Flat | implemented | Recall@10 ≥ 0.95 tester gate and exact FP32 GPU rerank |
| IVF-SQ8 | implemented | symmetric scalar candidate stage, bounded top-R, exact FP32 GPU rerank |
| Linux NVIDIA/AMD/Intel tester | built, awaiting hardware matrix | Vulkan package workflow |
| Windows NVIDIA/Intel tester | physically validated | RTX 4050, Intel UHD, and Intel Arc Vulkan; AMD package awaits hardware |
| Apple-silicon macOS tester | built, awaiting signed hardware run | Metal package workflow |
| Snapdragon/MediaTek Android tester | built, awaiting hardware | arm64 APK records SoC and thermal state |
| A-series iOS tester | source/build validation, signing required | arm64 SwiftUI shell; Apple provisioning/TestFlight is external |
| telemetry/results viewer | implemented | bearer auth, strict 1 MiB schema, SQLite WAL, responsive dashboard |
| CUDA/HIP/native Metal/direct Vulkan kernels | not implemented | portable wgpu first; CUDA Linux/Windows implementation requirements are tracked in `docs/cuda-backend-todo.md`; add only after an independent-oracle comparison proves a material gap |
| ANE/QNN/NeuroPilot NPU stages | research gate not executed | vendor SDK access and physical devices are required for the prescribed P95/energy go/no-go |
| FP16, IVF-PQ, RaBitQ, GPU graph ANN | not implemented | downstream research milestones, not required for the six tester packages |
| multi-GPU, replication, encryption | not implemented | independent database/product milestones; keys/topology/requirements are unspecified |

The six deliverables deliberately exercise the portable implementation before native vendor backends are added. A package detecting a vendor is not evidence that a vendor-specific kernel exists; reports name the actual wgpu adapter and graphics API.
