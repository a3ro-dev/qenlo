# Research-direction memo: E0/E2 partial campaign

Date: 2026-09-24. Scope: the canonical paper (`paper/paper.tex`), all retained
cohorts it cites, the R1 held-out router archive and its noise audit, and the
partial Runpod E0/E2 archive
`research/data/raw/runpod-e0-e2-partial-20260924.tar.gz`
(SHA-256 `e4ca1336…fd51ed0`). Nothing here is a new benchmark on the campaign host.

## Campaign status (never round up)

260 planned summaries, **227 present**: E0 20/20, E2 207/240. No `COMPLETE`
marker. Missing: 13 slots at B=16/E=30,000 (CPU blocks 1–4; each GPU engine
blocks 2–4) and all 20 at B=16/E=100,000. One summary-bearing gpu-mask block
(B=1/E=100, block 02) has process exit code 101. One CPU destination
(B=16/E=30,000 block 01) has no summary and counts as missing. Twenty summaries
report recall@10 **0.99998**: exactly the four engines × five blocks at
B=1/E=100,000, including CPU. The archive carries 6,575 trailing non-gzip bytes.
GNU tar aborts on them, but Python `tarfile` reads all 2,132 members.

## Directions compared

**D1 — keep "scalar routing rules fail" unchanged.** E0/E2 help this thesis.
At B=1/E=3,000 (EDB 1.152M), CPU beats gpu-rows in 9/10 E0 blocks (P50
rows/CPU 1.92 [1.44, 1.96]; P95 1.97 [1.55, 2.06]). At B=16/E=100 (EDB 0.614M),
gpu-rows wins 5/5 blocks (P50 0.709 [0.705, 0.725]). On one host with one binary,
no increasing EDB threshold classifies both cells. This replicates R1's ordering
contradiction, which previously rested on one 3.16%-margin cell that the R1
noise audit also flags as a P50/P95 winner disagreement. D1 alone misses two
things. The "8 of 16 wrong" count is not noise-robust. And backend choice is not
the largest lever in E2.

**D2 — reframe: routing is a backend × representation × preparation decision,
and noise bounds what a gate can certify.** This matches all retained evidence:
- Representation: at f ≤ 0.01, gpu-rows is 3.2–6.1× faster (paired medians, P50 and P95) than
  gpu-predicate and gpu-mask at both batches. This clears the conservative E0
  band. The ratio is larger than most CPU/GPU gaps near crossover.
- Preparation: host row materialization is 51% of the gpu-rows call at
  B=1/E=30,000 (2.21 of 4.33 ms median) and is paid by *all three* GPU
  representations. H2's one-pass/cache ablation already showed preparation
  matters.
- Noise: same-cell A/A block ratios reach 1.94×/1.99× (all-pairs P95, P50/P95).
  That is far above the 25% gates used to accept or reject routers. In R1,
  automatic vs same-backend A/A exceeds 25% in 9/16 cells. GPU block latency is
  bimodal and associated with end-of-run SM clock snapshots (80 concordant vs
  31 discordant within-cell pairs; association only).
- Batch: moves the CPU/gpu-rows winner by more than 30× in E on one host.
- R1's rejection survives on its maximum-regret cell (235.7%, a 3.36× ratio,
  outside even a 2× band). H1/P0's equal-E contradiction, P2's sparse-only
  optimization, and S0/S1's 5/7 selector are all consistent with D2.

**D3 — "gpu-rows should be the default GPU representation."** This is an
engineering candidate, not a research thesis. At B=1/f=1.00, predicate is
*faster* (P95 rows/predicate 1.673 [1.563, 1.796]). The B=16 high-fraction
cells are missing. It needs held-out validation (E4 below).

**Decision: D2, as a narrower but better framing of the existing thesis.** The
held-out negative result stays the anchor. The paper now says what a router
must decide and measure, and it stops presenting the 8/16 count as
noise-robust. Subtitle: "Why Scalar Routing Rules Fail and What a Router Must
Measure."

## Claim-to-evidence map

