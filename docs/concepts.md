# Concepts

## Canonical and derived state

The canonical collection contains normalized FP32 vectors, public IDs, user IDs, signed timestamps, liveness, and a generation number. This state determines which records exist.

CPU scan layouts, WGPU buffers, ANN indexes, tensor snapshots, and eligibility plans are derived. They may be rebuilt or discarded, but they must not redefine canonical membership.

## Eligibility

A filter selects live records by optional user equality and a timestamp range whose lower bound is inclusive and upper bound is exclusive. The number of selected records is the eligible cardinality, `E`.

Search cost is influenced by more than total collection size `N`. Eligible work, dimension, batch size, predicate representation, preparation, selection, transfers, and device state can all affect the winning execution path.

An eligibility plan binds a prepared predicate result to a canonical generation. Mutations advance the generation and invalidate plans that no longer describe current state.

## Exact search

Qenlo uses “exact” to mean exhaustive coverage of every eligible candidate. It does not mean every backend is bit-identical to an ideal FP64 computation. Stored FP32 values and different accumulation orders can change rankings near numerical ties.

Correctness evaluation therefore keeps exhaustive coverage and FP64-oracle recall as separate properties.

## Approximate search

USearch HNSW visits only part of the eligible search space. Its latency is meaningful only beside recall for the same data, filters, and tuning. A measured recall of 1.0 on one workload does not make an ANN method universally exact.

## Routing

CPU, WGPU, and tensor execution have different fixed and variable costs. CPU often fits tiny eligible sets; accelerators can help when enough scoring work or batching amortizes dispatch and transfer overhead.

Qenlo exposes explicit route selection and a hardware-bound profile mechanism. The current research archive does not validate an adaptive router on held-out workloads, so the fallback threshold should not be treated as a universal crossover.

## Completed-call timing

The native timing boundary begins at the host search call and ends with ordered, host-visible results. It can include eligibility work, transfers, dispatch, synchronization, readback, and merging. Isolated GPU phase timings may overlap and must not be summed as if they were disjoint.

## Durability boundary

File-backed collections publish validated mutations through snapshots, WAL state, and a generation watermark. In-memory collections do not have that durability boundary. Recovery tests demonstrate named scenarios; they do not prove behavior for every filesystem or crash schedule.

## Evidence boundary

A benchmark observation belongs to its source revision, hardware, software environment, data, queries, filter, and timing definition. Results from different cohorts are useful context but are not interchangeable replications. See the [benchmark protocol](benchmark-protocol.md) and [verification notes](verification.md).
