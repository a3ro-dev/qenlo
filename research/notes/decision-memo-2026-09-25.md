# Research decision memo — 2026-09-25

Supersedes the framing in `direction-memo-2026-09-24.md`. The original manuscript
(`paper/paper.tex`, `QENLO-RESEARCH-PAPER.pdf`) is preserved untouched; the rebuilt
paper lives in `paper/v2/`.

## 1. Candid diagnosis of the current paper

**Actual thesis.** "Scalar CPU/GPU routing rules fail; a router must jointly choose
backend, eligibility representation and preparation, and be certified with
noise-calibrated tests." No router is validated.

**What it really is.** An audit ledger of 13 cohorts (S0–S2, H0–H7, R1, P0, P2,
E0/E2) with different hosts, revisions, APIs, datasets and timing boundaries. Most
sections report scoped, non-comparable observations (Chroma ratios up to 3233×,
Android/Arc aggregates, lifecycle, spend). The title promises a result, but the
abstract is a list of caveats.

**Research problems (not presentation):**
1. *The headline negative is weakly novel.* Fragile CPU/GPU placement models
   (Breß et al., SIGMOD 2016) and batch-dependent GPU crossovers for vector search
   (Mageirakos et al., arXiv 2605.15957) anticipate it. The R1 "8/16 wrong" count
   is not noise-robust: A/A exceeds 25% in 9/16 cells (paper admits this).
2. *The strongest positive effect is an implementation artifact.* The paper
   presents "rows 3.2–6.1× faster than predicate/mask" as a general
   representation lever. In code (`crates/qenlo/src/gpu_exact.wgsl:117-152`),
   predicate/mask selection is a single-workgroup, k-round scan over all N rows.
   The effect measures that selector, and the relational version of the
   list-vs-bitmap reversal is already published (Ngom et al., DaMoN 2021).
3. *Undisclosed confounds.* The CPU arm switches kernels at E=4,096 (certified
   FP32→FP64 path, `qenlo-core/src/lib.rs:930`), is single-threaded, and re-filters
   per query inside a batch. This distorts every "batch moves the crossover >30×"
   and rows/CPU statement above E=4,096.
4. *Factual error.* H1 is labelled RTX 4050/DX12; its raw configs record Vulkan.
5. *The unexplained ~2× A/A noise is treated as a nuisance.* The paper
   calls it a gate-calibration problem. It is the most interesting phenomenon in
   the archive, and its mechanism was never investigated.

**Presentation problems.** No central figure; the reader cannot tell what was
discovered. There are 20+ tables and 12 figures for heterogeneous cohorts, and
provenance hashes crowd the main text.

## 2. Evidence inventory (key artifacts)

| Evidence | Location | Unit / n | Status |
|---|---|---|---|
| E0/E2 RTX 4090 Linux Vulkan, 227/260 blocks, per-call samples + device timestamps + end-of-process clock snapshots | `research/data/raw/runpod-e0-e2-partial-20260924.tar.gz` (sha `e4ca1336…`); processed `research/data/processed/runpod-e0-e2-{analysis,mechanisms}-20260924/` | fresh process per engine×block; 5 blocks (E2), 10 (E0); 2,181,693 calls | recomputed independently; matches |
| Retrospective clock analysis | `research/data/processed/dvfs-retrospective/`, `research/scripts/dvfs_retrospective.py` | 227 blocks | new (this session) |
| R1 held-out router, RTX 4050 Windows DX12 | `research/data/raw/alpha5-router-heldout-rtx4050.tar.gz`; `research/data/processed/router-noise-audit/` | 1 process/engine/cell, 5 runs | recomputed; no clocks |
| H1 native crossover RTX 4050 (**Vulkan**, not DX12) | `research/data/raw/2026-09-02-native-crossover/` | 1 process per point | recomputed |
| DVFS pilot (exploratory), RTX 4050 Laptop Windows Vulkan, 50 ms telemetry | `research/data/raw/2026-09-25-dvfs-pilot/` | 1 process per condition | new |
| D1 confirmatory heater experiment | protocol `research/experiments/dvfs-d1/protocol.md` (sha256 `f7f6f0dc…`, written before data); raw `research/data/raw/2026-09-25-dvfs-d1/`; analysis `research/scripts/analyze_dvfs_d1.py` | 5 blocks × 30 conditions, fresh processes | see §5 |
| Dataset | `data/ag-news/ag-news-100k-384.qnb` sha256 `ba87a322…` (matches E0/E2 manifest) | 100k corpus, 5k held-out queries | — |
| Historical cohorts S0–S2, H0, H2–H7, P0, P2 | see `paper/tables/claim-to-artifact.json` | heterogeneous | context only; not used for the new thesis |

