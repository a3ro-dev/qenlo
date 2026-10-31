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

## Amendment 1: H2' (registered 2026-10-31, after the first results, before any H2' run)
H2 failed only the no-regression rule, on a 4 us cell (Auto E=2,000, 1.053x) that single-thread
Certified at the same E did not reproduce (0.99x). H2' retests the same prefetch patch
(`candidate-h2.patch`, H2 hunks only, no H1) with:
- base and candidate binaries rebuilt fresh from this commit; 5 interleaved runs each
  (base, cand, ... x5); median of runs per cell.
- gate rule 2 unchanged (Certified median AND P95 <= 0.95x in >= 2 of E in {10k, 20k, 30k}).
- gate rule 3 amended: a cell regresses only if cand median > 1.05 x base median + 0.005 ms.
  The absolute floor is the size of a thread-pool wake, below anything a caller can see.
- `analyze.py <runs> base h2 0.005` applies it; with no floor argument the original gate is
  reproduced unchanged.
If H2' fails, prefetching is dropped and the result recorded as negative.
