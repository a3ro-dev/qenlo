# Spec: locked-clock A/B and a FAISS second engine

Status: **specified, not run.** Written 2026-09-25 after the paper in `paper/v2/`
(now `QENLO-RESEARCH-PAPER.pdf`). Neither experiment has any data yet. Both
protocols should be copied into `research/experiments/<name>/protocol.md` and
hashed before their first run, the same way D1/D1R/H100 were.

These are the two experiments that separate "strong workshop paper" from
"credible MLSys submission". Reviewers will attack the paper at two points:

1. **The mechanism is inferred, not isolated.** The heater raises the GPU's power
   state only indirectly, and it also loads the host CPU (H100 pod: host
   materialization +40–48%).
2. **One engine.** Both GPU paths come from Qenlo's WGPU kernels, including a
   simple O(k·N) selection kernel. The effect could be a Qenlo/WGPU quirk.

E-LOCK addresses the first point and E-FAISS the second.

---

## E-LOCK: locked-clock A/B

### Question
If both paths run at identical, fixed SM and memory clocks, does the light
kernel's variation disappear, and does it run at its best observed speed?

### Claim under test
The light kernel's up-to-4× spread between processes and hosts
(`paper/v2/tables/cross_host.tex`) is caused by driver-selected power states,
not by the kernel, the host or WGPU scheduling.

### Hardware and permissions
Needs a machine where clock locking is allowed. The cloud containers refused it
(`nvidia-smi -lgc` → "does not have permission", rc 4).

| Option | How | Notes |
|---|---|---|
| A. Maintainer laptop (RTX 4050 Laptop, Windows) | Elevated shell: `nvidia-smi -lgc <f>,<f>`; memory: `nvidia-smi -lmc <m>,<m>` | Some laptop GPUs reject `-lmc`. If so, run SM-lock only and record it as a limitation. Always reset with `nvidia-smi -rgc` / `-rmc`. **The maintainer runs these commands, not an agent.** |
| B. Bare-metal or VM with root-level driver access (e.g. a dedicated server with an RTX 4090 or L40S) | same commands | Gives the Linux counterpart. Check `nvidia-smi -q -d SUPPORTED_CLOCKS` first. |

Record before each lock: `nvidia-smi -q -d CLOCK,PERFORMANCE,SUPPORTED_CLOCKS`.

### Design
- **Binary, data, cells:** identical to D1 (`research/experiments/dvfs-d1/protocol.md`).
  AG News 100K×384 (`ba87a322…`), B=1, k=10, 200 warm-ups, 3×5,000 queries,
  fresh process per condition.
- **Engines:** `gpu-rows` (light), `gpu-predicate` (heavy). CPU is not needed.
- **E:** {1,000; 3,000; 10,000}.
- **Clock conditions** (between-process factor, shuffled within block):
  - `U`: unlocked (driver default), as in D1.
  - `L-hi`: SM locked at the heavy path's observed boost (laptop 2,670 MHz),
    memory at max if lockable.
  - `L-lo`: SM locked at the light path's observed low state (laptop 585 MHz),
    memory at 810 MHz if lockable. This reproduces the penalty on purpose.
- **Heater arm:** heater on/off crossed with `U` and `L-hi` only
  (2×2 + `L-lo` = 5 clock×heater conditions).
- **Blocks:** 5, order shuffled with seed 4000+block, order seed 9301+block.
  6 cells × 5 conditions × 5 blocks = **150 runs** (≈2 h on the laptop).
- **Telemetry:** 50 ms `nvidia-smi` log (existing `run_dvfs_cell.py`). Also log
  `clocks_event_reasons` to confirm the lock held.

### Predictions (confirmatory)
- **K1.** Under `L-hi`, the light kernel's device-time max/min across blocks is
  ≤ 1.15 in every cell (D1 unlocked: up to 4.16).
- **K2.** Under `L-hi`, the light kernel's median device time is within 15% of
  the best value observed unlocked for that cell (i.e. locking high recovers the
  "best" state).