Independence: the only unit of independent replication is the fresh process
(block). The 3 runs × 5,000 calls inside a process are repeated measurements. The
same 5,000 queries recur in every block.

## 3. Candidate theses (ranked)

**T1 — Power-state inversion ("faster when busier"). RECOMMENDED.**
*Claim.* For latency-bound GPU vector-search queries, the driver's power
management gives work-efficient query paths a low performance state and heavy
paths a high one. The light path's latency is therefore set substantially by
the power state rather than by its work. Making the GPU *busier* with an
unrelated concurrent load speeds the light path up, while it slows the heavy
path. *Why AI researchers care.* Retrieval for RAG and agents, small-batch
inference and other bursty GPU work are latency-bound. Unlocked-clock latency
comparisons between implementations of different intensity silently reward
inefficiency, and isolated benchmarks misstate latency under co-located load.
*Closest prior.* Governor/latency interaction is known on CPUs (Meisner ISCA'11;
Kanev IISWC'14) and mobile GPUs (GearDVFS MobiCom'23). Clock variation as
benchmark noise is 1–22% (Sinha SC22; PRISM). arXiv 2609.11938 argues the
*opposite* bias under power caps. *Difference.* A rank-relevant inversion on
discrete NVIDIA GPUs in a real query path (the efficient path is penalized), an
exogenous intervention with a built-in negative control, and consequences for
routing crossovers. *Support.* Retrospective RTX 4090: light path ends at
345–1,455 MHz, heavy at 2,745–2,775 MHz; device time tracks clock across blocks
(ρ=−0.83, p=0.003 in E0). Pilot RTX 4050: rows in P5 (435 MHz SM / 810 MHz mem)
vs predicate P3 at 2,670 MHz; heater: rows P50 0.754→0.397 ms, predicate
2.136→2.288 ms. *Alternative explanation.* Contention or host-side effects
(heater changes CPU scheduling/power budget); reverse causality (faster host →
higher utilization → higher clocks). *Decisive test.* D1: paired heater on/off,
light vs heavy path vs CPU, 5 blocks, with per-call device timestamps and
host-materialization time as a host-side control. *Refuted if* the rows benefit
is absent at E=3,000, predicate benefits as much as rows, or host
materialization also shrinks (host common cause).

**T2 — Router certification is below the noise floor.** Same-cell A/A reaches ~2×
(E0; R1 9/16 cells >25%). This is established methodology (Mytkowicz ASPLOS'09;
Kalibera & Jones ISMM'13; Maricq OSDI'18), so T2 is novel only in magnitude. It is
retained as a *consequence* of T1: part of the bimodality is power state.

**T3 — Representation matters more than backend.** Ngom 2021 anticipates it, and
here it is an artifact of an O(kN) selector. Not a paper-level claim; it moves to
the appendix as system description.

**T4 — Scalar routing rules fail on held-out work.** Anticipated by prior work
(Breß 2016; Mageirakos 2026), and the CPU arm is confounded. Kept as one
paragraph of motivation (R1) with honest caveats.

**T5 — Scoring is only 2–23% of the call.** True in this system, but it follows from
the naive selector and host path, and VecFlow reports 71% scoring inside CAGRA.
Not general. Dropped.

## 4. Recommendation (revised after data)

**Thesis (one sentence):** The driver runs light GPU vector-search queries in
low power states. The work-efficient kernel is therefore the one whose speed
depends on the machine and the moment: it varies up to ~4x within and across
same-model hosts, while a kernel doing 30-1,000x more work reproduces within
2-4%. Raising the power state with unrelated GPU load speeds up only the light
kernel.

This is narrower than the pre-data version. "Unrelated load makes whole queries
faster" and "the CPU/GPU crossover moves" did **not** hold robustly (see §5), so
the paper does not claim them.

