# E0/E2 Runpod partial campaign analysis — 2026-09-24

## Executive summary

- The archive hash matches the supplied SHA-256. The preregistered matrix is **260 summaries**; **227 summaries** exist (E0 20/20; E2 207/240). There is no `COMPLETE` marker. This is a partial campaign, not 230/230 or a completed campaign.
- All 227 summaries parse and say `status=completed`; all report `recall_target_passed=true` and `filter_violations=0`. However, one completed summary has a recorded process exit code of 101, and one additional CPU directory has no summary. Both are retained and flagged below.
- E0 A/A variation is substantial. CPU P95 has one extreme block; gpu-rows P50/P95 have two unusually fast blocks. The original ±25% gate is not defensible as a universal noise tolerance. These are observations; GPU clocks, thermal state, and host contention were not experimentally isolated as causes.
- E2 shows gpu-rows clearly faster than gpu-predicate and gpu-mask at 0.1% and 1% eligibility for both batches and both P50/P95. At batch 1 and 100% eligibility, gpu-predicate is faster than gpu-rows. The exact crossover fraction is not identified. Batch 16 has no 100% cell.
- Retain the current production router pending a completed confirmatory campaign and E1/E3/E4 evidence. This dataset supports a sparse gpu-rows candidate for targeted validation, not a scalar eligible-count threshold or production fit.

## Integrity, provenance, and data quality

| Item | Finding | Verification / caveat |
|---|---|---|
| Archive SHA-256 | `E4CA1336757CF55A5F5150A8BCC601AB3538CB7E9E9C2270C137E5A72FD51ED0` | Recomputed from the unchanged archive; matches supplied value. |
| Source revision | `e5d85adbb638caa05a99fb8d1ef0a96147091cf0` | Manifest attestation; matches local `git rev-parse HEAD` at analysis time. Working tree has unrelated pre-existing changes. |
| Benchmark binary SHA-256 | `b28e16ffda779304f10bc6fea61ec6fe368ce0923590593e2ab27443bc70285b` | Manifest attestation; executable is not in archive, so independent recomputation is unavailable. |
| Dataset SHA-256 | `ba87a322b6846ce225ce54140cf00ac33250271447ef8f9f83df246131719cba` | Manifest attestation; dataset is not in archive. Configuration records CRC32 `beb3c848` in all completed runs. |
| Runner SHA-256 | `c0ae079bc4ddbac8ad4efef2a4a25c5be8418e9a1276edb5bcd7dda71a24ae56` | Recomputed from `research/scripts/run_e0_e2_runpod.py`; matches manifest. |
| Runtime | Linux x86_64; `qenlo-bench` 0.1.0-alpha.6; WGPU Vulkan; NVIDIA GeForce RTX 4090 | Configurations, manifest, and 227 run records. The benchmark configuration itself says `git_revision=unavailable`; source revision comes from the manifest. Driver/CUDA version and CPU model are not recorded in the archive. |
| Workload | 100,000 rows × 384 dimensions; independent synthetic metadata; k=10; 200 warmups; 3 repetitions; recall target 0.99; batch 1 or 16 | All observed configurations agree. Timed latency is batch-call completion, with nearest-rank within-run percentiles. |
| GPU telemetry | SM clock snapshots 210–2775 MHz; 23–35 °C; 12.24–131.26 W | Before/after only, not continuous telemetry. Values do not establish a cause of latency variation. |
| Summary correctness | 227/227 completed, 227/227 target passed, 227/227 report zero filter violations | Evaluation recall@10 is exactly `1` in 207 summaries and `0.99998` in 20 (all batch 1, E=100,000). Tuning recall@10 is `1` in all 227. Across 2,181,693 sample rows, minimum per-sample recall@10 is 0.9; the aggregate target still passes. |
| Completion exception | E2 B=1 E=100 block 02 gpu-mask has summary and all 3 run rows, but run record says exit code 101 | Included in primary descriptive results and marked in `block_metrics.csv`. Omitting just this block changes rows/mask P95 ratio from 0.197 to 0.198; the finding is unchanged. The error cause is not evident from its record. |
| Unfinished destination | E2 B=16 E=30,000 block 01 CPU has configuration, metadata, truth, and tuning files only | No `runs.csv`, `samples.csv`, or `summary.txt`; counted as missing, never as a latency or failed correctness observation. |
| Runner/stopper logs | Runner log includes 150,941 repeated `can't open file` errors for a later missing script path; stopper log has a shell test syntax error | These are process/control-plane anomalies, not proof that completed benchmark summaries are invalid. The archive has no `COMPLETE` marker. |