- **K3.** Under `L-lo`, the light kernel's median device time is within 25% of
  the D1 slow-mode values (e.g. E=10,000 ≈ 0.46–0.60 ms). This shows the low
  state alone reproduces the penalty.
- **K4.** Under `L-hi`, the heater's effect on light device time is
  ≥ 0.95 (no speed-up left to give). Under `U` it reproduces D1 (< 0.9 at
  E ≥ 3,000).
- **K5.** The heavy kernel's device time under `L-hi` equals its unlocked value
  within 3%.

### Rejection criteria
- **K1 fails** (light spread ≥ 2× at locked clocks): the variation is **not** a
  power-state effect, and the paper's mechanism claim must be withdrawn.
- **K2 fails but K1 holds:** clocks explain the variance but not the level. The
  paper's "understates the efficient kernel" claim is then limited to the
  variance.
- **K3 fails:** the low state alone does not reproduce the penalty. Something
  else co-occurs with it (e.g. memory-controller or PCIe link state). Report
  and investigate `pcie.link.gen.current`.

### Outputs
- `research/data/raw/<date>-dvfs-lock/` (same layout as D1, plus a `lock.json`
  per run with the requested and observed clocks).
- Analysis: extend `research/scripts/analyze_dvfs_d1.py` with a `clock`
  factor; add a table `paper/v2/tables/lock.tex`.
- Paper change if confirmed: promote to a new Figure 2, "same kernel, three
  clock conditions", and upgrade the mechanism statement from inferred to
  demonstrated.

---

## E-FAISS: FAISS as the second engine

### Question
Does the reproducibility asymmetry appear in a GPU vector-search engine other
than Qenlo? FAISS is chosen because it is the reference GPU similarity-search
library (Johnson, Douze, Jégou, IEEE TBD 2019/2021).

### Claim under test
A light filtered brute-force path (work ∝ E) varies with the GPU's power state,
while a heavy one (work ∝ N) does not. The claim concerns power-state
sensitivity, not FAISS's absolute speed.

### Environment
- **Linux + CUDA.** FAISS GPU is not distributed for Windows. Use Runpod RTX 4090
  (secure, $0.74/h) and optionally one H100 ($3.49/h). Budget ≤ $5; E-LOCK
  cannot run there (no clock permission). Only the unlocked + heater design runs
  in the cloud.
- **Install:** `pip install faiss-gpu-cu12` (or conda `pytorch::faiss-gpu`).
  Pin the version and record `faiss.__version__`, CUDA runtime, driver, and
  `torch.__version__` if used.
- **Heater:** the existing WGPU heater (`crates/qenlo/examples/gpu_heater.rs`)
  works on these pods with the EGL Vulkan ICD fix
  (`research/experiments/dvfs-d1r/campaign-log.md`). Add a CUDA heater with the
  same duty cycle (a 1 ms-sleep loop launching a small FMA kernel via
  `torch.cuda` or CuPy), so the perturbation does not depend on Vulkan.
  **Improvement over D1:** pin the heater to one CPU core
  (`taskset -c <last core>`) and put the benchmark on the other cores
  (`taskset -c 0-<n-2>`). That limits the host-CPU confound the H100 exposed.

### Gate G0 (feasibility, 30 min, before the protocol is locked)
Determine which FAISS GPU filtered-search forms exist in the pinned version.
Do not assume; test and record in `research/experiments/faiss-g0/notes.md`.

1. Does `GpuIndexFlatIP.search(x, k, params=faiss.SearchParameters(sel=...))`
   accept `IDSelectorBatch` / `IDSelectorBitmap` on GPU? If it raises or
   silently ignores the selector, record that exactly. Verify correctness of the
   returned ids against a CPU oracle, not just that it runs.
2. Does `faiss.knn_gpu(res, xq, xb_subset, k)` (bfKnn) run on a gathered subset?
3. Timing boundary: host call → host-visible ids/distances, including any gather.

