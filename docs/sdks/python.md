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
with Collection.memory(dim=3) as db:
    db.add(Record(id=1, user_id=10, timestamp=1700000000, vector=(0.2, 0.8, 0.0)))
    db.add(Record(id=2, user_id=20, timestamp=1700000010, vector=(0.8, 0.1, 0.1)))

    response = db.search(
        query=(0.2, 0.7, 0.1),
        filter=Filter(user_id=10),
        top_k=5,
    )
    print(f"Matched ID: {response.results[0].id}")
```

## Anonymous Telemetry Notice

To monitor binary loader health across platforms, the Python SDK records anonymous operational metrics. No vector data, record payloads, or queries are ever collected.
