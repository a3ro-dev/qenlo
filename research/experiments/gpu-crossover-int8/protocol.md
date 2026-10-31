# gpu-crossover-int8: where should Automatic send work to the GPU? (registered 2026-10-31)

Precondition: runs only on a tree where sq8-certified (chunked arena + int8 certified scan) has
passed its gate and been committed. If it failed, this is re-registered against the FP32 CPU.

## Why
`AUTOMATIC_GPU_MIN_ELIGIBLE_ROWS = 4_096` (crates/qenlo/src/lib.rs) came from the 2026-09-02
native-crossover run (alpha.1, `research/data/raw/2026-09-02-native-crossover`), where the CPU took
~1.3 ms at E=4,000. The int8 CPU scan now takes ~0.15 ms there in `cpu_cliff_bench`. Every query
the router keeps on the CPU skips a GPU submit, a buffer map and a blocking poll, and the GPU never
needs to hold a VRAM mirror for those queries.

## Prediction
At batch 1 the GPU does not beat the CPU at any E <= 100k on this laptop (RTX 4050 laptop,
unlocked clocks, i5-13420H).

## Method
`qenlo-bench run` (built with `--features gpu-wgpu`) on `data/ag-news/ag-news-100k-384.qnb`,
`--k 10 --warmups 200 --repetitions 5 --metadata independent`, backends `cpu` and `gpu-rows` (the
backend the original crossover used), `--eligible-count` in {4000, 10000, 30000, 100000}.
Primary: `--batch 1`. Exploratory: `--batch 16`. Statistic: `median_run_p95_batch_ns` from
`summary.txt`, plus the median of `batch_latency_ns` from `samples.csv`.
Two sessions at least 30 minutes apart, plugged in, nothing else open. Cells run in a shuffled
order fixed by seed 20261031.

## Decision rule (fixed now)
- New threshold = the smallest swept E at which `gpu-rows` has a lower batch-1 p95 AND median than
  `cpu` in both sessions, and also at every larger swept E.
- If no swept E qualifies, the threshold becomes `usize::MAX`, so Automatic keeps batch-1 queries
  on the CPU. `WgpuRequired` is unchanged, so anyone who wants the GPU still gets it.
- Exploratory batch-16 results are reported but do not move the threshold.
