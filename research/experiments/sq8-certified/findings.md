# Byte-cutting the exact scan: exploratory probe (2026-10-31)

Probe: `crates/qenlo-core/examples/sq8_probe.rs`. N=100k, D=384, k=10, 200 queries, single thread,
scattered timestamps. Data: AG News all-MiniLM-L12-v2 embeddings (`data/ag-news`) and uniform
random. EXPLORATORY: one session on the i5-13420H laptop; nothing here is a registered gate.

## Where the bytes go
| | B/row |
|---|---|
| vectors (FP32, D=384) | 1,536 |
| store built with exact reserve (`restore`) | 1,629 |
| store built by `add` (1.5x regrow) | 2,226 |
| peak heap during the `add` build | 347 MiB for a 155 MiB store (2.2x) |

At this N, 597 B/row (27% of the heap) is regrow slack. The slack swings between 0 and 50% of
the vector bytes depending on where N falls between regrows, and every regrow copies the whole
buffer, which is a stall on one unlucky `add`.

## Int8 certified first pass
Each row gets int8 codes plus one scale (`x = s*c + e`, `|e_i| <= s/2`). The query is quantized the
same way. The integer dot is exact in i32 (AVX2 `madd_epi16`). The deterministic bound is
`|q.x - sq*sx*<cq,cx>| <= sq*sx/2*|cq|_1 + sq/2*|x|_1`. A row is rescored with an FP64 dot only if
its lower-bound distance can beat the running k-th exact distance. This is the existing certified
FP32 route one precision step down, and the bound is deterministic, unlike RaBitQ's
probabilistic one.

| E | Certified FP32 ms | int8 certified ms | ratio | rows rescored | id mismatches vs FP64 |
|---|---|---|---|---|---|
| 1,000 | 0.063 | 0.068 | 1.07 | 75 | 0/200 |
| 10,000 | 1.417 | 1.230 | 0.87 | 114 | 0/200 |
| 30,000 | 3.492 | 2.295 | 0.66 | 136 | 0/200 |
| 100,000 | 9.582 | 4.806 | 0.50 | 162 | 0/200 |
(AG News; the random data is within 0.02 of every ratio, see `probe-*.txt`.)

The probe's layout is naive: a heap `Vec<i8>` per row, so every row is a pointer chase, and
the FP32 pass ran first on each query (warm-cache bias in its favour). A contiguous arena should
do better than these numbers.

## What this adds up to
- Speed: about 2x at full scans, 1.5x at E=30k, and no gain under ~2k eligible rows (route by E,
  as `Auto` already does for threads).
- RAM: the sidecar costs +392 B/row (384 codes + scale + L1, contiguous). A segmented vector
  arena (fixed-size chunks, no whole-buffer regrow) removes the 0-50% slack and the 2.2x build
  peak. At this N the net is 2,226 -> ~2,020 B/row with half the scan time. Averaged over N, the
  steady-state heap comes out about even, while peak heap and insert stalls drop a lot.
- Bigger follow-on: if FP32 rows are only needed for ~160 rescores per query, they can stay in the
  durable layer (mmap of the snapshot) instead of the resident heap, leaving ~490 B/row resident
  (3.3x smaller). This is a change to the storage contract, not just a kernel.

## Literature
- PDX, Kuffo, Krippner, Boncz, SIGMOD 2025 (arXiv 2503.04422): exact scan kernels are memory-bound.
- Aguerrebere et al., LVQ, VLDB 2023: per-vector scaled quantization for bandwidth-bound search.
- Gao and Long, RaBitQ, SIGMOD 2024: quantization with an error bound for re-ranking
  (probabilistic; the bound used here is worst-case).
- FAISS IndexRefine: compact first pass plus a full-precision refine.
