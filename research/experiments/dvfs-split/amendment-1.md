# E-SPLIT amendment 1 (2026-10-25, before any valid E-SPLIT data)

The binary registered in `protocol.md` (sha256 `602a61c1...`) was built without the
`gpu-wgpu` cargo feature, so it has no GPU backend. The first run attempt (13:04-13:05)
failed every process at startup with `qenlo-bench: unknown or feature-disabled backend`
(rc 1, about 6 s each, no measurements). The runner was stopped after 10 processes.

Change: the binary is rebuilt from the same commit (2a64473) with
`cargo build --release -p qenlo-bench --features gpu-wgpu`:
`target/release/qenlo-bench.exe` sha256 `1d0c143e702e771d152b5ab0691ce7f264b5e5465f89fc37b0e66b16889b461b`.
An unlocked smoke run of one cell (gpu-rows, E=10,000, 1 pass, scratch directory outside the
data tree) completed with rc 0 on the RTX 4050; it is not E-SPLIT data.

The failed attempt is kept, unanalyzed, in
`research/data/raw/2026-10-25-dvfs-split-failed-no-gpu-feature/`. Nothing else in the
protocol changes: design, predictions, exclusions and analysis are as registered.
