# Python SDK

The Python SDK provides typed bindings for Qenlo using precompiled binary wheels.

## Installation

```bash
pip install qenlo
```

## Quick example

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

`Collection.add_buffer` accepts a C-contiguous native `float32` matrix along with parallel arrays for ID, user ID, and timestamp values. It performs a single native batch commit. Read-only matrix exporters require one bulk copy, while writable exporters are borrowed until the call returns.

Install `qenlo[torch]` to use `TorchIndex`. Calling `TorchIndex.from_collection` captures filtered live rows through the typed native ABI, binds the index to that specific generation, and rejects searches after any mutation. Supported device targets include `cpu`, `cuda`, and `mps`. The package imports PyTorch lazily only when you use tensor features.

The tensor backend supports IDs up to `2**63 - 1`, whereas the durable core supports the full unsigned 64-bit range. Qenlo checks this limit during construction because PyTorch backends vary in their eager `uint64` coverage. The `max_bytes` setting acts as a lower bound for explicit index and search tensors, not as a cap on process RSS or allocator cache usage.

## Background work and networking

Importing or using the Python SDK starts no background threads and makes no network requests. If your application needs telemetry, export it from your own application layer.
