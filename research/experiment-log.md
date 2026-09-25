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

| UTC | Revision | Machine | Command/workload | Output | Status/observation |
|---|---|---|---|---|---|
| 2026-09-23 | `e5d85ad` | none (read-only) | `python research/scripts/audit_router_noise.py` over the alpha.5 held-out archive | `research/data/processed/router-noise-audit` | automatic-vs-same-backend A/A differs >25% in 9/16 cells; automatic GPU route used ShaderPredicate while the regret counterfactual was gpu-rows |
| 2026-09-23 | `e5d85ad` | Windows i5-13420H / RTX 4050 DX12 | CPU E=3000 100k×384 B=1, 3 reps, affinity P-core (0x1) vs E-core (0x100) vs unpinned; gpu-rows/gpu-predicate/automatic smoke | `research/data/raw/2026-09-23-affinity-smoke` | CONTAMINATED smoke: an untracked concurrent process (research/scripts/run_decision_queue.py, not this session) wrote smoke checkpoints 23:19:32–23:21:28 local, overlapping all runs except cpu-e3000-pcore; directional only. E-core P95 1.3–1.5× P-core; same-affinity drift 1.5×; gpu-predicate ≈3× gpu-rows P50; recall@10=1 everywhere |
| 2026-09-24 | `e5d85ad` (attested) | Runpod RTX 4090 / Linux Vulkan | `research/scripts/run_e0_e2_runpod.py`: E0 noise floor (B=1/E=3000, 10 blocks, CPU vs gpu-rows) and E2 representation×batch ablation (4 engines, B∈{1,16}, E∈{100…100000}, 5 blocks) | `research/data/raw/runpod-e0-e2-partial-20260924.tar.gz` (SHA-256 `e4ca1336…fd51ed0`) | PARTIAL: 227/260 summaries (E0 20/20, E2 207/240), no COMPLETE marker; 13 missing at B16/E30000, 20 at B16/E100000; b1-e100 block-02 gpu-mask exit 101 with summary (runner halted, resume skipped it); b16-e30000 block-01 cpu has no summary; 20 summaries recall@10=0.99998 (all engines at B1/E100000) |
| 2026-09-24 | working tree over `e5d85ad` | none (read-only) | `python research/scripts/audit_e0_e2_mechanisms.py` over the partial archive | `research/data/processed/runpod-e0-e2-mechanisms-20260924` | independent re-verification of coverage/exit codes/recall; per-call diagnostics; clock-snapshot association 80/31/114; same-host EDB ordering witness; found harness B× overcount of batch transfer counters |
| 2026-09-24 | working tree (bench v4 + range-filter probe/scan) vs clean `e5d85ad` worktree | Windows i5-13420H / RTX 4050 Laptop, Vulkan | `python research/scripts/run_local_filter_ab.py` — interleaved ABAB, B=1, gpu-rows E∈{1000,10000,30000}, cpu E=30000, 3 reps × 5000 queries | `research/data/raw/2026-09-24-local-filter-ab` | local laptop only; not comparable to the E0/E2 host; see `ab_summary.csv` |
| 2026-09-25 | working tree | Windows i5-13420H / RTX 4050 Laptop | `cargo test -p qenlo-core`; `cargo test -p qenlo --features gpu-wgpu`; `cargo test -p qenlo-bench --features gpu-wgpu`; runner resume validation probe | terminal | pass: core, Qenlo/WGPU, benchmark, integration, and doctest suites; required predicate batches avoid host row lists; no performance claim |
