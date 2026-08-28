# contributing

start with the smallest change that has a failing check behind it. a new index
or abstraction needs evidence; fixing recovery does not need a grand redesign.

## setup and checks

use the Rust toolchain in `rust-toolchain.toml`. inspect existing changes before
editing and keep unrelated work intact. start with the portable suite:

```powershell
cargo test --workspace --no-default-features
cargo fmt --all -- --check
```

the full native checks include optional C++ and GPU code:

```powershell
$env:CC = 'clang-cl'
$env:CXX = 'clang-cl'
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo doc --workspace --all-features --no-deps
```

the compiler variables are a Windows workaround for the observed MSVC 14.29
native-dependency crash. other platforms should use their supported compiler.
if Windows incremental-cache finalization reports access denied, retry with
`$env:CARGO_INCREMENTAL = '0'` and retain the warning in the verification record.

GPU tests may skip without a device. require hardware on the RTX 4050 acceptance
host so a skipped test cannot look like a successful GPU run:

```powershell
$env:QENLO_REQUIRE_GPU = '1'
cargo test -p qenlo --features gpu-wgpu -- --nocapture
Remove-Item Env:QENLO_REQUIRE_GPU
cargo test -p qenlo --features usearch --test cpu_quality -- --nocapture
cargo test -p qenlo-bench --features otlp
```

record exact commands, toolchain, adapter, failures, skips, and measurements in
[docs/verification.md](docs/verification.md). a compiled shader is not runtime
acceptance. simulated device loss does not prove behavior on every driver.

## checks that belong with changes

- storage: restart, corruption, interrupted publication, missing acknowledged
  snapshots, admission limits, and failure/retry. preserve canonical files on
  uncertainty. never silently fall back after committed corruption.
- transactions: duplicate IDs, invalid vectors, mixed batches, rollback, and
  visibility during concurrent searches. inspect returned rows, not just a
  generation counter.
- search: an independent float64 oracle over the entire eligible subset.
  filtering an unfiltered top-k does not produce ground truth.
- telemetry: collector failure and shutdown must not alter search results.
  sensitive row/query data must stay out of default spans.
- reports: state what is unavailable or experimental. zero is not a substitute
  for a cost that was never measured.

use [the benchmark protocol](docs/benchmark-protocol.md) for performance work.
retain raw samples and configuration. distinguish synthetic fixtures from real
embedding workloads, and report recall alongside latency.

keep commits scoped to one coherent phase. do not include generated collections,
large datasets, credentials, or unrelated branding edits. pushing branches and
opening pull requests are separate actions, not part of running local checks.

## reporting problems

include a minimal input shape, backend, feature flags, toolchain, error, and
reproduction steps. use synthetic vectors and IDs. for a possible security issue,
follow [SECURITY.md](SECURITY.md) before sharing sensitive details.
