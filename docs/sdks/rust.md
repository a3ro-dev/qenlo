# Rust API Reference

The core `qenlo` crate provides high-performance, asynchronous, in-process vector search with zero external service dependencies.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
qenlo = "0.1.0-alpha.1"
tokio = { version = "1", features = ["full"] }
```

## Basic Usage

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
        println!("ID: {}, Score: {}", match_record.id, match_record.score);
    }

    Ok(())
}
```