## Complete coverage table

Counts are completed summaries per engine, with five planned blocks for each E2 engine and ten for each E0 engine. `—` means the engine was not planned for E0. Missing block identifiers are listed after the table.

| Experiment | B | E | f | CPU | gpu-rows | gpu-predicate | gpu-mask | Total |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| E0 | 1 | 3,000 | 0.030 | 10/10 | 10/10 | — | — | 20/20 |
| E2 | 1 | 100 | 0.001 | 5/5 | 5/5 | 5/5 | 5/5 | 20/20 |
| E2 | 1 | 1,000 | 0.010 | 5/5 | 5/5 | 5/5 | 5/5 | 20/20 |
| E2 | 1 | 3,000 | 0.030 | 5/5 | 5/5 | 5/5 | 5/5 | 20/20 |
| E2 | 1 | 10,000 | 0.100 | 5/5 | 5/5 | 5/5 | 5/5 | 20/20 |
| E2 | 1 | 30,000 | 0.300 | 5/5 | 5/5 | 5/5 | 5/5 | 20/20 |
| E2 | 1 | 100,000 | 1.000 | 5/5 | 5/5 | 5/5 | 5/5 | 20/20 |
| E2 | 16 | 100 | 0.001 | 5/5 | 5/5 | 5/5 | 5/5 | 20/20 |
| E2 | 16 | 1,000 | 0.010 | 5/5 | 5/5 | 5/5 | 5/5 | 20/20 |
| E2 | 16 | 3,000 | 0.030 | 5/5 | 5/5 | 5/5 | 5/5 | 20/20 |
| E2 | 16 | 10,000 | 0.100 | 5/5 | 5/5 | 5/5 | 5/5 | 20/20 |
| E2 | 16 | 30,000 | 0.300 | 1/5 | 2/5 | 2/5 | 2/5 | 7/20 |
| E2 | 16 | 100,000 | 1.000 | 0/5 | 0/5 | 0/5 | 0/5 | 0/20 |

Missing: at B=16/E=30,000, CPU blocks 1–4 and all three GPU engines' blocks 2–4 (13 slots); at B=16/E=100,000, every engine's blocks 0–4 (20 slots). Thus the missing evidence is concentrated in the final **two** E2 cells. No measurements are imputed.

## Methods and uncertainty

For metric q∈{P50,P95}, `L[b,e,q]` is the **lower-middle** of the three `runs.csv` batch-latency quantiles for block b and engine e. Reported cell latency is the median of available block values `L`. P50 and P95 are calculated separately. Each block is the resampling unit; three repetitions within one block do not count as three independent blocks. The paired comparison uses only blocks containing both engines: `d_b=ln(L[b,left,q]/L[b,right,q])`; its point ratio is `exp(median_b d_b)`. A ratio below 1 favors the left engine. The 95% percentile bootstrap draws `n` block values or paired `d_b` values **with replacement**, computes the median for each of 20,000 draws (seed 20260924 plus deterministic cell offsets), and takes the 2.5th/97.5th percentiles. Ratios exponentiate the log interval endpoints. CIs describe repeat-block uncertainty for this one workload and host; they are not population-wide or causal intervals. n=1 has no meaningful interval; n=2 intervals are especially weak.

E0 A/A comparisons use same-engine block latencies. The five disjoint adjacent comparisons are `a_j=ln(L[2j+1]/L[2j])`, j=0,…,4; the all-pairs descriptive set has `ln(L[j]/L[i])` for i<j (45 dependent pairs per engine). The noise statistic is `|a|`, so `exp(|a|)` is the larger/smaller latency ratio. A 25% gate excursion is `exp(|a|)>1.25`. We bootstrap the median of the five disjoint absolute log ratios; the all-pairs 95th percentile is descriptive and is not treated as 45 independent replicates. An E2 advantage is called larger than the conservative E0 band only when the relevant paired 95% CI lies entirely beyond the **larger** of CPU and gpu-rows E0 all-pairs P95 absolute-log bands, metric by metric. This is a screening comparison, since E0 tested only B=1/E=3,000 and its noise may not transfer across E2 cells.

