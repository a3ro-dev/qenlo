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

CPU is the default and wins when filtering leaves few candidates. Since beta.2 the exact scan runs certified FP32 at any size and splits large queries across threads. On a 12-thread laptop (100k rows, 384 dimensions, k=10) 30,000 eligible rows went from 6.6 to 3.3 ms and 100,000 from 15.2 to 6.6 ms. That is still not a bound on optimized libraries: earlier retained runs had FAISS Flat and Torch CPU roughly nine times faster on one filtered batch cell, and that gap has not been re-measured against the new scan.

### GPU portability

WGPU provides a single programming path across graphics APIs, but it does not guarantee performance or hardware availability. Adapter selection, drivers, Vulkan ICDs, shader compilation, dispatch, and readback can dominate useful work. Required mode fails explicitly; automatic mode may fall back.

### Approximate and tensor paths

USearch changes the correctness contract, so recall must accompany latency. PyTorch can be attractive when CUDA is already present, but it is a substantial optional dependency and its index is a generation-bound snapshot rather than durable state.

### Reopen and mutation

Canonical mutations can invalidate prepared eligibility and accelerator state. Reopening a collection may rebuild derived state. Existing tests cover defined recovery cases, not arbitrary concurrent or power-loss schedules.

## Routing guidance

Start with exact CPU execution. Add WGPU or tensor execution only after measuring completed calls on the target device and realistic filters. Without a tuning profile the automatic router keeps single queries on the CPU at any size and sends batches of 8 or more to the GPU: against the int8 CPU scan, a batch-1 GPU call won only at mid-sized scattered filters on a few devices and lost at E=100k on all seven measured hosts.

The September 2026 paper found route reversals, but its cohorts differ in hardware, data, and source revision. Its headline corrected WGPU cells used a selector candidate that was later rejected. Those results motivate profiling; they do not select a production default.

## Choose another system when

Use PostgreSQL with pgvector for relational data and transactions. Use a managed or distributed vector database for remote access, replication, sharding, and service operations. Use FAISS, cuVS, or another standalone library when a canonical durable store is unnecessary.
