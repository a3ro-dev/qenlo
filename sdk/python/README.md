# Qenlo Python

Typed Python bindings for Qenlo's embedded, durable vector database. The wheel
contains the Rust native library for its target platform. Source checkouts can
set `QENLO_LIBRARY_PATH` to a locally built library.

```python
from qenlo import Collection, Filter, Record

with Collection.memory(3) as db:
    db.add(Record(id=1, user_id=7, timestamp=10, vector=(1.0, 0.0, 0.0)))
    response = db.search((1.0, 0.0, 0.0), Filter(user_id=7), k=10)
    assert response.results[0].id == 1
```

`Collection.create(path, dimension)` creates durable state in a new directory.
Use `Collection.open(path, dimension)` after restart. Filters combine optional
user equality with a lower-inclusive, upper-exclusive timestamp range.
