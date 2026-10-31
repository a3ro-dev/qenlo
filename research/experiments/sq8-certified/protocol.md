# sq8-certified: registered protocol (2026-10-31, before any integrated run)

Motivated by the exploratory probe in `findings.md`. Two changes to `qenlo-core`, judged separately.

## A. Chunked vector arena
Vectors live in fixed-size chunks of whole rows (`CHUNK_ROWS`, 8 under `cfg(test)` so every test
crosses chunk edges). The first chunk grows by doubling up to the cap, and later chunks are
allocated at full size. No regrow ever copies more than one chunk.
Prediction: the `add`-built heap matches the exact-reserve heap within 5%, the build peak falls
from 2.2x to <= 1.15x the final heap, and scan latency is unchanged.

## B. Int8 certified first pass
A transient per-row sidecar of int8 codes (row stride rounded up to 16) plus `scale` and an
upward-rounded `|x|_1`, stored in the same chunks and rebuilt on restore. The snapshot and WAL
formats are unchanged. The query is quantized the same way, and the i32 dot is exact. A row is
rescored with the FP64 dot only if `1 - approx - bound <= cutoff + 1e-6`, where
`bound = sq*sx/2*|cq|_1 + sq/2*|x|_1` padded by 1.0001x + 1e-7. New `ExactScan::Int8` (one thread);
`Auto` uses int8 when AVX2 is present and E >= `INT8_MIN_ROWS`.
Prediction: Auto median and P95 <= 0.85x of base at E=30k and 100k.

## Method
`cpu_cliff_bench` (random, N=100k, D=384, k=10, scattered rows) and `footprint`, base = this
commit vs candidate. Two batches at least 30 minutes apart, each 3 interleaved base/candidate
runs; per-cell median of runs within a batch. `analyze.py` from cpu-scan-locality with a 0.005 ms
floor.

## Gate (both batches must pass)
1. `cargo test -p qenlo-core --release` passes, with `Int8` added to every oracle-equality test.
2. A: `footprint` add-path heap at 100k <= 1.05x exact-reserve heap (1,629 B/row base), and total
   heap including the int8 sidecar <= the base's 2,226 B/row.
3. B: Auto median AND P95 <= 0.85x base in at least 2 of E in {10k, 30k, 100k}.
4. No Certified or Auto cell regresses by > 5% + 0.005 ms in median.
A failing part is reverted on its own and recorded as negative.

## Amendment 2 (2026-10-31 20:12, before batch 3 data)
Batch 2 (19:56) failed part 4 by the letter: Auto E=10k median 1.16x and Certified E=20k 1.34x.
In that batch the base binary's own three runs spread 0.82-1.30 ms at E=10k Auto and 3.29-5.67 ms
at E=20k Certified, so the batch cannot resolve a 5% change. Batch 2 stays recorded as failed.
Added rule, fixed before batch 3: a batch is noise-invalid if, in any gate cell (Auto or
Certified at E in {10k, 20k, 30k, 100k}), the base's max/min over its runs exceeds 1.25. A
noise-invalid batch neither passes nor fails. Batch 3 runs at 20:30 with the batch 1/2 binaries.
The candidate ships only if batches 1 and 3 both pass and batch 3 is noise-valid; if batch 3 is
noise-valid and fails, the failing part is reverted as originally planned.
