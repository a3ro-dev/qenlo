# cpu-scan-locality results (2026-10-31)

Three interleaved runs per binary (base, h2, h12 = H1 on top of H2), i5-13420H laptop.
Per-cell medians and the gate are in `gate-*.csv`, raw runs in `runs/`, and the candidate diff
in `candidate-h1-h2.patch`. Both changes are reverted from `qenlo-core` as the protocol requires.

## H2 software prefetch: REJECT under the registered gate
- Confirmatory: Certified E=20k median 0.68x / P95 0.80x, E=30k 0.73x / 0.76x (2 of 3 wins);
  E=10k 0.96x / 0.97x missed the 5% bar.
- It failed the no-regression rule on one cell: Auto at E=2,000 median 1.053x (0.076 -> 0.080 ms,
  4 us, 2 threads). Certified at the same E was 0.99x, so this is probably pool-wake noise and not
  prefetch cost. The gate was fixed in advance, though, so the verdict stands.
- Exploratory: the dense E=100k cell also dropped (0.77x Certified, 0.83x Auto). The prediction
  was ~1.0x, so the hardware streamer was not saturating DRAM either.

## H1 deferred row metadata (marginal over H2): REJECT
- The scattered cells it was gated on were 0.94-0.97x.
- Exploratory: cache-resident cells moved more than predicted (E=1k-6k Certified median
  0.74-0.87x). Once the vector is in cache, the scattered 32 B meta load is a visible share of
  per-row cost.

## Next (needs a new registration, not a rerun of this one)
- H2': same patch, 5 interleaved runs, regression rule applied to median-of-runs with a 5% + 5 us
  floor so a 4 us cell cannot veto. Registered before running.
- H1': gate on E <= 6,000, where the exploratory effect sits.

## H2' confirmatory rerun (amendment 1): REJECT, prefetching dropped
Fresh binaries, 5 interleaved runs, 5 us regression floor (`gate-h2prime.csv`, `runs-h2prime/`).
- Certified, scattered: E=10k 0.95x / 0.97x, E=20k 0.92x / 0.95x, E=30k 0.97x / 0.95x
  (median / P95). Only 1 of 3 cells cleared 5% on both, so the gate fails. No cell regressed.
- The first round's 0.68-0.77x did not replicate. The base itself ran much faster this time
  (E=20k Certified 3.99 -> 2.62 ms, E=100k 10.96 -> 7.12 ms), so the earlier gap came from
  machine state during that session (thermal or power plan), and interleaving three runs did not
  cancel it. What is left is a real but small 3-8% Certified gain, within this laptop's noise.
- Auto E=2,000 was 1.053x again (0.076 -> 0.080 ms) in both rounds. That consistency suggests
  real prefetch overhead when rows are cache-resident and split over two threads, not noise.
- Lesson: on this laptop a single session can shift absolute latency by ~1.5x. Any future CPU gate
  needs runs from at least two separate sessions before a result counts.
