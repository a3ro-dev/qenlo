# Certified FP32 at any E, plus intra-query threads

The v2 paper's CPU arm (`ExactScan::Legacy`) uses two-pass certified FP32 up to 4,096 eligible
rows and FP64 accumulation above, single-threaded. Its CPU/GPU crossover between E=3,000 and
10,000 therefore sits on the CPU's own FP32-to-FP64 cliff. This change makes the default route:

- `ExactScan::Certified`: one pass, any E. Each row is scored in FP32 and rescored in FP64 only if
  it falls within `2 * error_bound` of the exact running k-th distance.
- `ExactScan::Auto` (what `search` uses): the same scan split into contiguous chunks on a persistent
  std-only pool, one thread per 2^18 eligible-row dimensions, with a per-chunk top-k and a merge.
- `ExactScan::Parallel`: every pool thread regardless of size (for tests and benchmarks).

Results are identical to the FP64 reference in every mode
(`every_scan_mode_matches_fp64_oracle_on_near_ties`, plus the existing oracle tests).

## Measurement

`cargo run --release -p qenlo-core --example cpu_cliff_bench`: N=100k, D=384, k=10, scattered
eligible rows, 200 queries per cell after 20 warmups, modes interleaved per E. i5-13420H
(4P+4E, 12 threads), Windows 11, on AC power. Each cell is the median of three runs' medians, in ms.
The machine was running other work (~40% CPU load) and one of the three runs was disturbed
throughout, so treat differences under ~20% as noise.

| E | Legacy (paper) | Certified 1T | Parallel 12T | Auto | Legacy/Certified | Legacy/Auto |
|---:|---:|---:|---:|---:|---:|---:|
| 1,000 | 0.061 | 0.058 | 0.061 | 0.059 | 1.05x | 1.03x |
| 2,000 | 0.148 | 0.142 | 0.110 | 0.095 | 1.04x | 1.56x |
| 3,000 | 0.329 | 0.408 | 0.167 | 0.250 | 0.81x | 1.32x |
| 4,096 | 0.518 | 0.455 | 0.230 | 0.267 | 1.14x | 1.94x |
| 4,097 | 0.736 | 0.452 | 0.213 | 0.263 | 1.63x | 2.80x |
| 6,000 | 1.149 | 0.765 | 0.351 | 0.342 | 1.50x | 3.36x |
| 10,000 | 1.973 | 1.462 | 0.873 | 1.369 | 1.35x | 1.44x |
| 20,000 | 4.288 | 3.735 | 2.460 | 2.434 | 1.15x | 1.76x |
| 30,000 | 6.626 | 5.440 | 3.274 | 3.262 | 1.22x | 2.03x |
| 100,000 | 15.235 | 15.371 | 6.207 | 6.648 | 0.99x | 2.29x |

At E <= 4,096 Legacy and Certified run nearly the same algorithm, so the 0.81x at E=3,000 is
noise. Above 4,096, single-thread Certified removes most of the cliff (1.15-1.6x). At E=100k, where
every row is read in sequence, both routes are limited by DRAM bandwidth and tie; that is where
certified stops winning on this host, compared with +10% for the old two-pass route on the A40
host (`phase2-cpu-exact-progress.md`). Above about 8k scattered rows the working set no longer
fits the 12 MB L3, so threads stop scaling at about 2x (about 25 GB/s at E=100k).

Waking all 12 threads costs about 25 us, so `Parallel` loses below about E=1,500; `Auto` stays
single-threaded there.

Open: this is one laptop under background load. A fair crossover for the paper needs quiet,
interleaved runs on the paper's own hardware, with `ExactScan::Legacy` plumbed through the
`qenlo` collection API if the benchmark harness should select it.