### Paths (final choice depends on G0)
| Path | Preferred form | Fallback if G0.1 unsupported |
|---|---|---|
| **Light** (work ∝ E) | Gather the E eligible rows on device (`torch.index_select` on a resident tensor, or a cached per-filter `GpuIndexFlatIP` built from the subset). Then run `knn_gpu` / `search` over E rows. | same |
| **Heavy** (work ∝ N) | `GpuIndexFlatIP` over all N with `IDSelectorBitmap(sel)` | Full-N inner product plus `masked_fill(−inf)` on ineligible rows plus `topk`, in PyTorch on the same device. Report that the heavy path is then PyTorch, not FAISS. Post-filtering an unfiltered FAISS top-k is **not** acceptable: it is not exact at selective E. |

State in the paper exactly which path each FAISS arm used. The light path is the
one that matters most, since the claim is about light-kernel sensitivity.

### Design
- **Data and cells:** the same AG News corpus and queries as D1 (normalized FP32,
  inner product = cosine). E ∈ {1,000; 3,000; 10,000}, B = 1, k = 10, the same
  timestamp-prefix eligible sets (export `metadata.csv` ranks from any D1 run).
- **Per process:** load → build resident index/tensors → 200 warm-ups → 3×5,000
  timed calls. Record per-call latency (host), CUDA-event kernel time around the
  search, and 50 ms `nvidia-smi` telemetry.
- **Conditions:** path {light, heavy} × E × heater {off, on}. 5 blocks, one
  block per pod (as in D1R). Shuffle seed 5000+block; order seed 9401+block.
  12 conditions × 5 blocks = **60 runs** (≈25 min per pod on a fast host).
- **Correctness:** recall@10 against the existing FP64 oracle (`truth.csv` from
  D1 runs); gate 0.99.
- **Host-speed control:** record `lscpu` and the 10M-add Python loop time. Reject
  hosts slower than 1.5× the fastest D1R host *before* any run, and log each
  rejection (the D1R `b3` lesson).

### Predictions (confirmatory)
- **F1.** The heavy path's kernel-time max/min across blocks is ≤ 1.10 in
  every cell.
- **F2.** The light path's kernel-time max/min across blocks exceeds the heavy
  path's in at least 2 of 3 cells.
- **F3.** In the regression-to-the-mean-free "relative to best" metric
  (`research/scripts/tail_table.py`), the heater lowers the light path's median
  and p90 and does not lower the heavy path's.
- **F4.** Median in-run SM clock is lower for the light path than for the heavy
  path at E ≥ 3,000 with the heater off.

### Rejection criteria
- **F2 and F4 both fail:** FAISS's light path holds boost clocks and reproduces
  as tightly as the heavy path. The effect is then specific to Qenlo's
  WGPU/Vulkan query loop, and the paper must scope its title and abstract to
  "a WGPU query engine".
- **F4 holds but F2 fails:** power states differ but do not matter for FAISS
  kernel time. This is plausible if FAISS's light kernels are launch-bound, and
  it would narrow the claim to memory- or clock-bound kernels.

### Outputs
- `research/data/raw/<date>-faiss/` (tarball per pod, same meta layout as D1R).
- A new FAISS row in `paper/v2/tables/cross_host.tex` and `tail.tex`; a panel
  (d) in Figure 1 if F1–F3 hold.
- An update to `research/decision-memo-2026-09-25.md` §5.

---

## Order and cost
1. **G0** (FAISS feasibility): ~$0.50, one pod, 30 min.
2. **E-FAISS:** 5 pods × ~30 min ≈ $2–3; optional H100 +$1.5.
3. **E-LOCK:** laptop, ~2 h, $0; maintainer runs the clock commands. Optional
   bare-metal Linux arm if a machine with driver access is available.

Both experiments reuse the existing scripts (`run_dvfs_cell.py`,
`analyze_dvfs_d1.py`, `cross_host.py`, `tail_table.py`, `make_tables.py`).
Neither needs changes to the Qenlo crates.
