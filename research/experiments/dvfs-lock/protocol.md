# E-LOCK protocol: does a locked clock remove the light kernel's variation?

Written 2026-10-19, before any E-LOCK data. Derived from
`research/specs/2026-09-25-clock-lock-and-faiss.md`; where the two differ, this file
governs. The pre-run probe below is the only measurement taken before this file was hashed.

## Question
If both GPU paths run at fixed SM and memory clocks, does the light kernel's
process-to-process variation disappear, and does it run at its best unlocked speed?
The paper (`paper/v2/`) infers a power-state cause from telemetry and the heater
intervention; this experiment tests it directly.

## Pre-run probe (2026-10-19, unprivileged shell)
`nvidia-smi -q -d SUPPORTED_CLOCKS`: memory 405 / 810 / 6001 / 7001 / 8001 MHz;
SM 210-3105 MHz in 15 MHz steps (585 and 2670 present). `nvidia-smi -lgc` from a
non-admin shell: "does not have permission" (expected; the run uses an elevated shell).
GPU idle at P8, 40 C. One unlocked pipeline smoke run (gpu-rows, E=1,000, heater on, scratch
directory outside the data tree) completed with rc 0 and a 42 C peak; it is not E-LOCK data
and is not analyzed.

## Host and binary
Windows 11 laptop, i5-13420H, RTX 4050 Laptop GPU, driver 616.56, Vulkan via WGPU, AC power.
Binary: the D1 binary, unchanged and not rebuilt:
`target/release/qenlo-bench.exe` sha256 `4f49932c080b2d37ee5b9219eba12e80c53b881ad79749671150ee1565d086c4`,
`target/release/examples/gpu_heater.exe` sha256 `6fa819de50c8233fd970420d2f997e7d9e869abd7f4f35b07284aabaa000269a`.
The runner refuses to start if either hash differs.
Data: `data/ag-news/ag-news-100k-384.qnb` (as D1). B=1, k=10, 200 warm-ups, 3 x 5,000 queries,
fresh process per run, recall target 0.99. Per-run cell code is `research/scripts/run_dvfs_cell.py`, unchanged from D1.

## Design
- Engines: `gpu-rows` (light), `gpu-predicate` (heavy). E in {1,000; 3,000; 10,000}.
- Clock conditions:
  - `U`: unlocked (driver default), as in D1.
  - `Lhi`: `nvidia-smi -lgc 2670,2670` and `-lmc 8001,8001` (the heavy path's observed boost in D1; max memory).
  - `Llo`: `nvidia-smi -lgc 585,585` and `-lmc 810,810` (the light path's D1 state at E=10,000).
- Heater (500 iters, 1 ms sleep, as D1) is crossed with `U` and `Lhi` only:
  conditions U-h0, U-h1, Lhi-h0, Lhi-h1, Llo-h0.
- 6 cells x 5 conditions = 30 runs per block; 5 blocks = 150 runs.
  Run order within block shuffled with `random.Random(4000 + block)`; bench order seed 9301 + block.
- Locks are applied before the run's process starts and reset (`-rgc`, `-rmc`) after it ends,
  so every run starts from a known state. 5 s pause between runs.
- If `-lmc` is rejected at preflight, the run proceeds with SM-only locks, records this in
  `preflight.json`, and K3 is reported as not testable.

## Telemetry and safety
- 50 ms `nvidia-smi` log per run (SM, memory clock, P-state, utilization, power, temperature), as D1.
- A 1 s poller logs `clocks_event_reasons.active` and the applied clocks to `poll.csv` per run.
- Each run's `lock.json` records requested clocks, `nvidia-smi -lgc/-lmc` return codes, and
  `nvidia-smi -q -d CLOCK,PERFORMANCE` before the run.
- Thermal guard: a run starts only when the GPU is at or below 70 C (waits up to 10 min).
  If the GPU reaches 90 C, clocks are reset immediately, the current run is marked
  `thermal_abort`, and the runner stops. The runner also stops if the laptop leaves AC power.
- Clocks are reset on every exit path (normal end, error, Ctrl+C).

## Metric
Per run: median of `device_selection_ns` over all timed calls (the top-k kernel; same metric
as paper Table 1). Secondary: `device_scoring_ns`, end-to-end P50 (`batch_latency_ns`),
`row_materialization_ns`. Runs with `thermal_abort` or a non-zero return code are excluded
and reported.

## Predictions (confirmatory)
- K1. Under Lhi-h0, the light kernel's device-time max/min across blocks is <= 1.15 at every E
  (D1 unlocked laptop: up to 4.16).
- K2. Under Lhi-h0, the light kernel's median device time is within 15% of the best value
  observed for that E in this experiment's U-h0 and U-h1 runs.
- K3. At E=10,000 only, under Llo-h0, the light kernel's median device time is within 25% of the
  D1 laptop heater-off median (0.461 ms). (Llo is D1's observed state only at E=10,000; at
  E=1,000 and 3,000 Llo is reported descriptively.)
- K4. Median over blocks of the heater-on/off light device-time ratio is >= 0.95 under Lhi at
  every E. Under U it is < 0.9 at E=3,000 and E=10,000 (descriptive replication of D1).
- K5. The heavy kernel's median device time under Lhi-h0 is within 3% of its U-h0 median at every E.

## Rejection criteria
- K1 fails with a light max/min >= 2 at locked clocks: the variation is not explained by
  power state, and the paper's mechanism claim is withdrawn.
- K1 fails with max/min between 1.15 and 2: power state explains part of the variation; the
  paper says so and quantifies the residual.
- K1 holds but K2 fails: clocks explain the variance but not the level; the "understates the
  efficient kernel" claim is limited to variance.
- K3 fails: the low clock state alone does not reproduce the D1 penalty; something else
  co-occurs with it (e.g. PCIe link state). Report `pcie.link.gen.current` if available.
- Any verified lock failure (observed SM clock while GPU-active differs from the requested
  clock by more than 5% in a locked run) excludes that run and is reported.

## Outputs
`research/data/raw/2026-10-19-dvfs-lock/block-<b>/<engine>-e<E>-<cond>/` with the D1 layout
plus `lock.json` and `poll.csv`; `preflight.json` and `runner.log` at the top level.
Analysis: `research/scripts/analyze_dvfs_lock.py`; paper table `paper/v2/tables/lock.tex`.
