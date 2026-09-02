# Core Concepts

Understand the data model, storage mechanisms, and filtering rules in Qenlo.

---

## Data Model

Every item stored in Qenlo is a `Record`:

* **`id`** (`u64`): Unique record identifier. IDs are non-reusable once deleted.
* **`user_id`** (`u64`): Primary partition / tenant identifier for security boundary enforcement.
* **`timestamp`** (`i64`): Signed Unix timestamp in seconds for temporal range queries and expiration.
* **`vector`** (`[f32; dim]`): FP32 vector representation. Automatically normalized to unit length for cosine distance computation.

---

## Filter Semantics

Unlike traditional vector databases that compute approximate nearest neighbors and then filter (causing recall drop), Qenlo evaluates scalar filters **before** scoring:

```rust
pub struct Filter {
    pub user_id: Option<u64>,
    pub timestamp_min: Option<i64>,
    pub timestamp_max: Option<i64>,
}
```

* **`user_id`**: Matches records with the exact same user ID.
* **`timestamp_min`** / **`timestamp_max`**: Half-open interval `[min, max)` matching records in that range.
* **Combined Filters**: When multiple clauses are specified, they are combined with strict boolean `AND`.

---

## Storage Modes

1. **In-Memory**: Transient workspace ideal for unit tests, ephemeral worker processes, and client-side web sessions.
2. **Durable Directory**: Write-ahead log (WAL) and segment snapshots stored on disk with checksum verification and recovery.
3. **`.qn` Portable Snapshots**: Single-file, standalone export that can be distributed across machines or embedded into mobile assets.

---

## Execution Reports

Every search query returns an `ExecutionReport` detailing:

* **Engine**: The path used (`CpuExact`, `CpuSimd`, `WgpuCompute`, `Cuda`).
* **Scanned Candidates**: Total records considered.
* **Filtered Count**: Records passing scalar filter constraints.
* **Elapsed Time**: Nanoseconds spent in filtering vs vector scoring.
