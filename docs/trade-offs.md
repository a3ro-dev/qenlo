# Trade-offs

Qenlo is an in-process vector store for small collections. It is a fit when one application owns the data, needs durable local records, and can express filtering as optional user equality plus a timestamp range.

## What Qenlo buys

- One canonical record model across CPU, WGPU, ANN, and tensor-derived views.
- Exact filtered search with deterministic distance-then-ID ordering.
- Durable batches, tombstones, checksum validation, and fail-closed generations.
- Explicit CPU, automatic, and required-GPU behavior.
- No daemon and no network activity in the core or SDKs.

## What it costs

- Canonical vectors, an aligned CPU scan view, and metadata indexes remain resident after preparation. The default admission limit is 512 MiB; larger collections require an explicit `StorageOptions` budget.
- WGPU adds device initialization, resident allocations, scratch, transfers, and synchronization. On the measured 100k-by-768 cell it used about 311 MB of Qenlo-owned accelerator allocation and a 1.204 GB process RSS high-water mark.
- Reopen rebuilds resident GPU state. The measured first search after reopen was 101.296 ms on one RTX 4090 workload.
- PyTorch can be fastest when CUDA is already present, but it is a large optional dependency and its tensor index is derived state, not durable storage.
- USearch is approximate. Its recall must be measured for the actual filters and tuning values.

## When to use something else

Use PostgreSQL with pgvector when vectors participate in relational joins, foreign keys, and existing database transactions. Use a distributed or managed vector database when the collection requires multi-node sharding, replication, service-level operations, or hosted ingestion. Use a standalone matrix or ANN library when persistence and canonical mutation semantics are unnecessary.

Qenlo does not implement SQL, replication, encryption, multi-node consensus, an embedding model, or a hosted service. Current Android and iOS performance is unmeasured, and release packaging for those targets is not yet verified.

## Backend choice

CPU is the conservative embedded default. WGPU is useful on tested desktop GPUs near 100k rows, but the selector revision was faster in only five of 12 qualified before/after pairs. PyTorch CUDA won the two tested 768-dimensional cells. Use explicit configuration or a small measured rule; the evidence does not justify a learned router or a universal threshold.