| # | Claim (as the paper now states it) | Class | Evidence | Status |
|---|---|---|---|---|
| 1 | 227/260 summaries; E0 complete; E2 missing 33 slots in two B=16 cells | Observed | archive; `runpod-e0-e2-analysis-20260924/coverage.csv`; `runpod-e0-e2-mechanisms-20260924/audit.json` | Verified independently (SHA, counts, exit codes, recall values) |
| 2 | Same-cell A/A block variation up to ~2× on RTX 4090/Vulkan | Observed | `e0_noise.csv`; `block_metrics.csv` | One cell (B=1/E=3,000), one host |
| 3 | Slow/fast GPU blocks are associated with end-of-run SM clock snapshots | Observed association | `gpu_clock_snapshots_b1.csv`; `audit.json` | Snapshot, not continuous telemetry |
| 3h | DVFS/power state causes the bimodality | **Causal hypothesis** | — | Needs E0′ with continuous clocks or locked clocks |
| 4 | gpu-rows ≫ predicate/mask at f ≤ 0.01, B ∈ {1,16} | Observed | `paired_comparisons.csv` | Clears the conservative E0 band |
| 5 | Predicate beats rows at B=1/f=1.00 | Observed | `paired_comparisons.csv` | Does not clear E0 band; B=16 counterpart missing |
| 6 | f*(1) lies in (0.10, 1.00); no f*(16) estimate | Observed limit | `paired_comparisons.csv` | Not point-identified; no interpolation |
| 7 | Within-host EDB ordering contradiction (backend choice) | Observed, post-hoc | `edb_ordering_witness.csv` | Paired-block CIs resolve it; conservative A/A band does not |
| 8 | Predicate/mask selection over all N rows costs ~0.70 ms regardless of E | Observed diagnostic | `diagnostics_by_cell.csv` (device timestamps) | Device timestamps overlap host phases; not additive |
| 8h | Full-N selection explains predicate/mask's sparse deficit | **Causal hypothesis** | code: `gpu.rs` `chunk_eligibility` / selection dispatch | Needs a selection-kernel ablation |
| 9 | Row materialization is 51% of gpu-rows B=1/E=30k and was paid by predicate/mask in the measured binary | Observed diagnostic + code | `diagnostics_by_cell.csv`; measured `lib.rs` `EligibilityPlan::compile` retained rows for all modes | Mechanism verified; audited source now avoids the unused list for required predicates |
| 10 | B>1 transfer/lock-wait fields were overstated B× by the harness | Observed defect + code | `qenlo-bench/src/main.rs` summed batch totals; archive B=16 upload = 16 × (24,576+400,000+64) | Fixed; S2 batch-8 transfer figures corrected in paper |
| 11 | Range-filter materialization is superlinear (BTree traversal + sort) | Code + local microbenchmark | `qenlo-core` `filter`; ignored timing test | Fixed with bounded probe → scan; local laptop only |
| 12 | R1 automatic path used ShaderPredicate while regret used gpu-rows | Observed (R1 archive) | `router-noise-audit/` | Representation mismatch is a *hypothesis* for R1's auto/best spread |
| 13 | A sparse gpu-rows automatic representation would reduce regret | **Engineering opportunity** | E2 sparse cells | Requires E4; not deployed |
| 14 | Any scalar E or EDB threshold for production | **Not claimed** | — | Explicitly refused on partial single-host data |
| 15 | Required shader-predicate batches can avoid host row-list materialization without changing routing | Engineering change, not a speedup claim | `qenlo-core::CoreStore::filter_count`; `EligibilityPlan::compile`; correctness and GPU diagnostic tests | Implemented; campaign-host latency unmeasured |

## Engineering opportunities (implemented only where behavior-preserving)

1. **Done:** the benchmark takes batch-total counters once per batch and
   fails on disagreement. Run format is now `qenlo-bench-run-v4`.
2. **Done:** timestamp-range materialization uses a bounded index probe
   (rows/16) and then a sequential scan. Output is identical; tested on both
   paths. Locally, ≥20k eligible gets 1.6–17× faster; 8k–12k gets up to 2×
   slower (about +0.15 ms worst case). Not measured on the campaign host.
3. **Done:** required shader-predicate batches use an allocation-free metadata
   count for reporting and do not build or sort a host row list that the shader
   cannot consume. Automatic predicate execution still retains rows for its CPU
   route and failure fallback. Correctness and diagnostics are tested; no speedup
   is claimed without a comparable campaign-host rerun.
4. **Done:** the E0/E2 runner validates the sibling run record and zero exit code
   before treating an existing summary as resumable completion; the retained
   exit-101 block remains untouched and disclosed.
5. **Not done (needs E4):** default automatic GPU representation (rows vs
   predicate), resident liveness for predicate mode (it re-uploads 400 KB per
   call), bit-packed masks, and E-proportional selection for predicate/mask.
   These change what production executes, not just how fast one fixed path runs.

## Claims that require further experiments

- **E1** (as defined in `research/scripts/run_decision_queue.py`): A/A-replicated
  R1 witness cells with the automatic arm. Needed before any per-cell "wrong
  choice" count is reported as robust.
- **E3** (same script): d × B × k factorization around the EDB witness. Needed
  to attribute the ordering contradiction to a factor.
- **E4** (not defined in any retained artifact; specified here): frozen
  automatic-path policy, including representation choice, evaluated end to end
  on held-out shapes and a second device. Interleaved forced/automatic blocks,
  continuous clock telemetry, the E0-derived noise band as the decision margin,
  and a preregistered maximum-regret limit that exceeds that band.
- **E2 completion:** the 33 missing slots, a fresh-destination rerun of the
  exit-101 block, and E0 controls interleaved with the high-fraction cells.
