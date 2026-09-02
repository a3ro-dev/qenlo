# CUDA backend TODO (Linux and Windows)

Status: **not implemented**. Qenlo currently uses its portable `gpu-wgpu`
backend. The Python/PyTorch prototype used in the September 2026 A6000 study is
not linked into a Qenlo binary and must not be described as a Qenlo CUDA backend.

Implement a native CUDA backend only after a valid, independent-oracle benchmark
shows a material benefit over the portable path and Faiss/cuVS. It needs all of:

1. An opt-in Cargo feature (`gpu-cuda`) and runtime adapter selection; requested
   CUDA must fail explicitly if the CUDA driver/device is unavailable, never
   silently use CPU or wgpu.
2. A stable FFI boundary around the CUDA Driver API (not a build-machine-specific
   `nvcc` dependency for normal users), with RAII ownership for context, stream,
   device buffers, events, and error codes.
3. Exact FP32 cosine/L2 score and deterministic top-k semantics matching the
   canonical CPU tie-breaker; predicate, row-materialization, and mask filter
   modes must share the same eligibility semantics.
4. End-to-end timing that includes query H→D, CUDA work, top-k, result D→H and
   synchronization; record transfer/allocation bytes separately.
5. CI/test matrix: Linux x86_64 NVIDIA CUDA and Windows x86_64 NVIDIA CUDA,
   plus compile-only coverage when GPU hardware is unavailable. Test required
   CUDA failure, explicit CPU fallback, cancellation/recovery, and independent
   exact-oracle recall.
6. Packaging: dynamically load the platform driver (`libcuda.so.1` on Linux,
   `nvcuda.dll` on Windows); document supported driver minimums and avoid
   redistributing NVIDIA driver libraries. Ship the same feature name and
   diagnostic schema on both targets.

Exit criterion: retain raw apples-to-apples results for Qenlo CUDA, Faiss GPU,
and cuVS on one GPU with a separate exact oracle and no correctness failures.
