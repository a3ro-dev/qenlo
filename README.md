# Qenlo

Qenlo started with a small suspicion: filtered vector search gets described as
a database problem long before anyone measures the work a real query does.
This repository is the measurement first version of that idea.

It is an embedded Rust prototype with an exact CPU path, USearch filtered ANN,
and three controlled `wgpu` exact-search paths. The GPU code is an experiment,
not a victory lap. The bet is only worth making if a repeatable 1m × 768,
1%-eligible, batch-1, `k=10` run beats the strongest qualifying CPU path by 2×
at P95, with recall and filter correctness intact.

## workspace

- `qenlo-core`: canonical storage, predicates, metadata indexes, and exact CPU search.
- `qenlo`: public async collection API and optional `usearch` / `gpu-wgpu` backends.
- `qenlo-bench`: correctness oracle and benchmark/reporting support.

The default build stays boring. It has no C++ or GPU dependency:

```toml
qenlo = "0.1"
# opt in only where needed:
qenlo = { version = "0.1", features = ["usearch"] }
qenlo = { version = "0.1", features = ["gpu-wgpu"] }
```

The library emits spans but never installs a global subscriber. Applications
own tracing and export configuration. The OTLP HTTP/protobuf example is in
`qenlo-bench`.

## development

```powershell
cargo test --workspace --no-default-features
cargo test -p qenlo --features gpu-wgpu
cargo check -p qenlo-bench --all-features --examples
```

On the target Windows laptop, MSVC 14.29 currently crashes inside USearch's
`numkong` dependency. `clang-cl` is a verified local workaround while the C++
Build Tools installation is repaired:

```powershell
$env:CC = 'clang-cl'
$env:CXX = 'clang-cl'
cargo test -p qenlo --features usearch
```

This workaround is intentionally not forced through workspace configuration,
so downstream consumers and non-Windows targets retain their native toolchain.

## status

The correctness and device smoke tests run. Dataset preparation, competitor
benchmarks, and the 2× gate have not been run yet. Native runtime acceptance is
currently Windows DX12/Vulkan; other platforms remain untested.
