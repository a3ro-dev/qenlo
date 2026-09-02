# Qenlo systems contribution audit

## Abstract

This audit tests whether Qenlo has evidence for a systems contribution under a preregistered gate: 1,000,000 vectors of dimension 768, 1% eligibility, batch 1, k=10, independent exact recall, five measured repetitions, and at least 2x lower P95 than the fastest recall-qualified CPU baseline. The gate was not executed. The local RTX 4050 laptop was safety-excluded because its available host memory was insufficient for the runner's documented multi-copy residency requirement. A retained RunPod RTX 3070 100k x 384 cohort was checksum-verified but does not match the gate and its Qenlo GPU cell failed due to no usable graphics adapter. Two new cost-capped RunPod scheduling attempts (A4000 and RTX 3090) failed before allocation. Existing 100k x 384 evidence shows an exact Qenlo GPU predicate path lower P95 than Qenlo exact CPU for an all-row workload, but slower at 1% selectivity. The only defensible final verdict is **no demonstrated advantage** for the preregistered claim.

## Problem definition and decision rule

For a normalized corpus `X` with metadata predicate `P`, a query `q`, and `k=10`, the target result is the ordered set of the k smallest cosine distances `1 - <q,x>` among live `x` satisfying `P(x)`. The oracle independently evaluates every eligible row in float64. The contribution gate passes only if all requested conditions below hold on the same 1M x 768 workload:

1. every Qenlo GPU sample executes on the required GPU backend with no fallback and has complete transfer/allocation telemetry;
2. CPU and GPU meet the recall target on held-out evaluation queries;
3. the selected fastest CPU baseline is recall-qualified; and
4. the median of five run P95 values gives a CPU/GPU ratio of at least 2.0.

No result in this report meets the workload shape, repetition count, and GPU-success requirements simultaneously. Therefore the decision rule evaluates to false; it is not treated as an estimate.

## Algorithm and architecture

Qenlo normalizes vectors, retains canonical FP32 host storage, and implements an exact GPU scan using a portable WGSL kernel. A GPU predicate, compact eligible-row materialization, or CPU mask determines eligibility. Per chunk, the GPU emits its top-k candidates; the CPU merges these bounded lists. Optional IVF-Flat and IVF-SQ8 stages are candidate generators followed by exact FP32 reranking, hence approximate overall only when unprobed candidates are omitted. The automatic router routes low-eligible single-query work to CPU; any fallback is recorded.

### Formal claim: exact chunk merge

Let `C_1,...,C_m` be a partition of the eligible rows, with a deterministic total ordering by `(distance,id)`. Let `T_i` be the first k items of chunk `C_i` under that ordering. Then the global first k items of the union are contained in `union_i T_i`.

Proof. If an item `z` is not in `T_i` for its chunk, that chunk contains k distinct items preceding `z`. Those same k items precede `z` in the global ordering, so `z` cannot be globally top k. Thus discarding all items outside every `T_i` preserves the global top k. Merging the retained sets under the same total ordering returns the exact result. This is a correctness property of the chunked exact scan, not a novelty claim or a performance bound.

## Experimental protocol

The retained Windows cohort uses a locally inferred, pinned AG News embedding corpus, disjoint corpus/tuning/evaluation ranges, precomputed float64 truth, k=10, batch 1, 200 warmups, five repetitions, and a synthetic independent metadata distribution. It reports an in-process Rust `search_batch` boundary including filtering, transfers, synchronization, readback, and result merging; index construction and oracle work are outside per-query latency. The retained Linux RunPod cohort uses the same style of local model inference but is a different 100k x 384 dataset/checksum and uses only 100 tuning, 500 evaluation queries, 50 warmups, and three repetitions. It must not be pooled with the Windows cohort.

FAISS was configured as CPU `IndexHNSWFlat` with `M=32`, `efConstruction=200`, and `efSearch=256`; the official Faiss tutorial documents these controls. Chroma used `PersistentClient` with configured HNSW settings. Qdrant local mode was explicitly excluded at 100k because the retained runner documented its nonrepresentative exact fallback warning. These are implementation facts, not a complete 1M comparison.

## Results

### Closest comparable retained evidence: Windows 100k x 384, all rows eligible

| Backend | Result type | Median run P95 | Recall@10 | Repetitions | CPU/GPU P95 ratio, 95% bootstrap interval | Notes |
|---|---|---:|---:|---:|---|---|
| Qenlo exact CPU | exact | 16.6567 ms | 0.99998 | 5 | reference | in-process |
| Qenlo GPU predicate | exact | 3.2404 ms | 0.99998 | 5 | 5.1403 [3.5521, 6.7749] | required discrete GPU, DX12 |

The exact CPU/GPU P95 ratio is 5.1403 with a seeded whole-run bootstrap interval [3.5521, 6.7749]. The retained raw comparison is `benchmarks/2026-08-28/real/comparisons/cpu-vs-gpu-predicate-384-all.json`. This is an all-row 100k x 384 observation; it is not the preregistered selective 1M x 768 result.

At 1% selectivity on the same old 100k x 384 host, Qenlo exact CPU has median run P95 0.2304 ms while GPU predicate has 2.8832 ms. Thus the available evidence is directionally adverse for the required selective workload and rules out generalizing the all-row result.

### Retained RunPod cohort: 100k x 384, all rows eligible

The archive SHA-256 matches its retained sidecar. CPU and USearch runs have 1,500 completed samples each (three runs x 500 queries). Qenlo GPU produced no samples: adapter creation failed with `GPU unavailable: No suitable graphics adapter found`. The following completed results are descriptive only and have no filter, no five-run interval, and a different Linux host:

