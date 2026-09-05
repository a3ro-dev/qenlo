# Heterogeneous roadmap status

This ledger separates implemented code from research directions and distribution prerequisites. “Built” means source and automated packaging exist; “physically validated” means the test ran on hardware available during implementation.

| Milestone | Status | Evidence or remaining gate |
|---|---|---|
| correctness/oracle hardening | implemented | deterministic semantic checks and independent exact truth in `qenlo-lab` |
| storage v2 WAL/manifest/mmap | implemented | atomic batch, recovery, corruption, reopen, and fail-closed tests |
| query-level router | partial | eligibility plans and hardware-bound threshold profiles are implemented; persisted calibration, online adaptation, and held-out regret evaluation remain open |
| ARM NEON fallback | implemented | target compilation; physical ARM run comes from macOS/mobile packages |
| portable exact GPU | implemented | persistent arenas, chunked bounded readback, true B×D batches |
| small-collection Runpod matrix | completed | 182 retained rows across six GPU configurations plus a corrected 100K × 768 supplement; final known spend $0.9401 |
| lane-minimum selector candidate | measured and rejected | won five and lost seven of 12 qualified pairs; final source retains the simpler full-rescan selector |
| post-mutation resident updates | implemented and measured | append/live-mask cells rebuilt zero times; reopen remains a full resident rebuild |
| optional PyTorch index | implemented, desktop-only | lazy CPU/CUDA/MPS tensor API bound to canonical generation; CUDA and CPU measured, MPS unavailable in this campaign |
| portable IVF-Flat | implemented | Recall@10 ≥ 0.95 tester gate and exact FP32 GPU rerank |
| IVF-SQ8 | implemented | symmetric scalar candidate stage, bounded top-R, exact FP32 GPU rerank |
| Linux NVIDIA tester | campaign validated | six cloud configurations; AMD/Intel and physical phone Linux remain untested |
| Windows NVIDIA/Intel tester | physically validated | RTX 4050, Intel UHD, and Intel Arc Vulkan; AMD package awaits hardware |
| Apple-silicon macOS tester | built, awaiting signed hardware run | Metal package workflow |
| Snapdragon/MediaTek Android tester | source present, package/device gate open | arm64 tester records SoC and thermal state; no current package or physical run evidence |
| A-series iOS tester | source/build validation, signing required | arm64 SwiftUI shell; Apple provisioning/TestFlight is external |
| optional results collector/viewer | implemented | separate host-owned service with bearer auth, strict 1 MiB schema, SQLite WAL, and no SDK auto-export |
| CUDA/HIP/native Metal/direct Vulkan kernels | not implemented | portable wgpu first; CUDA Linux/Windows implementation requirements are tracked in `docs/cuda-backend-todo.md`; add only after an independent-oracle comparison proves a material gap |
| ANE/QNN/NeuroPilot NPU stages | research gate not executed | vendor SDK access and physical devices are required for the prescribed P95/energy go/no-go |
| FP16, IVF-PQ, RaBitQ, GPU graph ANN | not implemented | downstream research milestones, not required for the six tester packages |
| multi-GPU, replication, encryption | not implemented | independent database/product milestones; keys/topology/requirements are unspecified |

The six deliverables deliberately exercise the portable implementation before native vendor backends are added. A package detecting a vendor is not evidence that a vendor-specific kernel exists; reports name the actual wgpu adapter and graphics API.
