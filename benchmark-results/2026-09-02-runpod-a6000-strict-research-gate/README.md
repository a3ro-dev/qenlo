# Strict TopK 1M research gate — RTX A6000

This bundle is a retained, bounded research-gate cohort.  It must not be
described as a shipped Qenlo benchmark: the Qenlo entry is a PyTorch CUDA
predicate prototype.  Source vectors and queries are the public TopK Bench
1M corpus; their SHA-256 values are recorded in each environment manifest.

## Valid measurement cells

Every valid cell uses 1,000,000 source rows x 768 FP32 dimensions, cosine via
temporary FP32-normalized working copies, `int_filter < 100`, batch size 1,
k=10, 100 warmups, five deterministic repetitions of 1,000 queries, and an
independent CPU float64 exhaustive oracle over those FP32 working copies.  The
measurement starts at the host search call and stops once IDs/distances have
returned to the host; H2D query transfer, GPU work, D2H results, and
synchronization are included.  Loading, normalization, oracle construction,
and index construction are excluded.

| System | Samples | Recall@10 (min/mean) | P50 ms | P95 ms | P99 ms | Status |
|---|---:|---:|---:|---:|---:|---|
| NumPy CPU exhaustive FP32 | 5,000 | 1.0 / 1.0 | 0.121747 | 0.197311 | 0.228314 | valid |
| FAISS GPU `GpuIndexFlatIP` | 5,000 | 1.0 / 1.0 | 0.126051 | 0.132222 | 0.145826 | valid |
| Qenlo CUDA predicate prototype | 5,000 | 1.0 / 1.0 | 0.160011 | 0.170722 | 0.180932 | valid, prototype only |
| cuVS brute-force | 0 | — | — | — | — | failed; retained below |

Qenlo is 1.29x slower than FAISS at P95 in this strict, valid cell.  It is
only 1.16x lower P95 than the retained CPU exhaustive reference, below the
pre-registered 2x gate.  Thus the result supports exactness of the prototype,
but **does not demonstrate a latency advantage or a fastest-search claim**.

## Retained failures and non-comparable runs

- `strict-gate/failures.json`: Qenlo failed with `CUBLAS_STATUS_INVALID_VALUE`
  when a Faiss-specific cuBLASLt preload was shared with PyTorch.  The valid
  Qenlo result is in `strict-qenlo-isolated/` and records its own environment.
- `strict-gate/raw_samples.csv`: initial cuVS output was interpreted using the
  wrong return order, producing recall 0.9.  This result is invalid and retained,
  not used in the table.
- `strict-cuvs-corrected/failures.json`: corrected adapter failed to locate
  `libcuvs_c.so`.
- `strict-cuvs-corrected-v2/failures.json`: after adding RAPIDS runtime paths,
  cuVS failed on an incompatible cuSOLVER/cuBLAS symbol.  No cuVS latency is
  claimed.
- `strict-gate/import_faiss_failure.log` documents the initial Faiss/cuBLASLt
  load conflict; `import_all.log` documents the resolved import configuration.

## Reproduce

Use `run_strict_topk_research_gate_v2.py` with the supplied TopK Bench Parquet
files.  Run Qenlo in an isolated PyTorch CUDA process and Faiss/cuVS in their
respective dependency environments.  Do not combine raw rows from invalid or
failed directories into a performance summary.  `raw_samples.csv`,
`independent_fp64_oracle.npz`, `summary.json`, `failures.json`, manifests,
logs, and the exact scripts are all retained.

The RunPod pod was RTX A6000 in US-TX-1 at $0.53/hour.  It was created
2026-09-02 08:17:24 UTC and deleted after collection at roughly 08:35 UTC,
for an estimated $0.16.  RunPod did not provide an itemized final charge in the
CLI response, so this is an estimate, not a billed-total assertion.