**Decisive figure:** `paper/v2/figures/fig1_central.pdf`.
(a) Per-process device time of the top-k kernel, light vs heavy path, on four GPUs.
(b) The in-process SM clock the driver picks for each path.
(c) Paired heater effect on device time, light vs heavy, per GPU.

## 5. Outcome (final: laptop 5 blocks, RTX 4090 pods 5 blocks on 5 hosts, H100 3 blocks)

Supported:
- **Reproducibility asymmetry.** Heavy top-k kernel spread across fresh processes
  is at most 1% on any single machine and at most 7% across five pod hosts (drivers
  570 and 580). Light kernel spread reaches 4.2x on the laptop, 2.6x on 4090
  host A (retained archive) and 4.3-4.6x across the 4090 pods. At E=3,000 the
  light-kernel median differs 3.8x between 4090 host A and the pods
  (`paper/v2/tables/cross_host.tex`).
- **Low power states for the light path.** Laptop at E=10,000: 585 MHz SM and
  810 MHz memory (P5). H100: 1,605 -> 645 MHz from E=1K to 10K. The heavy path
  runs at boost clocks on every GPU (`tables/power_states.tex`). The governor's
  input is **not** identified: within the light path, clocks fall as E grows, so a
  simple busy-fraction account fails.
- **Intervention (regression-to-the-mean-free test, `tables/tail.tex`).** With the
  heater, the share of light-kernel runs >1.5x slower than their best falls
  52% -> 0% (laptop), 28% -> 16% (4090 pods), 11% -> 0% (H100). The heavy kernel
  never speeds up: it is x1.19 on the laptop, x1.36 on the H100, unchanged on the
  pods. Supporting dose-response: Spearman rho = -0.66 over 59 pairs (exposed to
  regression to the mean; reported as consistent only).
- **Preregistered P2 and device-level P3** hold in every cell.

Refuted or unsupported:
- **P1** (end-to-end light-path speed-up >=10%) fails on every GPU. The pilot's
  1.9x did not replicate.
- **P3 end to end** holds in 10 of 13 cells; **P4** in 6 of 13. The heater also
  perturbs the host; on the H100 it made host materialization 40-48% slower.
- **Heater and CPU/GPU winners.** The heater never changed a winner, so the
  paper does not claim that the crossover moves.

Remaining gaps, and what would reject the thesis:
- **Locked-clock A/B** (admin on the laptop, or bare-metal cloud). If the light
  kernel's cross-host spread persists at identical locked SM and memory clocks,
  the power-state account is wrong.
- **A second engine** (FAISS or cuVS brute force with an ID list vs a bitset). If
  its light kernel reproduces as tightly as its heavy one, the effect is specific
  to Qenlo.
- **A CUDA runtime**; all runs here use Vulkan/WGPU.
- **A GPU-only perturbation** that does not load the host CPU.
- **D2** (DX12 API, batch 16): protocol locked, not run.

**Venue honesty.** This is a measurement and benchmark-validity result for light
GPU retrieval work. It fits MLSys, a systems-for-ML or evaluation workshop, or
an evaluation track. It is not an ICLR main-track paper without the locked-clock
A/B and a second engine.

## 6. Reproduce

```
python research/scripts/extract_tars.py %TEMP%/d1r research/data/raw/2026-09-25-dvfs-d1r/d1r-b{0,1,2,3r,4}.tar.gz
python research/scripts/extract_tars.py %TEMP%/h100x research/data/raw/2026-09-25-dvfs-h100/h100.tar.gz
python research/scripts/cross_host.py %TEMP%/d1r/2026-09-25-dvfs-d1r %TEMP%/h100x/2026-09-25-dvfs-h100
python research/scripts/pool_hosts.py research/data/raw/2026-09-25-dvfs-d1 %TEMP%/d1r/2026-09-25-dvfs-d1r %TEMP%/h100x/2026-09-25-dvfs-h100
python research/scripts/dose_response.py (same three dirs)
python research/scripts/tail_table.py
python paper/v2/scripts/make_tables.py && python paper/v2/scripts/fig_central.py && bash paper/v2/build.sh
```
Cloud spend: about US$5.6 (`research/experiments/dvfs-d1r/campaign-log.md`).
