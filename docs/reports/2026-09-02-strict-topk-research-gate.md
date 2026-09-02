# Qenlo strict GPU research-gate addendum

## Abstract

We evaluated a CUDA predicate prototype for Qenlo on the public TopK Bench
1M corpus under a strict filtered exact top-10 workload.  On a single RTX
A6000, the prototype achieved perfect recall against an independent CPU
float64 exhaustive oracle over five repetitions and 5,000 measured queries.
Its P95 end-to-end latency was 0.170722 ms.  FAISS GPU Flat IP achieved
0.132222 ms under the same workload and boundary.  Therefore this cohort
does not support a fastest exact filtered GPU-search claim or the
pre-registered 2x-lower-than-CPU P95 gate.  cuVS did not yield a valid
measurement on this pod because of retained runtime-library failures.

## Problem definition

For each query `q`, select rows satisfying `int_filter < 100`, then return
the ten candidate IDs with greatest cosine similarity.  The source files
remain unaltered FP32 embeddings.  The systems normalize temporary FP32
working copies and use inner product, which is equal to cosine for nonzero
unit-norm vectors.  The reference ranking is calculated separately using
float64 CPU matrix multiplication over those same FP32 working vectors.

## Experimental protocol

- Dataset: public TopK Bench `docs-1m.parquet` and `queries-1m.parquet`.
  SHA-256: `f1c9f0dd07c1d4b6da8fcf697b53a899ddc68e1de400922360aa3741671275d3`
  and `d9a34c1825ef4524da7a89f2a5b8be08bde1e98ce3e0b2ac0a1bf34a3b800870`.
- Workload: 1,000,000 rows, 768 dimensions, 1% strict filter condition,
  batch 1, k=10, 1,000 evaluation queries, 100 warmups, and five repetitions.
- Timing: host invocation until top-10 IDs and distances are available back on
  the host.  It includes query H2D transfer, GPU execution, result D2H transfer,
  and synchronization.  It excludes construction, loading, embedding
  generation, filter creation, and oracle construction.
- Hardware: RunPod RTX A6000 (48 GiB), 64 vCPUs, driver 570.133.07,
  secure US-TX-1.  CPU affinity and BLAS thread environment are in each
  `environment.json`.
- Qenlo status: prototype implemented with PyTorch CUDA, not the shipped Qenlo
  Rust/WGPU backend.  It is not evidence of product performance or portability.

## Results

| System | Samples | min recall@10 | P50 ms | P95 ms | P99 ms |
|---|---:|---:|---:|---:|---:|
| NumPy CPU exact FP32 | 5,000 | 1.0 | 0.121747 | 0.197311 | 0.228314 |
| FAISS GPU Flat IP exact | 5,000 | 1.0 | 0.126051 | 0.132222 | 0.145826 |
| Qenlo CUDA predicate prototype | 5,000 | 1.0 | 0.160011 | 0.170722 | 0.180932 |
| cuVS brute force | 0 | — | — | — | — |

The Qenlo-to-FAISS P95 ratio is 1.29, meaning Qenlo is slower in this cell.
Relative to the retained CPU exact reference, Qenlo's P95 reduction is 1.16x,
not the required 2x.  No confidence interval is asserted because the primary
result is already a failed gate; retained per-query data supports an
independent bootstrap calculation.

## Failures and threats to validity

cuVS results are not omitted: an initial adapter return-order error produced
0.9 recall; correction attempts hit a missing `libcuvs_c.so`, then an
incompatible cuSOLVER/cuBLAS symbol.  All logs and failure JSON are retained.
Faiss required a cuBLASLt preload in its process; that preload caused a
PyTorch GEMV failure, so Qenlo was measured in an isolated process.  This
weakens process-level apples-to-apples comparability, though all valid cells
share the same hardware, corpus, filter, query order protocol, timing boundary,
and oracle.

The campaign remains incomplete: no native Qenlo CUDA backend, no Qdrant/
Milvus/USearch/hnswlib/Chroma cells, no 0.1/10/100% strict selectivities,
no batch/concurrency sweep, no 384-dimensional run, no three-seed campaign,
no other GPU types, no energy measurement, and no 10M workload.  These are
gaps, not negative evidence for all potential Qenlo contributions.

## Verdict

**No demonstrated advantage** for the claimed exact filtered GPU-latency
regime.  A future systems argument may focus on portable/embedded deployment,
but it requires a shipped native CUDA implementation and a completed,
dependency-controlled baseline campaign before any comparative performance
claim.

## Reproducibility

The complete raw bundle is at
`benchmark-results/2026-09-02-runpod-a6000-strict-research-gate/`; see its
README for raw files, checksums, scripts, environments, and retained failures.
