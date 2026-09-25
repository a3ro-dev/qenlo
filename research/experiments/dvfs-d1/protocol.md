# D1 protocol — does GPU power state, not kernel work, set light-query latency?

Written 2026-09-25 before any D1 data. Pilot (exploratory, n=1 per condition):
`research/data/raw/2026-09-25-dvfs-pilot/` (rows E=3000 P50 0.754 -> 0.397 ms with heater; predicate 2.136 -> 2.288).

Host: Windows 11 laptop, i5-13420H, RTX 4050 Laptop (driver 616.56), AC power, Vulkan.
Binary: target/release/qenlo-bench.exe built from working tree over e5d85ad. Data: AG News 100k x 384 (.qnb sha256 ba87a322...).

Design: 5 blocks. Each block runs every condition once, in an order shuffled with seed 1000+block.
Conditions: engine {cpu, gpu-rows, gpu-predicate} x E {100, 1000, 3000, 10000, 30000} x heater {off, on}; B=1, k=10,
200 warmups, 3 x 5000 held-out queries, order seed 9001+block. Heater = examples/gpu_heater 500 iters, 1 ms sleep
(separate process, 2 s lead). nvidia-smi logs SM/mem clock, P-state, util, power every 50 ms. 5 s pause between runs.
Unit of analysis: run (fresh process) within block; paired heater-on/off ratio per block; median over blocks with
20,000-draw bootstrap over blocks.

Predictions (confirmatory):
P1 gpu-rows: heater-on/off P50 ratio < 0.9 with bootstrap CI excluding 1, at E=100, 1000, 3000.
P2 gpu-predicate: heater-on/off P50 ratio >= 0.95 at every E (heater only adds contention).
P3 interaction: rows ratio < predicate ratio at every E <= 10000.
P4 mechanism: gpu-rows median device_selection_ns falls with heater; host row_materialization_ns does not (ratio in [0.9,1.1]).
Secondary (descriptive): CPU vs gpu-rows winner at each E, heater off vs on; P-state residency by engine.

Refutation: if P1 fails at E=3000, or predicate improves as much as rows (P3 fails), the claim that power state
(not contention/other) causes the light-path penalty is rejected. P4 failing (materialization also falls) would point to a
host-side common cause.
