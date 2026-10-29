# Headline numbers

This page lists the cells where Qenlo is ahead of other engines, then the cells where it is not, then the full tables behind both.

Latencies are P95 call latency. Lower is better. Every Qenlo row has recall 1.000.

## Where Qenlo wins*

| Matchup | Qenlo | Other | Faster |
| --- | ---: | ---: | ---: |
| Exact search vs LanceDB 0.38, 100k × 384, CPU only | 12.9 ms | 242.3 ms | **18.8x** |
| Exact search vs Milvus Lite 3.0, same cell | 12.9 ms | 76.2 ms | **5.9x** |
| Time to first query vs FAISS HNSW, Chroma, Milvus Lite (index build) | 0.64 s | 79.7 s, 56.1 s, 48.4 s | **76x to 124x** |
| Selective filter vs Chroma, laptop, 1% of rows eligible | 0.24 ms | 43.0 ms | **~180x** |
| GPU vs FAISS flat (CPU), 100k × 768, k=1 | 0.90 ms | 14.8 ms | **16.5x** |
| GPU vs FAISS flat (CPU), 100k × 384, batch 16 | 4.05 ms | 33.1 ms | **8.2x** |
| GPU vs FAISS flat (CPU), 25k × 384, batch 64 | 1.26 ms | 25.3 ms | **20x** |
| GPU vs FAISS flat (CPU), 100k × 768, batch 8, k=64, 10% eligible | 0.90 ms | 3.06 ms | **3.4x** |
| Filter-first CPU vs FAISS flat (CPU), 50k × 384, 0.1% eligible | 7.5 µs | 29.3 µs | **3.9x** |
| Filter-first CPU vs FAISS flat (CPU), 10k × 384, 1% eligible | 8.2 µs | 27.6 µs | **3.4x** |
| Qenlo GPU vs Qenlo CPU, 100k × 768, batch 8, k=64 | 0.90 ms | 28.6 ms | **32x** |
| Qenlo GPU vs Qenlo CPU, 25k × 384, batch 64 | 1.26 ms | 69.1 ms | **55x** |
| Qenlo GPU vs Qenlo CPU, 100k × 384, batch 16 | 4.05 ms | 68.1 ms | **17x** |
| Qenlo GPU vs Qenlo CPU, laptop RTX 4050, 100k × 384 | 3.24 ms | 16.66 ms | **5.1x** (3.6x to 6.8x) |

\* How to read the asterisk:

- **Machines.** The LanceDB, Milvus and build-time rows come from a RunPod RTX 3070 pod. The FAISS and Qenlo GPU rows come from RunPod RTX 4090 pods. The Chroma and laptop rows come from an RTX 4050 laptop. Numbers do not transfer between machines.
- **FAISS is CPU here.** The 4090 runs used `faiss-cpu` 1.15.0, flat inner product, on pre-filtered rows. Its filter preparation is reported separately and is not in the FAISS latency.
- **Few repetitions.** The FAISS cells have 3 repetitions. The laptop cell has 5 and a bootstrap interval. Treat the ratios as direction and size, not decimals.
- **Chroma crosses an API boundary.** Its latency includes the Python call, bindings and serialization. Qenlo's number is a Rust in-process call. The Chroma cell also used 20 evaluation queries.
- **Exact vs approximate.** LanceDB flat and FAISS flat are exact. Milvus Lite, Chroma and FAISS HNSW are approximate, with recall of 0.998 to 0.999 in the 3070 cohort.

## Where Qenlo does not win

| Cell | Result |
| --- | --- |
| 100k × 384, RTX 3070 pod | FAISS HNSW answers in 1.49 ms, Chroma in 3.78 ms, Qenlo USearch in 5.09 ms. Qenlo exact CPU takes 12.9 ms and FAISS exact flat takes 10.2 ms. |
| 1M × 768, A6000, 1% eligible | FAISS on a GPU is 0.132 ms. The Qenlo CUDA prototype is 0.171 ms, 1.29x slower. It was a PyTorch prototype, not the shipped Rust and WGPU path. cuVS runs failed. |
| Small collections | Plain PyTorch CUDA matmul beats Qenlo's GPU path in several 4090 cells, for example 0.52 ms against 1.26 ms at 25k × 384, batch 64. |
| Laptop, 1% eligible | The Qenlo CPU path (0.23 ms) beats its own GPU paths (0.68 ms and 2.88 ms). |
| Qenlo GPU on the 3070 pod | No result. The container's Vulkan driver failed to load, so Qenlo failed closed instead of reporting CPU time as GPU time. |

Not run: Qdrant (local mode is brute force above 20k points), hnswlib, FAISS IVF, and the preregistered 1M × 768 gate on a host with enough RAM.

## Full cohort: 100k × 384, RTX 3070 pod

Same machine, same data, same queries, one query thread, 3 repetitions. Source: [RunPod README](https://github.com/a3ro-dev/qenlo/blob/main/benchmarks/runpod/2026-09-02-bb51987/README.md).

| System | Algorithm | Build | Recall@10 | P95 |
| --- | --- | ---: | ---: | ---: |
| FAISS 1.15.0 | HNSW, M=32, efSearch=256 | 79.7 s | 0.9994 | 1.494 ms |
| Chroma 1.5.9 | HNSW | 56.1 s | 0.9984 | 3.777 ms |
| Qenlo USearch | HNSW | 0.63 s | 0.9914 | 5.093 ms |
| FAISS 1.15.0 | flat, exact | 0.11 s | 1.0000 | 10.217 ms |
| Qenlo CPU | exact | 0.64 s | 1.0000 | 12.901 ms |
| Milvus Lite 3.0.1 | AUTOINDEX | 48.4 s | 0.9988 | 76.229 ms |
| LanceDB 0.38.0 | flat, exact | 1.9 s | 1.0000 | 242.256 ms |

## Where the other numbers live

- Laptop record, RTX 4050, 100k × 384: [measured results](results-2026-08-28.md).
- RTX 4090 small-collection matrix: [performance-matrix.csv](https://github.com/a3ro-dev/qenlo/blob/main/research/artifacts/runpod-small-2026-09-05/report/performance-matrix.csv).
- 1M × 768 A6000 gate: [strict gate README](https://github.com/a3ro-dev/qenlo/blob/main/benchmark-results/2026-09-02-runpod-a6000-strict-research-gate/README.md).
- Method and what each number can support: [benchmark protocol](benchmark-protocol.md) and [verification record](verification.md).
