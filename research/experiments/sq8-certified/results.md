# sq8-certified results (2026-10-31)

Verdict: SHIPPED BY DEVIATION, not a gate pass. Batch 1 passed every part; batch 2 failed part 4
and was noise-dominated (base max/min 1.72 over its own runs, Amendment 2's limit is 1.25); batch 3
was cancelled by the user before it ran. The deciding evidence is batch 1 plus the six-host
exploratory run (research/experiments/cross-host-2026-10-31/results.md).

## Gate, batch 1 (base max/min 1.21, valid)
1. Tests: pass, `ExactScan::Int8` in the near-tie FP64 oracle test; setting the bound to 0 makes
   it fail.
2. Heap at 100k (`footprint`): add path 2,047 B/row vs the candidate's exact reserve 2,027
   (1.01x); <= 2,226 base. Peak 347.7 -> 195.9 MiB. Pass. Note: a restored store grows from 1,629
   to 2,027 B/row (the int8 sidecar).
3. Auto median / P95 vs base: 10k 0.76/0.81, 30k 0.55/0.57, 100k 0.30/0.34. Pass (3 of 3).
4. No Certified or Auto median regression beyond 5% + 0.005 ms. Pass.

## Batch 2 (19:56, noise-invalid by Amendment 2, recorded as failed)
Part 4 failed: Auto 10k median 1.16x, Certified 20k 1.34x. Auto 30k/100k P95 0.63/0.40.

## Costs
- `add` 2.36 -> 2.73 us/row (+16%): quantizing each row on insert.
- Certified FP32 is 1.08-1.11x at E=1k on five of six cloud hosts (3-4 us): chunk lookup.
- Restored-store heap +24% (above). Small stores (2k rows) +6%.
