# gpu-crossover-int8 results (2026-10-31)

Laptop session 1 (20:17-20:33, RTX 4050 Laptop, driver 616.56, i5-13420H), AG News 100k x 384,
batch 1, k=10, warmups 200, repetitions 5. The CPU cells ran the int8 certified path from E=10k
(`cpu_distance_path` = Int8Avx2Certified).

| E | cpu p95 ms | gpu-rows p95 ms | cpu median ms | gpu-rows median ms |
|---|---|---|---|---|
| 4,000 | 0.47 | 0.98 | 0.35 | 0.65 |
| 10,000 | 0.90 | 1.17 | 0.63 | 0.94 |
| 30,000 | 2.48 | 1.96 | 1.87 | 1.54 |
| 100,000 | 2.48 | 3.38 | 2.07 | 2.51 |

Decision (rule fixed in protocol.md): the GPU must win at an E *and every larger swept E* in
both sessions. It loses at E=100k in session 1, so no E can qualify whatever session 2 shows;
session 2 was not run. Batch-1 Automatic stays on the CPU (`usize::MAX`, implemented by dropping
the static single-query GPU route). Batch 16 was not run.

Agrees with the six cloud hosts (cross-host-2026-10-31): the GPU won batch-1 cells only at 10k-30k
scattered filters on two devices and lost at E=100k everywhere.

Deviations: precondition met by a recorded deviation (sq8-certified shipped without a batch 3);
`--metadata independent` in the protocol is the bench's `--distribution independent`.
