# Quickstart

Create a collection, add records, search with a filter, and reopen it. Every example uses 3-dimensional vectors so you can read them; use your embedding model's dimension in practice.

A record is `(id, user_id, timestamp, vector)`. A filter selects live records by optional `user_id` and a timestamp range (start inclusive, end exclusive), and search ranks only those records.

## Rust

```bash
cargo add qenlo
cargo add tokio --features full
```

```rust
use qenlo::{Collection, CollectionConfig, Filter, NewRecord, TimestampRange};

#[tokio::main]
async fn main() -> Result<(), qenlo::Error> {
    let config = CollectionConfig::cpu_exact(3);
    let db = Collection::create("./notes.qenlo", config.clone()).await?;

    db.add_batch(&[
        NewRecord { id: 1, user_id: 7, timestamp: 10, vector: vec![1.0, 0.0, 0.0] },
        NewRecord { id: 2, user_id: 7, timestamp: 20, vector: vec![0.0, 1.0, 0.0] },
    ])?;

    let filter = Filter::new(Some(7), TimestampRange::new(Some(0), Some(30)));
    let response = db.search(&[1.0, 0.0, 0.0], &filter, 10).await?;
    assert_eq!(response.results[0].id, 1);

    db.delete(1)?;
    db.close()?;

    let reopened = Collection::open("./notes.qenlo", config).await?;
    assert_eq!(reopened.filter(&Filter::ALL), vec![2]);
    Ok(())
}
```

Use a new path for `create`. Collections larger than the default admission limit need an explicit budget through `Collection::new_with_options` and `StorageOptions { max_load_bytes }`.

## Python

```bash
pip install "qenlo==0.1.0b2"
```

```python
from qenlo import Collection, Filter, Record

with Collection.memory(dimension=3) as db:
    db.add(Record(id=1, user_id=7, timestamp=10, vector=(1.0, 0.0, 0.0)))
    db.add(Record(id=2, user_id=7, timestamp=20, vector=(0.0, 1.0, 0.0)))
    response = db.search(query=(1.0, 0.0, 0.0), filter=Filter(user_id=7), k=5)
    print(response.results[0].id)
```

## TypeScript

```bash
npm install @a3ro.dev/qenlo@beta
```

```typescript
import { Collection } from "@a3ro.dev/qenlo";

using db = Collection.memory(3);
db.add({ id: 1n, userId: 7n, timestamp: 10n, vector: [1, 0, 0] });
const response = db.search([1, 0, 0], { userId: 7n }, 5);
console.log(response.results[0]?.id);
```

## Run the bundled example

From a checkout of the repository:

```bash
cargo run -p qenlo --example quickstart -- ./demo.qenlo cpu
```

It adds records, runs a filtered cosine search, deletes a row, closes the collection and verifies state after reopen. Add `--features gpu-wgpu` and pass `gpu` instead of `cpu` to require the GPU path; it fails rather than silently falling back if the adapter cannot run it.

## Next

- [Core concepts](concepts.md) for filters, exactness and the execution report.
- [Collection browser](browser.md) to look inside a collection.
- Per-language details: [Rust](sdks/rust.md), [Python](sdks/python.md), [TypeScript](sdks/typescript.md). Full Rust API reference on [docs.rs](https://docs.rs/qenlo).
