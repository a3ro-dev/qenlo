# Quickstart

Qenlo alphas are published to crates.io (`qenlo`), PyPI (`qenlo`), and npm
(`@a3ro.dev/qenlo`, dist-tag `alpha`). Install commands with the current version
are in the [Rust](sdks/rust.md), [Python](sdks/python.md), and
[TypeScript](sdks/typescript.md) guides. The full Rust API reference is on
[docs.rs](https://docs.rs/qenlo/latest/qenlo/struct.Collection.html), and the
terminal browser's `?` tab lists every `Collection` method with an example.

## Rust from this checkout

```powershell
cargo run -p qenlo --example quickstart -- ./demo.qenlo cpu
$env:WGPU_BACKEND = 'dx12'
cargo run -p qenlo --features gpu-wgpu --example quickstart -- ./gpu-demo.qenlo gpu
```

Use a new directory. The example adds records, runs a filtered cosine search, deletes a row, closes the collection, and verifies state after reopen. Required GPU mode fails if the requested adapter cannot execute it.

## Minimal Rust API

```rust
use qenlo::{Collection, CollectionConfig, Filter};

async fn search() -> Result<(), qenlo::Error> {
    let db = Collection::new(CollectionConfig::cpu_exact(3)).await?;
    db.add(1, 42, 1_700_000_000, &[0.1, 0.8, 0.5])?;
    let response = db.search(&[0.1, 0.7, 0.5], &Filter::ALL, 5).await?;
    println!("{:?}", response.results);
    Ok(())
}
```

Collections above the default admission limit need an explicit budget, including in-memory benchmark collections:

```rust
use qenlo::{Collection, CollectionConfig, StorageOptions};

let db = Collection::new_with_options(
    CollectionConfig::cpu_exact(768),
    StorageOptions { max_load_bytes: 2 * 1024 * 1024 * 1024 },
).await?;
```

See the language-specific pages under `docs/sdks/` for source-checkout examples. Do not advertise an SDK backend unless its packaged native artifact contains that feature.
