# Python SDK

Type-safe Python bindings for Qenlo powered by precompiled binary wheels.

## Installation

```bash
pip install qenlo
```

## Quick Example

```python
from qenlo import Collection, Filter, Record

# In-memory collection
with Collection.memory(dimension=3) as db:
    db.add(Record(id=1, user_id=10, timestamp=1700000000, vector=(0.2, 0.8, 0.0)))
    db.add(Record(id=2, user_id=20, timestamp=1700000010, vector=(0.8, 0.1, 0.1)))

    response = db.search(
        query=(0.2, 0.7, 0.1),
        filter=Filter(user_id=10),
        k=5,
    )
    print(f"Matched ID: {response.results[0].id}")
```

## Bulk ingestion and optional tensors

`Collection.add_buffer` accepts a C-contiguous native `float32` matrix plus
parallel ID, user-ID, and timestamp arrays. It performs one native batch commit.
Read-only matrix exporters require one bulk copy; writable exporters are borrowed
until the call returns.

Install `qenlo[torch]` to use `TorchIndex`. `TorchIndex.from_collection` captures
filtered live rows through the typed native ABI, binds the index to that canonical
generation, and rejects searches after a mutation. `cpu`, `cuda`, and `mps` are
the supported device types. PyTorch remains lazily imported and optional.

The tensor backend currently supports IDs through `2**63 - 1`; the durable core
continues to support the full unsigned 64-bit range. This limit is checked before
construction because eager `uint64` operation coverage differs across PyTorch
backends. `max_bytes` is a checked lower bound for explicit index/search tensors,
not process RSS or allocator-cache usage.

## Background work and networking

Importing or using the Python SDK starts no background thread and sends no
network request. Telemetry export belongs to the host application.
