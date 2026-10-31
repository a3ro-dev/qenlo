# CPU exact-scan memory locality (registered 2026-10-31, before any candidate run)

## Observation that motivated this (exploratory, `baseline-exploratory.csv`)
`cpu_cliff_bench` (N=100k, D=384, k=10, shuffled timestamps), ExactScan::Certified, one thread:
contiguous eligibility (E=100k) costs ~112 ns/row; a scattered subset (E=10k-30k) costs ~220-250
ns/row. Same row size (1,536 B = 24 lines). Reading: the hardware prefetcher streams the dense scan
but restarts at every slot gap, so each scattered row pays DRAM latency.

## Hypotheses
- H1: `search()` revalidates every slot against `RowMeta` even though `filter()` produced them
  under the same borrow. Skipping that and reading `RowMeta` only for rows that pass the FP32 gate
  removes one scattered 32 B load per pruned row. Prediction: <=5% median gain; may not pass gate.
- H2: issuing software prefetches (`_mm_prefetch`, T0) for the next eligible row's vector while
  the current row is scored hides the per-gap DRAM latency (group prefetching; Chen, Ailamaki,
  Gibbons, Mowry, ICDE 2004 / TODS 2007). Prediction: >=15% lower median and P95 on Certified at
  E in {10k, 20k, 30k}; within noise at E=100k (already streamed) and E<=4096 (cache-resident).

## Method
Base and candidate binaries are built from the same tree and run interleaved
(base, cand, base, cand, base, cand) on the same laptop, plugged in, nothing else open.
Per (E, mode) cell take the median across the three runs of each statistic.

## Gate (decided now)
Keep a change only if all of:
1. `cargo test -p qenlo-core --release` passes (fp64-oracle equality tests included).
2. Certified median AND P95 drop >= 5% in at least 2 of E in {10k, 20k, 30k}.
3. No cell (any E, Certified or Auto) regresses by > 5% in median.
Otherwise revert that change and record it as a negative result here.