## E0 measurement noise

All latencies are milliseconds. CI is the block-bootstrap interval for median latency. Within-block spread is max/min among the three runs; all-pairs A/A quantiles use 45 dependent block pairs. Outlier blocks are retained.

| Engine | Metric | n blocks | Median latency [95% CI] ms | Median / max within-block spread | Adjacent median abs(log ratio) [95% CI] | All-pairs P95 / max abs(log ratio) | >25%: adjacent / all-pairs |
|---|---|---:|---:|---:|---:|---:|---:|
| cpu | P50 | 10 | 0.369 [0.360, 0.386] | 1.011 / 1.101× | 0.021 [0.015, 0.105] | 0.104 / 0.119 | 0/5; 0/45 |
| cpu | P95 | 10 | 0.382 [0.375, 0.402] | 1.029 / 1.550× | 0.027 [0.004, 0.714] | 0.688 / 0.722 | 1/5; 9/45 |
| gpu-rows | P50 | 10 | 0.702 [0.557, 0.714] | 1.051 / 1.208× | 0.008 [0.001, 0.623] | 0.661 / 0.683 | 2/5; 16/45 |
| gpu-rows | P95 | 10 | 0.750 [0.614, 0.776] | 1.117 / 1.312× | 0.108 [0.014, 0.532] | 0.592 / 0.671 | 2/5; 16/45 |

A robust modified-z rule, `0.6745(L−median(L))/MAD(L)`, threshold `|z|>3.5`, flags CPU block 06 P95 (0.756 ms), gpu-rows blocks 05 and 06 P50 (0.432 and 0.366 ms), and gpu-rows blocks 05 and 06 P95 (0.506 and 0.424 ms). No values are removed. The worst observed within-block P95 spread is 1.550× for CPU and 1.312× for gpu-rows. The conservative all-pairs 95th absolute-log bands are 0.661 (P50; 1.94×) and 0.688 (P95; 1.99×). The 25% tolerance misses this block-scale variation; its origin remains unmeasured. See [E0 block trace](e0-block-trace.png).

## E2 representation ablation

Entries are median batch latency in ms [95% block-bootstrap CI]; `n` is the count of completed blocks for each engine, shown in the coverage table. B=16/E=30,000 uses only 1 CPU block and 2 GPU blocks; B=16/E=100,000 is missing. [Latency plot](e2-latency.png).

### P50

| B | E | f | CPU | gpu-rows | gpu-predicate | gpu-mask |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 100 | 0.001 | 0.017 [0.017, 0.018] | 0.185 [0.182, 0.186] | 0.998 [0.971, 1.007] | 1.021 [0.985, 1.031] |
| 1 | 1,000 | 0.010 | 0.111 [0.110, 0.115] | 0.288 [0.285, 0.350] | 1.049 [1.034, 1.063] | 1.056 [1.051, 1.066] |
| 1 | 3,000 | 0.030 | 0.364 [0.346, 0.419] | 0.445 [0.427, 0.719] | 1.126 [1.079, 1.194] | 1.214 [1.046, 1.224] |
| 1 | 10,000 | 0.100 | 4.190 [4.153, 4.232] | 1.449 [1.088, 1.566] | 1.680 [1.667, 1.706] | 1.736 [1.668, 1.762] |
| 1 | 30,000 | 0.300 | 9.734 [9.718, 9.856] | 4.360 [3.481, 4.532] | 4.344 [4.258, 4.362] | 3.608 [2.767, 4.446] |
| 1 | 100,000 | 1.000 | 11.541 [11.498, 11.596] | 2.545 [2.024, 2.574] | 1.535 [1.519, 1.548] | 1.688 [1.681, 1.718] |
| 16 | 100 | 0.001 | 0.278 [0.275, 0.285] | 0.198 [0.196, 0.201] | 1.209 [1.205, 1.367] | 1.223 [1.186, 1.316] |
| 16 | 1,000 | 0.010 | 1.752 [1.728, 1.795] | 0.252 [0.248, 0.256] | 1.459 [1.414, 1.460] | 1.458 [1.353, 1.484] |
| 16 | 3,000 | 0.030 | 5.692 [5.242, 5.879] | 0.965 [0.806, 1.079] | 1.630 [1.363, 1.650] | 1.628 [1.623, 1.660] |
| 16 | 10,000 | 0.100 | 62.286 [62.189, 63.411] | 2.127 [2.076, 2.411] | 2.263 [2.239, 2.267] | 2.268 [2.262, 2.273] |
| 16 | 30,000 | 0.300 | 154.648 (n=1; CI unavailable) | 5.261 [5.184, 5.337] | 5.184 [5.171, 5.197] | 4.923 [4.751, 5.095] |
| 16 | 100,000 | 1.000 | — | — | — | — |

