# Trade-offs

Qenlo combines a durable embedded store with replaceable search paths. That keeps ownership and semantics local, but it does not make every path the fastest choice.

## What Qenlo provides

- One canonical record model across CPU, WGPU, ANN, and tensor views.
- Exact filtered search with deterministic distance-then-ID ordering.
- Atomic mutations, tombstones, checksummed persistence, and fail-closed recovery.
- Explicit required-GPU and automatic-fallback behavior.
- Route, preparation, transfer, allocation, and failure reports.
- No daemon and no automatic network activity.

## Costs and limits

### Memory

Canonical vectors, metadata indexes, and prepared scan layouts consume host memory. GPU execution adds resident vectors, scratch buffers, transfers, and synchronization. Large collections require explicit storage and accelerator budgets.

### CPU performance

CPU is the conservative default and often wins when filtering leaves very few candidates. Qenlo's measured CPU implementation is not a bound on optimized libraries: in one corrected filtered batch cell, FAISS Flat and Torch CPU were roughly nine times faster than Qenlo CPU. General CPU/GPU crossover claims therefore require a stronger CPU baseline.

### GPU portability

WGPU provides a single programming path across graphics APIs, but it does not guarantee performance or hardware availability. Adapter selection, drivers, Vulkan ICDs, shader compilation, dispatch, and readback can dominate useful work. Required mode fails explicitly; automatic mode may fall back.

### Approximate and tensor paths

USearch changes the correctness contract, so recall must accompany latency. PyTorch can be attractive when CUDA is already present, but it is a substantial optional dependency and its index is a generation-bound snapshot rather than durable state.

### Reopen and mutation

Canonical mutations can invalidate prepared eligibility and accelerator state. Reopening a collection may rebuild derived state. Existing tests cover defined recovery cases, not arbitrary concurrent or power-loss schedules.

## Routing guidance

Start with exact CPU execution. Add WGPU or tensor execution only after measuring completed calls on the target device and realistic filters. Treat the built-in automatic threshold as a fallback, not a learned or transferable router.

The September 2026 paper found route reversals, but its cohorts differ in hardware, data, and source revision. Its headline corrected WGPU cells used a selector candidate that was later rejected. Those results motivate profiling; they do not select a production default.

## Choose another system when

Use PostgreSQL with pgvector for relational data and transactions. Use a managed or distributed vector database for remote access, replication, sharding, and service operations. Use FAISS, cuVS, or another standalone library when a canonical durable store is unnecessary.
