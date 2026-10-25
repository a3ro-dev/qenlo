# E-SPLIT protocol: SM clock or memory clock?

Written 2026-10-25, before any E-SPLIT data. Follows E-LOCK
(`research/experiments/dvfs-lock/protocol.md`); where the two differ, this file governs.

## Question
E-LOCK showed that fixing the clocks removes the light kernel's variation and that locking
both SM and memory low (585 / 810 MHz) reproduces the slow mode. It moved the two clocks
together, so it cannot say which one causes the slowdown. The paper says the driver cuts the
memory clock to 810 MHz for the light path at E=10,000 and that the kernels read every eligible
vector, which suggests the memory clock matters. This experiment crosses the two clocks.

## What was looked at before writing this
E-LOCK medians (already collected, 2026-10-19), going from Lhi (2670/8001) to Llo (585/810):
light selection 3.9-4.0x slower, heavy selection 3.8x, light scoring 3.3-4.4x, heavy
scoring 4.3-4.5x. The SM ratio is 4.56x and the memory ratio 9.9x. The predictions below are
based on these numbers. No E-SPLIT data existed when this file was written.

## Host and binary
Same laptop as E-LOCK: Windows 11, i5-13420H, RTX 4050 Laptop GPU, driver 616.56, Vulkan via
WGPU, AC power. The E-LOCK binary was deleted with `target/`. The binary is rebuilt at commit
2a64473 (`cargo build --release -p qenlo-bench`, rustc 1.98.0):
`target/release/qenlo-bench.exe` sha256 `602a61c1f8d66518ce45f6bcf2239000eabb811e35716632e1693e7a10ff7f7a`.
`crates/qenlo/src/gpu.rs` and `gpu_exact.wgsl` are unchanged since e5d85ad (the D1 build base);
the bench diff since then changes only batch-total counters, which are per-call at B=1.
The runner refuses to start if the hash differs. Because the binary is new, both E-LOCK corners
are re-run here (prediction S0), so all comparisons are within this experiment.
Data, cell code and query settings are as E-LOCK (B=1, k=10, 200 warm-ups, 3 x 5,000 queries,
fresh process per run, recall target 0.99, `research/scripts/run_dvfs_cell.py` unchanged).

## Design
- Engines: `gpu-rows` (light), `gpu-predicate` (heavy). E in {1,000; 3,000; 10,000}.
- Clock conditions, all heater off (SM MHz / memory MHz):
  - `HH` 2670 / 8001 (= E-LOCK Lhi)
  - `HL` 2670 / 810
  - `LH` 585 / 8001
  - `LL` 585 / 810 (= E-LOCK Llo)
- 6 cells x 4 conditions = 24 runs per block; 5 blocks = 120 runs.
  Order within block shuffled with `random.Random(5000 + block)`; bench order seed 9301 + block.
- Locks applied before each process starts, reset (`-rgc`, `-rmc`) after it ends; 5 s pause.
- Preflight probes each of the four (SM, memory) pairs and records `nvidia-smi -q -d CLOCK`.
  A pair the driver rejects is not run and is reported as not testable.

## Telemetry and safety
As E-LOCK: 50 ms `nvidia-smi` log per run, 1 s poller with clock event reasons, `lock.json`
per run, start only at <= 70 C, stop and reset at 90 C or if AC power is lost, clocks reset on
every exit path.

## Metrics
Per run: median `device_selection_ns` (top-k kernel) and median `device_scoring_ns`
(scoring kernel), each analyzed separately. Secondary: end-to-end P50.
For each engine, E and kernel, with t = median over blocks:
memory share m = ln(t_HL / t_HH) / ln(t_LL / t_HH).
m = 0: only the SM clock matters; m = 1: only the memory clock matters.
SM share s = ln(t_LH / t_HH) / ln(t_LL / t_HH). If the effects add on a log scale, m + s = 1.

## Exclusions
A run is excluded and reported if its return code is non-zero, it hit the thermal abort, or its
observed GPU-active median SM or memory clock differs from the requested value by more than 5%.

## Predictions (confirmatory)
- S0. Rebuild check: HH and LL selection medians for both engines are within 10% of the
  E-LOCK Lhi-h0 and Llo-h0 medians at every E.
- S1. Locked clocks stay stable: light selection max/min across blocks <= 1.15 in every
  condition and E.
- S2. Selection kernel is SM-driven: m < 0.3 for both engines at every E.
- S3. Scoring kernel: m < 0.5 for both engines at every E (the SM clock matters more than
  the memory clock even for the kernel that reads the vectors).
- S4. Descriptive: m + s, per engine, E and kernel.

## What follows from each outcome
- S2 and S3 hold: the slow mode is mainly the SM clock. The paper drops the suggestion that
  the 810 MHz memory clock explains the slowdown and says the kernels are latency- or
  SM-bound at these sizes.
- S3 fails (m >= 0.5 for scoring at some E): the memory clock matters for scoring there; the
  paper says so for those cells and keeps the memory-bound wording only for them.
- S2 fails: the memory clock affects even the selection kernel; reported as found.
- S0 fails: the rebuilt binary is not comparable to E-LOCK; E-SPLIT is still analyzed
  internally, and cross-experiment comparisons are dropped.
- HL or LH not settable: the corresponding share is not testable and is reported as such.

## Outputs
`research/data/raw/2026-10-25-dvfs-split/block-<b>/<engine>-e<E>-<cond>/` with the E-LOCK
layout; `preflight-*.json` and `runner.log` at the top level.
Runner: `research/scripts/run_dvfs_split.py`. Analysis: `research/scripts/analyze_dvfs_split.py`;
paper table `paper/v2/tables/split.tex`.