| Backend | Type | Median P95 | Recall@10 | Build time |
|---|---|---:|---:|---:|
| Qenlo CPU | exact | 12.9014 ms | 1.0000 | not separately retained |
| USearch | approximate | 5.0933 ms | 0.9914 | not separately retained |
| FAISS Flat | exact | 10.2171 ms | 1.0000 | 0.112 s |
| FAISS HNSW | approximate | 1.4939 ms | 0.9994 | 79.727 s |
| Chroma | approximate | 3.7767 ms | 0.9984 | 56.137 s |

Raw per-sample files and original logs are retained under `retained-archive/artifacts/`. The table makes no Qenlo GPU comparison because the GPU cell failed.

### Cloud availability and cost

The two new RunPod requests were explicitly pinned to advertised available locations but failed at scheduler allocation. Both errors and prices are retained in `raw/runpod-scheduler-attempts.json`. No pod from this audit remained running afterward, and new pod runtime cost is $0.00. Historical account billing shown by RunPod is not attributed to this campaign and is not included in the reported cost.

### Campaign accounting

| Item | Count | Status |
|---|---:|---|
| Newly completed benchmark cells | 0 | no pod was allocated; local large execution was safety-excluded |
| Newly failed scheduler cells | 2 | A4000 and RTX 3090, both before allocation |
| Retained completed Linux 100k x 384 backend cells | 7 | CPU, USearch, Chroma, FAISS Flat/HNSW, Milvus, LanceDB |
| Retained failed Linux GPU cells | 1 | Qenlo GPU adapter creation failed |
| Cloud resources remaining | 0 pods; 0 network volumes | independently listed after attempts |

The exact remaining gaps are the full preregistered 1M x 768 cell, a pinned independent one-million-text source and local 768-dimensional model run, five-repetition Qenlo CPU/GPU comparisons, every requested large-scale ablation, FAISS IVF/hnswlib/service-mode Qdrant at the gate, two allocated RunPod GPU types, and a matching non-NVIDIA GPU run. No energy measurement was available.

## Ablations and hardware scaling

The requested matrix is recorded in `RESULT_MATRIX.csv`. It shows completed prior 100k x 384 observations, not surrogate gate results. There is no valid hardware-scaling curve: the only retained non-NVIDIA evidence is a 100k x 384 Intel Arc device-lab run, while the RunPod GPU attempt failed and the local RTX 4050 is a different Windows host. Plotting a fitted scaling line from these would be interpolation, so none is presented.

## Related work

FAISS provides exact and ANN CPU/GPU indexes including HNSW and IVF controls. USearch and hnswlib are HNSW-family ANN libraries. Chroma, Qdrant, and Milvus are vector databases with materially different persistence, filter, service, and API-boundary behavior. Qenlo's portable exact GPU scan should be compared against these only with identical vectors, predicates, recall oracle, search tuning, and explicitly separated in-process and end-to-end/RPC timings. The current evidence does not meet that comparison standard at the requested scale.

## Limitations and threats to validity

- The hard gate is unmeasured; there is no 1M x 768 result, no 1% selective GPU result at scale, and no qualified 2x comparison.
- No source corpus of one million independently embedded 768-dimensional texts was generated. Fabricating a duplicated or interpolated corpus would violate the protocol, so none was used.
- The retained RunPod 100k x 384 cohort has a dirty worktree and only three repetitions; its GPU failure is retained rather than retried away.
- hnswlib, FAISS IVF, service-mode Qdrant, and a controlled Milvus/Qdrant HTTP cohort are absent at the gate.
- Required router, reranking, IVF, SQ8/FP32, batch, selectivity, dimension, and three-seed ablations remain open at the gate.
- Energy was unavailable in every retained run. Host RSS was unavailable for the Linux archive; allocation telemetry is Qenlo-owned buffers, not physical VRAM.
- Prior hardware differs by OS, driver, API, CPU, and dataset; it cannot be used to estimate a cloud scaling relationship.

## Reproducibility appendix

1. Verify the retained archive and recompute statistics:

   `python scripts/verify_campaign.py --archive benchmarks/runpod/2026-09-02-bb51987/qenlo-runpod-artifacts.tar.gz --archive-sha256 benchmarks/runpod/2026-09-02-bb51987/qenlo-runpod-artifacts.tar.gz.sha256 --extracted benchmark-results/2026-09-02-runpod-archive/retained-archive --output benchmark-results/2026-09-02-runpod-archive/verification.json`

2. Recompute compatible-run intervals from any future gate artifacts:

   `python scripts/compare_runs.py --baseline <cpu-run> --candidate <gpu-run> --output <fresh-comparison.json>`

3. Evaluate the immutable gate only after complete matching artifacts exist:

   `python scripts/research_gate.py --cpu <cpu-run> --cpu <usearch-run> --gpu <gpu-predicate-run> --output <fresh-gate.json>`

4. The intended RunPod command sequence is documented in `scripts/run_runpod_benchmark.sh`. It must be extended only after selecting a pinned one-million-text source and a pinned local 768-dimensional model revision, then run on allocated hardware. Every failed allocation and workload must remain in the result directory.

5. Test the result tooling with `python -m unittest discover -s scripts -p 'test_*.py'`. The deliberately rejected gate invocation and an initial wrong test command are documented in `raw/verification-failures.md`.

## Final verdict

**no demonstrated advantage**. The observed 100k x 384 all-row GPU result is real but outside the gate, and the observed 1% selective result is slower than CPU. The required 1M x 768 evidence is absent.
