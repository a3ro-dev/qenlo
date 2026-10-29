# Qenlo implementation notes

Audit revision: `3e2a4a9bd82e130192907fc29eee6321269fb374` (plus the benchmark-harness changes in this worktree).

## Implemented

- `CoreStore` is canonical: normalized FP32 vectors, public IDs, user/timestamp metadata, permanent slots, tombstones, and a mutation generation. Metadata indexes are rebuilt from rows.
- Exact CPU search filters first, scores every eligible row, retains a size-*k* heap, selects AVX2/FMA at runtime when available, and otherwise uses scalar code. Accumulation is FP64 and ordering is distance then ID.
- The optional USearch adapter is an in-process filtered HNSW index. Canonical live-ID membership is checked during graph search; the graph cannot resurrect a tombstoned row.
- The optional WGPU backend implements exact scan with CPU mask, compact eligible-row, and GPU-predicate modes. It also contains deterministic IVF-Flat and IVF-SQ8 candidate generation followed by exact FP32 reranking. Work is chunked and chunk top-k lists are merged on the host.
- `search_batch` is a real GPU batch. GPU buffers include persistent corpus data and a per-collection scratch arena protected by a gate. Reports expose actual backend, fallback, transfer counts, allocation, filtering mode, lock wait, and routing reasons.
- Automatic routing exists and uses workload/device state; required-GPU mode fails instead of silently falling back.
- Durable mutations publish immutable checksummed WAL generations and a checksummed `HEAD`; `flush`/`close` compact to a full snapshot. Recovery validates contiguous generations. Process-crash recovery is tested; universal power-loss durability is not claimed.

## Not implemented or not established

- The CUDA/PyTorch code used in the A6000 comparison is an external transparent prototype, not the shipped Rust/WGPU backend.
- ANN graphs and GPU buffers are derived and rebuilt after restart; they are not durable indexes.
- There is no background compaction, MVCC, remote service, distributed execution, or multi-client performance result.
- The Runpod image exposed CUDA but no usable NVIDIA Vulkan ICD. Consequently no valid shipped-WGPU A6000 sample was obtained.
- Current evidence does not cover the requested full dimension/batch/distribution Cartesian product, a controlled 1M HNSW comparison, or router regret at scale.

## Exact chunk merge

For a partition of eligible rows into chunks, any item excluded from its chunk's top-*k* has at least *k* items ahead of it in that chunk and therefore cannot be in the global top-*k*. Merging each chunk top-*k* under the same `(distance,id)` ordering is exact.