### P95

| B | E | f | CPU | gpu-rows | gpu-predicate | gpu-mask |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 100 | 0.001 | 0.020 [0.018, 0.020] | 0.210 [0.202, 0.211] | 1.054 [1.042, 1.085] | 1.053 [1.036, 1.074] |
| 1 | 1,000 | 0.010 | 0.120 [0.116, 0.160] | 0.347 [0.327, 0.376] | 1.109 [1.067, 1.134] | 1.116 [1.094, 1.228] |
| 1 | 3,000 | 0.030 | 0.466 [0.375, 0.615] | 0.512 [0.467, 0.751] | 1.264 [1.186, 1.312] | 1.264 [1.195, 1.335] |
| 1 | 10,000 | 0.100 | 4.718 [4.541, 5.179] | 1.780 [1.195, 1.901] | 2.017 [1.831, 2.067] | 1.880 [1.844, 2.304] |
| 1 | 30,000 | 0.300 | 10.947 [10.344, 11.150] | 4.883 [4.634, 6.011] | 4.696 [4.559, 5.336] | 4.871 [4.589, 5.090] |
| 1 | 100,000 | 1.000 | 12.281 [11.895, 12.439] | 2.704 [2.581, 2.929] | 1.651 [1.592, 1.676] | 1.887 [1.778, 1.946] |
| 16 | 100 | 0.001 | 0.363 [0.343, 0.380] | 0.234 [0.207, 0.250] | 1.326 [1.231, 1.397] | 1.403 [1.222, 1.525] |
| 16 | 1,000 | 0.010 | 1.842 [1.771, 2.637] | 0.268 [0.261, 0.316] | 1.502 [1.490, 1.582] | 1.606 [1.477, 1.642] |
| 16 | 3,000 | 0.030 | 5.874 [5.390, 6.779] | 1.104 [0.961, 1.229] | 1.659 [1.473, 1.712] | 1.707 [1.655, 1.935] |
| 16 | 10,000 | 0.100 | 74.766 [66.611, 86.755] | 2.233 [2.215, 3.638] | 2.345 [2.310, 2.387] | 2.317 [2.294, 2.373] |
| 16 | 30,000 | 0.300 | 172.614 (n=1; CI unavailable) | 5.770 [5.434, 6.106] | 5.769 [5.563, 5.975] | 5.367 [5.022, 5.712] |
| 16 | 100,000 | 1.000 | — | — | — | — |

### Paired ratios and crossover

Selected paired P95 ratios (left/right) show the transition without interpolating missing observations. Complete P50 and P95 comparisons, including CPU, are in `paired_comparisons.csv`.

