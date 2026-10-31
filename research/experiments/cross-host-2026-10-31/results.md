# Cross-host results (EXPLORATORY, 2026-10-31)

Six RunPod hosts, one session each, base vs the sq8-certified candidate (chunked arena + int8
certified first pass). Full tables: `analysis.txt` (from `python analyze.py`). Per-query samples
stay on disk only (`.gitignore`); summaries, run CSVs and probes are committed.

Deviation from the protocol: the planned RTX 2000 Ada and A6000 were out of stock, so the driver
fell back per slot (protocol amended in 83169c0). Spend: about $0.82 for this run by pod lifetime
x hourly price (driver.log), plus about $0.08 for the aborted first launch. Every pod was deleted;
`runpodctl pod list -a` was empty afterwards.

| slot | CPU (threads) | GPU |
|---|---|---|
| cpu | EPYC 4564P (32) | none |
| ada-dc | EPYC 9254 (48) | L4 |
| ada-consumer | EPYC 7763 (128) | RTX 4090 |
| ampere-consumer | EPYC 7H12 (256) | RTX 3090 |
| ampere-dc | Xeon Gold 6342 (96) | A40 |
| blackwell | EPYC 7352 (48) | RTX PRO 4000 Blackwell |

## CPU: candidate vs base, P95, median of two rounds

| host | Auto 10k | Auto 30k | Auto 100k | Int8 / base Certified, 100k |
|---|---|---|---|---|
| cpu | 0.57 | 0.37 | 0.39 | 0.41 |
| ada-dc | 0.71 | 0.57 | 0.77 | 0.51 |
| ada-consumer | 1.02 | 0.68 | 0.32 | 0.34 |
| ampere-consumer | 0.64 | 0.51 | 0.22 | 0.40 |
| ampere-dc | 1.04 | 1.01 | 0.84 | 0.31 |
| blackwell | 0.89 | 0.81 | 0.39 | 0.39 |

- The single-thread int8 pass is 0.31-0.51x of single-thread FP32 Certified at E=100k on every
  host, matching the laptop (0.5x on AG News). The scan is memory-bound everywhere, so a quarter
  of the bytes pays off regardless of microarchitecture (AMD Zen 2-4, Intel Ice Lake).
- Auto (int8 + scan pool) gains less on the busiest shared hosts (A40/Xeon, 4090/EPYC 7763):
  pool threads compete with other tenants. Single-thread Int8 is the more stable win.
- Certified (FP32, unchanged algorithm, now on the chunked arena) is 1.08-1.11x slower in median
  at E=1k on five of six hosts (A40: 0.96). That is 3-4 us, within the gate's 5% + 5 us
  tolerance, but consistent: chunk lookup costs something on tiny scans. From E=4k the ratios
  scatter 0.79-1.12 with no consistent direction.
- Exactness: 0 mismatches against the FP32 Certified ids on every host (random data, 4 E values
  x 200 queries).
- Heap (100k x 384, `add` path): 2,226 -> 2,037 B/row, peak 347 -> 194 MiB on every host
  (allocation is deterministic). The restored-store heap rises 1,629 -> 2,027 B/row: the int8
  sidecar costs about 400 B/row once the 27% regrow slack is gone.

## GPU vs CPU (candidate tree, synthetic 100k x 384, batch 1, k=10, p95 ms)

| host | E=4k cpu / gpu | 10k | 30k | 100k |
|---|---|---|---|---|
| L4 | 0.24 / 0.48 | 0.39 / 0.90 | 1.07 / **0.83** | 1.14 / 1.82 |
| RTX 4090 | 0.25 / 0.50 | 0.46 / 0.67 | 0.88 / 1.15 | 0.74 / 1.20 |
| RTX 3090 | 0.59 / 0.67 | 1.04 / 1.65 | 1.90 / 2.47 | 1.97 / 2.82 |
| A40 | 0.53 / 1.10 | 1.30 / 2.72 | 2.48 / 2.54 | 3.34 / 4.52 |
| PRO 4000 Blackwell | 0.21 / 0.25 | 0.54 / **0.48** | 1.13 / **0.87** | 1.05 / 1.16 |

- With the int8 CPU path, batch-1 gpu-rows beats the CPU in only 3 of 20 cells (Blackwell at
  10k and 30k, L4 at 30k), by 10-23%. At E=100k (a contiguous range, sequential scan) the CPU wins
  on every host.
- GPU time is dominated by fixed per-call cost (upload, dispatch, map_async readback): the 4090
  is no faster than the L4 at 4k. Single-query GPU offload does not pay on these servers; any GPU
  win needs batching or a lower per-call overhead, not a faster GPU.
- Recall is identical between backends (0.99998 at 4k from one exact tie in synthetic data).

## What this suggests
1. Int8 certified is a cross-platform win; ship it once the laptop gate (batch 3) clears.
2. The Automatic GPU threshold (4,096 rows) is far too low for batch 1 against this CPU path. The
   laptop gpu-crossover-int8 run decides the number; these hosts say "rarely, at batch 1".
3. Next lever is fewer bytes again (4-bit/1-bit cascade in front of int8), not more threads.
