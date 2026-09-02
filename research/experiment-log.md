# Experiment log

| UTC | Revision | Machine | Command/workload | Output | Status/observation |
|---|---|---|---|---|---|
| 2026-09-02 | `3e2a4a9` | Windows local | `cargo test --workspace --no-default-features -- --test-threads=1` | terminal | pass: 84 tests plus doctests |
| 2026-09-02 | `3e2a4a9` + harness edit | Windows local | `cargo test -p qenlo-bench --no-default-features` | terminal | pass |
| 2026-09-02 | `3e2a4a9` | Windows local | `cargo test -p qenlo --features gpu-wgpu -- --test-threads=1` | terminal | pass: 49 tests/doctests |
| 2026-09-02 | `3e2a4a9` | Windows local | `cargo test -p qenlo --features usearch -- --test-threads=1` | terminal | inconclusive: upstream C++ compile stalled and was interrupted |
| 2026-09-02 | `3e2a4a9` | Runpod RTX A6000 | Rust/WGPU smoke, Vulkan and GL | retained failure evidence | no usable NVIDIA graphics adapter; zero performance samples |
| 2026-09-02 | `3e2a4a9` | Runpod RTX A6000 | TopK public parquet download | terminal | aborted: 5.56 GiB transfer rate would violate economical campaign intent; partial remote file deleted with pod |
| 2026-09-02 | worktree | Runpod RTX A6000, driver 570.195.03 | CUDA synthetic 1M×768, E=1k/10k/100k/1M, seed 20260902 | `data/raw/2026-09-02-a6000-cuda/tuning` | complete, five repetitions |
| 2026-09-02 | worktree | same | same, fresh seed 20260903 | `data/raw/2026-09-02-a6000-cuda/heldout` | complete; minimum Qenlo/FAISS top-10 agreement 1.0 |
| 2026-09-02 | — | Runpod account | terminate all pods; query billing | API record | zero pods remain; day campaign spend $0.9926765 |
| 2026-09-02 | `3e2a4a9` + harness edit | Windows RTX 4050/Vulkan | native CPU and GPU-row sweep, E=2k/3k/4k/6k/8k/10k, 100k×384, batch 1 | `research/data/raw/2026-09-02-native-crossover` | complete; 5,000 held-out queries × 5 runs/cell, recall@10=1; winner reversal bracketed to (2k,3k) |
| 2026-09-02 | app 0.1.0 | Android 16, MT6897/Mali-G615 MC6/Vulkan | author-supplied quick/full/soak schema-v1 records | `research/data/processed/android_device_lab.csv` | transcribed; 21/21 cells passed with recall@10=1 and no fallback; soak contains 512 samples/cell |

One initial `cpu-e2000` invocation failed before measurement because its parent output directory did not exist. The directory was created and the complete command was rerun; the failed invocation contributed no samples.

The earlier failed pod/image experiments and the retained benchmark archive remain in `benchmark-results/2026-09-02-runpod-archive` and the strict TopK cohort in `benchmark-results/2026-09-02-runpod-a6000-strict-research-gate`.