| B | f | rows / predicate P95 ratio [95% CI] | rows / mask P95 ratio [95% CI] | paired n |
|---:|---:|---:|---:|---:|
| 1 | 0.001 | 0.198 [0.187, 0.201] | 0.197 [0.190, 0.203] | 5 |
| 1 | 0.010 | 0.315 [0.306, 0.331] | 0.301 [0.286, 0.337] | 5 |
| 1 | 0.030 | 0.420 [0.368, 0.633] | 0.429 [0.369, 0.625] | 5 |
| 1 | 0.100 | 0.910 [0.578, 0.972] | 0.877 [0.635, 1.011] | 5 |
| 1 | 0.300 | 1.040 [0.963, 1.127] | 1.009 [0.910, 1.234] | 5 |
| 1 | 1.000 | 1.673 [1.563, 1.796] | 1.422 [1.411, 1.640] | 5 |
| 16 | 0.001 | 0.181 [0.148, 0.201] | 0.176 [0.145, 0.189] | 5 |
| 16 | 0.010 | 0.178 [0.169, 0.211] | 0.181 [0.160, 0.197] | 5 |
| 16 | 0.030 | 0.666 [0.579, 0.835] | 0.652 [0.497, 0.697] | 5 |
| 16 | 0.100 | 0.961 [0.947, 1.551] | 0.973 [0.941, 1.570] | 5 |
| 16 | 0.300 | 0.999 [0.909, 1.098] | 1.075 [1.069, 1.082] | 2 |
| 16 | 1.000 | — | — | 0 |

For B=1, rows/predicate favors rows at f=0.10 for P50 and P95 (paired CI below 1), is unresolved at f=0.30, and favors predicate at f=1.00 (paired CI above 1). Thus **f*(1) is not point-identified**; a broad observed transition bracket is (0.10, 1.00), with no supported interpolation through 0.30. Rows/mask P50 changes sign between f=0.10 and 0.30, but the high-side effect is modest relative to E0 noise; P95 is unresolved at 0.10 and 0.30, then mask wins at 1.00. For B=16, rows wins through f=0.03 on paired P50/P95 CIs, results near f=0.10 are unresolved, f=0.30 has only two GPU blocks, and f=1.00 is absent. **No supported f*(16) estimate** exists. Predicate versus mask differences are generally small; neither has a dependable sparse-regime advantage here.

Against the conservative E0 band, rows' advantage over both predicate and mask at f=0.001 and 0.01 clears the P50 and P95 threshold for both B=1 and B=16 (all relevant paired CIs lie below about 0.52 for P50 and 0.50 for P95). At f=0.03 or above, representation differences generally do not clear that stringent band, including the observed predicate advantage at B=1/f=1.00. This screen is deliberately conservative and should not be mistaken for an equivalence test. CPU is decisively fastest at B=1/f≤0.01; gpu-rows is decisively faster than CPU at B=16/f≥0.01 in observed complete cells. Near B=1/f=0.03, CPU/rows P95 ratio is uncertain (rows/CPU 1.211 [0.834, 2.004]); a scalar threshold cannot be read off that point.

## Router implications, limitations, and next action

The current automatic GPU representation should **not be changed in production from this archive alone**. The low-fraction gpu-rows advantage is a clear candidate for a controlled, held-out automatic-path test on the same binary and hardware. A single eligible-count threshold is unsupported: batch changes the CPU/GPU relation, representation matters, and the B=1/f=0.03 point sits inside a wide uncertainty interval. Eligibility fraction here is simply E/100,000 on one corpus; fixed E on different corpus sizes was not tested.

This partial E0/E2 campaign supports claims about repeatability and representation rankings at the measured cells on one RTX 4090 Vulkan environment. It does not establish generalization across hardware, filter shape, corpus size, concurrency, caching, or deployment load; it does not measure a production automatic-route policy or held-out regret. E1/E3/E4 and a complete, predeclared holdout are needed before fitting or recommending a production router. The missing B=16/f=1.00 cell removes the likely high-fraction endpoint for that batch, while the sparse B=16/f=0.30 cell has too few blocks to establish a transition. The one nonzero process return code and log anomalies warrant a targeted rerun or trace inspection, without retroactively treating all summaries as failures.

**Recommended next action:** finish the 33 missing E2 slots, rerun the anomalous B=1/E=100 gpu-mask block in a fresh destination, and repeat E0 noise controls interleaved with high-fraction E2 cells. Preserve the original archive, the partial status, and all raw run records; then predeclare a held-out automatic-path validation before any router change.

## Machine-readable outputs

`audit.json` contains provenance and exceptions. `coverage.csv`, `block_metrics.csv`, `e0_noise.csv`, `latency_summary.csv`, and `paired_comparisons.csv` contain all derived counts and statistics. Run `python analyze.py --extracted-root <temporary-extraction>/e0-e2`, then `python render_report.py` to reproduce the outputs. The original archive and existing research files were not changed.
