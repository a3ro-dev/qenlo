# Quickstart

Get started with Qenlo in your language of choice in under 5 minutes.

---

## Installation

### Python
```bash
pip install qenlo
```

### TypeScript / Node.js
```bash
pnpm add @qenlo/qenlo
# or npm install @qenlo/qenlo
```

### Rust
```toml
[dependencies]
qenlo = "0.1.0-alpha.1"
```

### Go
```bash
go get github.com/a3ro-dev/qenlo/sdk/go
```

---

## Code Examples

### Python

```python
from qenlo import Collection, Filter, Record

# Open an in-memory collection with vector dimension = 3
with Collection.memory(dim=3) as db:
    # Insert records (id, user_id, timestamp, vector)
    db.add(Record(id=1, user_id=42, timestamp=1700000000, vector=(0.1, 0.8, 0.5)))
    db.add(Record(id=2, user_id=99, timestamp=1700000050, vector=(0.9, 0.1, 0.2)))

    # Search with exact scalar filter
    query = (0.1, 0.7, 0.5)
    response = db.search(
        query=query,
        filter=Filter(user_id=42),
        top_k=5,
    )

    for match in response.results:
        print(f"ID: {match.id}, Score: {match.score:.4f}")
```

### TypeScript / Node.js

```typescript
import { Collection } from "@qenlo/qenlo";

// Open in-memory collection
using db = Collection.memory(3);

// Add records
db.add({
  id: 1n,
  userId: 42n,
  timestamp: 1700000000n,
  vector: [0.1, 0.8, 0.5],
});

// Search
const response = db.search([0.1, 0.7, 0.5], { userId: 42n }, 5);
console.log(`Matched ID: ${response.results[0]?.id}`);
```

### Rust

```rust
use qenlo::{Collection, CollectionConfig, Filter, Record};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Collection::new(CollectionConfig::cpu_exact(3)).await?;
    
    db.add(1, 42, 1700000000, &[0.1, 0.8, 0.5])?;
    
    let filter = Filter {
        user_id: Some(42),
        ..Filter::ALL
    };
    
    let response = db.search(&[0.1, 0.7, 0.5], &filter, 5).await?;
    println!("Found {} results", response.results.len());
    Ok(())
}
```

---

## Next Steps

* Learn more about [Core Concepts](concepts.md).
* Check out [Storage and .qn exports](qn-format-v1.md).
* Explore the [Python SDK Reference](sdks/python.md) or [TypeScript Reference](sdks/typescript.md).
