# Qenlo Python SDK

Type-safe Python bindings for **Qenlo** — the embedded, durable vector database written in Rust.

Qenlo provides exact filtered cosine vector search with atomic commits, write-ahead logging (WAL), portable `.qn` snapshot files, and zero external database services. Every search returns an execution report containing routing decisions, memory allocations, and hardware execution telemetry.

## Installation

```bash
pip install qenlo
```

Pre-built binary wheels bundle the native Rust engine for:
- Linux (`x86_64`, `aarch64`)
- macOS (`Apple Silicon arm64`, `Intel x86_64`)
- Windows (`x86_64`)

For source checkouts or development builds, set `QENLO_LIBRARY_PATH` to point to your compiled `qenlo_ffi.dll`, `libqenlo_ffi.so`, or `libqenlo_ffi.dylib`.

---

## Quickstart

### In-Memory Collection

```python
from qenlo import Collection, Filter, Record

# Create an in-memory collection with 3-dimensional vectors
with Collection.memory(dimension=3) as db:
    # Insert records
    db.add(Record(id=1, user_id=42, timestamp=100, vector=(1.0, 0.0, 0.0)))
    db.add(Record(id=2, user_id=42, timestamp=200, vector=(0.0, 1.0, 0.0)))
    db.add(Record(id=3, user_id=99, timestamp=150, vector=(0.7, 0.7, 0.0)))

    # Search with combined user and timestamp filters
    response = db.search(
        query=(1.0, 0.0, 0.0),
        filter=Filter(user_id=42, timestamp_lower=50, timestamp_upper=150),
        k=5,
    )

    for hit in response.results:
        print(f"ID: {hit.id}, Cosine Distance: {hit.distance:.4f}")

    # Inspect hardware and routing telemetry
    report = response.report
    print(f"Backend: {report.actual_backend}, Algorithm: {report.algorithm}")
    print(f"Total Duration: {report.total_duration_ns} ns")
```

---

## Durable Storage & Restarts

Qenlo collections can be persisted to disk with crash-safe write-ahead logging (WAL) and atomic compaction:

```python
from qenlo import Collection, Record, Filter

path = "./my_collection.qenlo"

# 1. Create a new durable collection directory
with Collection.create(path, dimension=128) as db:
    db.add(Record(id=1, user_id=7, timestamp=10, vector=my_vector))
    db.flush()  # Compact and ensure full disk sync

# 2. Reopen across application restarts
with Collection.open(path, dimension=128) as db:
    response = db.search(query=my_query, filter=Filter(user_id=7), k=10)
    print(f"Found {len(response.results)} matches")
```

---

## Portable `.qn` Interchange Files

Export and import standalone, checksummed, immutable `.qn` snapshots:

```python
# Export an existing collection to a .qn file
db.export_qn("snapshots/v1.qn")

# Import from a .qn file into a fast in-memory collection
with Collection.import_qn("snapshots/v1.qn", dimension=128) as snapshot_db:
    stats = snapshot_db.stats()
    print(f"Loaded {stats.live_rows} rows from generation {stats.generation}")
```

---

## Batch Operations

Qenlo supports high-throughput atomic batch mutations:

```python
records = [
    Record(id=10, user_id=1, timestamp=1000, vector=(0.1, 0.2, 0.3)),
    Record(id=11, user_id=1, timestamp=1001, vector=(0.4, 0.5, 0.6)),
    Record(id=12, user_id=2, timestamp=1002, vector=(0.7, 0.8, 0.9)),
]

# Insert all atomically (all-or-nothing validation)
db.add_batch(records)

# Delete multiple records by ID
db.delete_batch([10, 11])
```

---

## Data Model & Types

### `Record`
- `id`: `int` (unsigned 64-bit integer, unique and non-reusable)
- `user_id`: `int` (unsigned 64-bit integer)
- `timestamp`: `int` (signed 64-bit integer)
- `vector`: `Sequence[float]` (normalized FP32 components)

### `Filter`
- `user_id`: `Optional[int]` (exact equality match)
- `timestamp_lower`: `Optional[int]` (inclusive lower bound)
- `timestamp_upper`: `Optional[int]` (exclusive upper bound)

### `ExecutionReport`
- `operation_id`: `int` — Unique monotonically increasing query ID
- `requested_backend`: `str` — `Cpu`, `GpuPredicate`, or `Automatic`
- `actual_backend`: `str` — Hardware engine that executed the search
- `algorithm`: `str` — Search algorithm (`Exact`, `IvfFlat`, etc.)
- `filter_execution`: `str` — Filter strategy evaluated
- `index_generation`: `int` — Generation watermark observed
- `total_duration_ns`: `int` — Total wall-clock time in nanoseconds
- `lock_wait_ns`: `int` — Time spent acquiring read locks
- `eligible_rows`: `Optional[int]` — Number of live rows passing metadata filters
- `upload_bytes`: `Optional[int]` — Host-to-device bytes transferred
- `readback_bytes`: `Optional[int]` — Device-to-host bytes read back

---

## Error Handling

All native and validation failures raise `QenloError` or standard Python exceptions (`ValueError`):

```python
from qenlo import Collection, QenloError, Record

try:
    with Collection.memory(3) as db:
        db.add(Record(1, 1, 0, (1.0, 0.0, 0.0)))
        # Duplicate IDs are strictly rejected
        db.add(Record(1, 1, 0, (0.0, 1.0, 0.0)))
except QenloError as e:
    print(f"Operation rejected: {e}")
```

---

## License

Dual-licensed under **MIT** or **Apache-2.0** at your option.

