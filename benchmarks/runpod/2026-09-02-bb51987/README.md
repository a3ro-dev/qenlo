# RunPod 100K x 384 quick benchmark

This run uses one local, frozen embedding pass and replays the identical
`ag-news` 100,000-vector corpus through Qenlo and OSS vector databases.

## Provenance

- RunPod pod: `pr27xjlhdfrk1s`, community RTX 3070, `$0.13/hour`
- Image: `runpod/pytorch:1.0.2-cu1281-torch280-ubuntu2404`
- Revision: `bb519873c00128e1fec93836d6f55a69ce418b57`
- Model: `sentence-transformers/all-MiniLM-L12-v2`, dimension 384
- Dataset: `fancyzhx/ag_news`; 100,000 corpus + 100 tuning + 500 evaluation rows
- Embedding output SHA-256: `70f5f11325823020c23e4b6ebbfbbb1b7849dd5fd15dd747777af7d1a5b8021a`
- Prepared `.qnb` SHA-256: `906ee4134368ce50691a2a664844d55250c33ab721c7708a6816b9caf1abe4e2`
- All replay queries use cosine/IP, `k=10`, independent distribution, full corpus,
  one native query thread, 50 warmups, and three measured repetitions.

## Results

Latency is the median of the per-run P95 query batch, in milliseconds. Recall is
evaluation Recall@10. Lower latency is better.

| Backend | Algorithm | Build | Recall@10 | P95 (ms) | Target |
|---|---|---:|---:|---:|---|
| Qenlo CPU | exact | 0.641 s | 1.0000 | 12.901 | pass |
| Qenlo USearch | HNSW | 0.627 s* | 0.9914 | 5.093 | pass |
| Chroma 1.5.9 | HNSW | 56.137 s | 0.9984 | 3.777 | pass |
| FAISS 1.15.0 | IndexFlatIP exact | 0.112 s | 1.0000 | 10.217 | pass |
| FAISS 1.15.0 | HNSW M=32, efSearch=256 | 79.727 s | 0.9994 | 1.494 | pass |
| Milvus Lite 3.0.1 | AUTOINDEX cosine | 48.447 s | 0.9988 | 76.229 | pass |
| LanceDB 0.38.0 | exact flat cosine | 1.885 s | 1.0000 | 242.256 | pass |

`*` Qenlo USearch's build field includes the library's readiness/index setup;
the raw values are preserved in `artifacts/qenlo-usearch/summary.txt`.

Qdrant was explicitly excluded: its local mode warns that corpora above 20,000
points use exact brute-force search, so it would not be an apples-to-apples ANN
cohort here. The exclusion and intended HTTP-server follow-up are recorded in
`artifacts/qdrant-excluded.json`.

## GPU status

The RTX 3070 was visible to CUDA (`nvidia-smi`), but the container's Vulkan ICD
failed to load (`libGLX_nvidia.so.0` returned no `vkCreateInstance`), and the
OpenGL fallback also failed. Qenlo therefore failed closed with no GPU timing;
no CPU fallback was mislabeled as a GPU result. The exact stderr, exit code,
environment capture, and failed output metadata are preserved under `artifacts/`.

The attempted alternate CUDA-devel image was terminated during image download,
before execution. No 1M x 768 research-gate claim is made by this quick run.

## Raw evidence

`qenlo-runpod-artifacts.tar.gz` is checksum-verified against
`qenlo-runpod-artifacts.tar.gz.sha256` and contains stdout/stderr, exit codes,
package freeze, environment capture, provenance, and backend summaries.
