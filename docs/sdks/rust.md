# Rust API reference

`qenlo` is an asynchronous, in-process vector database for Rust. It runs locally in your application without external service dependencies.

## Installation

Add `qenlo` to `Cargo.toml`:

```toml
[dependencies]
qenlo = "0.1.0-alpha.5"
tokio = { version = "1", features = ["full"] }
```

## Basic usage

```rust
use qenlo::{Collection, CollectionConfig, Filter, Record};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize in-memory collection
    let config = CollectionConfig::cpu_exact(3);
    let db = Collection::new(config).await?;

    // Insert records
    db.add(1, 100, 1700000000, &[1.0, 0.0, 0.0])?;
    db.add(2, 200, 1700000050, &[0.0, 1.0, 0.0])?;

    // Perform filtered vector search
    let filter = Filter {
        user_id: Some(100),
        ..Filter::ALL
    };

    let response = db.search(&[0.9, 0.1, 0.0], &filter, 5).await?;
    for match_record in response.results {
        println!("ID: {}, Distance: {}", match_record.id, match_record.distance);
    }

    Ok(())
}
```
