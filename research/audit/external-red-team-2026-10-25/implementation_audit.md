# Qenlo implementation audit (Phase 2D, 2E, 5) — WORK IN PROGRESS
HEAD 142e96cf43684a13e95513a161bda4515ad6f1ca (origin/main, 2026-10-25 15:16 +0530). Crates identical between 99a92c9 (E-LOCK data revision) and HEAD.
Interim findings (to be expanded):
- Router at HEAD: crates/qenlo/src/lib.rs:56 AUTOMATIC_GPU_MIN_ELIGIBLE_ROWS=4096; route_automatic_to_cpu_batch lib.rs:2063-2073 => CPU iff E<threshold && batch<8. No E*D*B rule in any crate revision. Benchmarks use forced backends (gpu-rows / gpu-predicate = WgpuRequired), router not exercised.
- Heavy path: EligibilityPlan.transfer_bytes = 0 for ShaderPredicate (lib.rs:559) BUT gpu.rs chunk_eligibility mode 2 uploads chunk.live.clone() (4N bytes) per query; raw upload_bytes = 401,600 = 1536 + 400,000 + 64. Paper's 4N claim is correct; planner field misreports.
- Readback is k candidates per chunk (192 B = 10*16 + 32 timestamp bytes), not 2k.
- qenlo-core 20 unit tests pass via rustc 1.97 (crates.io blocked; cargo unusable).
