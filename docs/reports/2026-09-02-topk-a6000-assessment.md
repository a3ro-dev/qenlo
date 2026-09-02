# Qenlo GPU assessment: TopK Bench A6000 collection

## Abstract

We evaluated a CUDA/PyTorch predicate-search prototype, Faiss GPU Flat, and
cuVS brute force on the public TopK Bench 1M × 768 corpus using cosine search,
top-k=10, a one-query call boundary, and the TopK provider’s inclusive integer
filter semantics. At the approximately 1% cell, all three systems matched the
supplied TopK top-10 lists in all 10,000 retained query calls. Qenlo’s prototype
P95 was 0.1834 ms, versus 0.1332 ms for Faiss GPU Flat. The proposed fastest
exact-filtered-search claim is therefore unsupported. More importantly, the
prototype is not the shipped Qenlo Rust/wgpu binary, and the study lacks an
independent exact oracle. This is an incomplete research campaign, not evidence
for a systems-performance contribution.

## Problem definition

For a query vector `q`, vectors `x_i`, an eligibility predicate `F(i)`, and
`k=10`, exact filtered cosine top-k returns the ten IDs with highest cosine
score among all `i` satisfying `F(i)`, with deterministic handling of ties.
The intended hard gate was 1,000,000 vectors, 768 dimensions, strict 1%
selectivity, batch 1, independently computed recall@10=1.0, and at least 2×
lower P95 than the best validated CPU baseline.

## Algorithms and implementation

The Qenlo line in this report is `qenlo_cuda_predicate_prototype`, implemented
with PyTorch GPU matrix-vector multiplication followed by GPU top-k and host
readback. Faiss is `GpuIndexFlatIP`; cuVS is its brute-force inner-product
index. To represent cosine, vectors and queries were normalized in temporary
in-memory FP32 working copies; the public Parquet bytes were never altered.
Each adapter received the same pre-materialized eligible vector subset. This
excludes filtering/index setup from the timing, and it means the cuVS result is
not a measurement of its native prefilter API.

This code is an experiment harness, not integration with Qenlo’s actual
portable GPU backend. The Qenlo binary continues to use wgpu; a Linux/Windows
CUDA backend remains explicitly unimplemented in `docs/cuda-backend-todo.md`.

## Experimental protocol

Public TopK Bench documents and queries were used unchanged:

- documents SHA-256: `f1c9f0dd07c1d4b6da8fcf697b53a899ddc68e1de400922360aa3741671275d3`;
- queries SHA-256: `d9a34c1825ef4524da7a89f2a5b8be08bde1e98ce3e0b2ac0a1bf34a3b800870`;
- TopK Bench source revision: `398ce7d72dea7ad765ada54c5daa014c498efb52`.

The hardware was an NVIDIA RTX A6000 (49,140 MiB), driver 580.159.03, CUDA
12.8. PyTorch was 2.8.0+cu128; Faiss 1.14.1; cuVS 26.08.01. For each adapter
and selectivity, 100 queries warmed the system, followed by 10 repetitions of
1,000 deterministically shuffled queries. Timed calls begin immediately before
the host invokes search and finish after IDs/scores are host-resident, including
query H→D transfer, device execution, top-k, D→H transfer, and synchronization.

The public provider implementation uses `int_filter <= threshold`; hence the
reported “1%” cell has 10,148 eligible rows, not the requested strict `<100`
predicate. This result is described as provider-compatible approximately 1%,
not as satisfying the strict preregistered gate.

## Results

Only the provider-compatible approximately-1% cell passed the supplied ground
truth check for every sample. Values are from 10,000 raw calls per row; the
median bootstrap 95% intervals are retained in `summary.json`.

| System | Recall@10 | P50 ms | P95 ms | P99 ms | Result |
|---|---:|---:|---:|---:|---|
| Qenlo CUDA prototype | 1.0000 | 0.1680 | 0.1834 | 0.1901 | slower than Faiss |
| Faiss GPU Flat | 1.0000 | 0.1237 | 0.1332 | 0.1365 | fastest measured row |
| cuVS brute force | 1.0000 | 0.5243 | 1.1156 | 1.4914 | slower; different Python binding path |

The Qenlo prototype’s P95 / Faiss P95 ratio is 1.38. Thus Qenlo is slower in
this measured cell; it does not meet the target, let alone a 2× advantage.

At 10% and 100%, all adapters had some supplied-ground-truth disagreements:
Qenlo had 10 and 200 non-perfect rows respectively out of 10,000; Faiss had 0
and 120; cuVS had 10 and 150. Those rows must be regarded as invalid for exact
performance comparisons until a separate oracle resolves them.

## Interpretation and theoretical claims

No formal theoretical performance claim is made, so there is no proof to offer.
A filtered candidate-set
materialization strategy can reduce exact search work proportionally to the
eligible set, but it does not establish a general Qenlo advantage; in the
approximately-1% measurement, the same basic strategy is faster through Faiss.

Qenlo’s portable wgpu execution is a separate deployability property: it can
target embedded and heterogeneous devices where a CUDA-only Faiss GPU path is
not available. This study does not quantify that property or establish novelty
against other portable vector-search systems.

## Related work

This experiment uses the public [TopK Bench](https://github.com/topk-io/bench)
corpus and its supplied recall lists. Faiss GPU Flat and NVIDIA cuVS
brute-force search are the direct GPU flat-search comparators. They are not
portable embedded-device substitutes for Qenlo’s wgpu path, which is an
important deployment distinction, but they remain relevant latency comparators
on the same NVIDIA GPU.

## Completion matrix

| Required campaign element | Status |
|---|---|
| Public TopK 1M × 768 corpus, batch 1, k=10 | measured |
| Same-A6000 Qenlo prototype / Faiss / cuVS | measured |
| 10 × 1,000 query repetitions | measured |
| Strict 1% `<100` cosine cell | not completed |
| Independent exact CPU oracle | not completed |
| Shipped Qenlo GPU backend | not completed; prototype only |
| Valid exact comparison at 10% and 100% | failed recall gate |
| CPU, ANN, database baseline matrix | not completed |
| 384 dimensions, batches 8/32/128, three seeds | not completed |
| Second RunPod GPU, non-NVIDIA GPU, CPU-only hardware | not completed |
| Kernel-only, QPS/concurrency, memory, energy | not completed |
| 10M extension | not completed |

## Threats to validity and remaining gaps

The campaign is incomplete. It has no independent CPU oracle, strict `<100`
cell under cosine, local-embedding cohort, Qenlo binary CUDA backend, CPU
baseline, HNSW/IVF/USearch/hnswlib/Chroma/Qdrant baseline matrix, query
concurrency/QPS test, kernel-only timing, memory/transfer/allocation/energy
measurement, three data seeds, dimensions 384/768 matrix, batch 8/32/128,
IVF/SQ8 ablations, second RunPod GPU, non-NVIDIA GPU, 10M scale, plots beyond
the retained P95 overview, or exact billing total. No performance conclusion
should be extrapolated beyond the one valid approximate-1% prototype cohort.

## Reproducibility appendix

Raw samples, environment manifests, logs, failed attempts, and summaries are
in `benchmark-results/sota-a6000-topk-2026-09-02/`. Re-run the plot with:

```text
python scripts/plot_topk_exact_gpu.py
```

Recompute statistics (it fails closed on recall misses unless explicitly told
to inspect inexact rows):

```text
python scripts/summarize_topk_exact_gpu.py benchmark-results/sota-a6000-topk-2026-09-02/cosine_inclusive
```

## Final verdict

**No demonstrated advantage.**
