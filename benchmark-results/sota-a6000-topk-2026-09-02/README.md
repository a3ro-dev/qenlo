# TopK Bench A6000 GPU collection — not a performance claim

## Verdict

**No demonstrated advantage.** This collection does not support the proposed
claim that Qenlo is the fastest reproducible exact filtered GPU search system.

The closest completed cohort used the public TopK Bench 1M × 768 data,
top-k=10, one query per call, ten shuffled repetitions of 1,000 queries, and
host-call-to-host-result timing. It used cosine similarity and the *inclusive*
`int_filter <= threshold` predicate used by the public TopK provider code. The
source Parquet files were not modified; in-memory normalized FP32 working copies
were used to implement cosine through inner product. Eligible-set materialization
and index creation were excluded for every adapter.

The 1% provider-compatible cell actually had 10,148 candidates (1.0148%), not
the requested strict `int_filter < 100` condition. Therefore it is not a valid
result for the requested strict-1%-selectivity headline.

| Adapter | Eligible rows | Recall@10 | P50 ms | P95 ms | P99 ms |
|---|---:|---:|---:|---:|---:|
| Qenlo CUDA **prototype** | 10,148 | 1.0000 | 0.1680 | 0.1834 | 0.1901 |
| Faiss GPU Flat IP | 10,148 | 1.0000 | 0.1237 | 0.1332 | 0.1365 |
| cuVS brute-force IP | 10,148 | 1.0000 | 0.5243 | 1.1156 | 1.4914 |

The Qenlo prototype P95 is 1.38× Faiss P95 (slower), so it misses the requested
2×-lower-P95 gate. It is also a PyTorch implementation in
`scripts/run_topk_a6000_exact_gpu.py`, not the shipped Qenlo Rust/wgpu binary.
It must not be represented as a Qenlo product/backend result.

## Validity and retained failures

- The 10% and 100% inclusive cells have non-perfect agreement with TopK’s
  supplied ground truth (including Qenlo). They are retained in `summary.json`
  but excluded from exact-result comparisons. A separate independent exact
  oracle has not been run, so the source of those disagreements is unresolved.
- `cuvs_initial_failure/` retains an initial output-copy binding failure.
  `cuvs_invalid_result_order/` retains a complete but invalid run where
  cuVS distances and IDs were assigned in the wrong order. Neither is deleted
  or used in the table.
- The earlier non-cosine/strict-less-than cohort is retained in sibling folders
  but is protocol-invalid because it did not match TopK’s cosine/provider
  semantics. It is not summarized as a result.
- No kernel-only figures, CPU oracle, QPS/concurrency scaling, memory,
  allocation/transfer-byte, energy, CPU baseline, or index-build comparison are
  claimed here. No local embedding model cohort, 10M cohort, or other hardware
  cohort was run.

## Reproduction and files

The runner is `scripts/run_topk_a6000_exact_gpu.py`; the independent statistical
checker is `scripts/summarize_topk_exact_gpu.py`. Raw per-query samples are in
`cosine_inclusive/{qenlo,faiss,cuvs}/raw_samples.csv`; all summary numbers are
computed from those files by `cosine_inclusive/summary.json`.

Run identity:

- GPU: NVIDIA RTX A6000, 49,140 MiB; driver 580.159.03; CUDA 12.8.
- Qenlo prototype: PyTorch 2.8.0+cu128.
- Faiss: 1.14.1. cuVS: 26.08.01.
- TopK Bench checkout: `398ce7d72dea7ad765ada54c5daa014c498efb52`.
- documents SHA-256: `f1c9f0dd07c1d4b6da8fcf697b53a899ddc68e1de400922360aa3741671275d3`.
- queries SHA-256: `d9a34c1825ef4524da7a89f2a5b8be08bde1e98ce3e0b2ac0a1bf34a3b800870`.

The A6000 RunPod was priced at $0.53/hour and was deleted after collection.
The collection costs were not retrieved from RunPod billing, so no exact spend is
claimed; the raw cloud billing record remains an outstanding gap.
