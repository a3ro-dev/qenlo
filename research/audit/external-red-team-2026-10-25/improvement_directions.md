# Qenlo improvement and research directions (Phases 6–8): literature-grounded portfolio, scoring, flagship experiment spec

Scope note for the report writer. These notes cover Phases 6, 7 and 8 of the audit brief: directions A–J, the scored portfolio, and experiment specifications. Everything under "Cited Findings" has a source. Everything under "Inferences" is my reasoning: numbers derived from the paper's tables are labelled as derived, and every proposed experiment is labelled PROPOSED. Portfolio scores are judgments, not measurements.

Repository snapshot used: `a3ro-dev/qenlo`, shallow clone of the default branch at commit `142e96cf43684a13e95513a161bda4515ad6f1ca` (committed 2026-10-25 15:16 +0530). I only read it to establish what exists; the deep code audit is someone else's job. Repo file links below are pinned to that commit (prefix `https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/`, written `REPO/` below to save space; expand `REPO/` to that prefix).

---

## Q0. What does Qenlo already implement? (Baseline for every proposal)

### Takeaway
Several brief directions are partly built already: two GPU eligibility modes plus a dense-mask mode, a per-adapter router-profile struct, a certified FP32→FP64 CPU path, IVF-Flat/IVF-SQ8 and HNSW (USearch) backends, snapshot+WAL persistence with CRC32 and directory fsync, and a C ABI with zero-copy buffer intake in Python. Three gaps matter most. The GPU top-k is a single-workgroup k-pass scan. The CPU scan is single-threaded and switches to FP64 accumulation above 4,096 eligible rows. The automatic router is a static constant `E < 4096 && batch < 8 → CPU`, with a "replace with persisted per-device autotune profile" TODO.

### Cited Findings
- Workspace crates: `qenlo`, `qenlo-core`, `qenlo-bench`, `qenlo-testkit`, `qenlo-telemetry`, `qenlo-mobile`, `qenlo-ffi`, `qenlo-browser`, plus a Tauri desktop app. SDKs: Python, TypeScript, Go, Kotlin, Swift (Go/Kotlin/Swift marked preview). — [REPO/Cargo.toml](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/Cargo.toml); [REPO/README.md](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/README.md)
- The GPU backend is optional: feature `gpu-wgpu` pulls `wgpu = 30.0.1`. HNSW is optional through `usearch = 2.26.1`. — [REPO/crates/qenlo/Cargo.toml](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo/Cargo.toml)
- Router: `const AUTOMATIC_GPU_MIN_ELIGIBLE_ROWS: usize = 4_096` (lib.rs:56). `route_automatic_to_cpu` returns `eligible_rows < 4096` with the comment "ponytail: static crossover; replace with the persisted per-device autotune profile" (lib.rs:2057–2060). The batch variant routes to CPU only if below the crossover **and** `batch_size < 8` (lib.rs:2063–2073). A `RouterProfile {adapter_name, dimension, batch_size, filter_mode, cached_rows, gpu_min_eligible_rows}` can be installed and "is ignored on mismatch" (lib.rs:109–121, 1101–1107). — [REPO/crates/qenlo/src/lib.rs](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo/src/lib.rs)
- Eligibility planner: `GpuFilterMode {CpuMask, CpuEligibleRows, GpuPredicate}` and `EligibilityRepresentation {Empty, TinyRows (≤64), SortedRows, DenseMask, ShaderPredicate}`. Transfer bytes are `4E` for row lists and `4N` for DenseMask (one u32 per corpus row). The planner also computes `contiguous_runs` (the number of maximal runs of consecutive slot ids) and reports it as a diagnostic (lib.rs:540–571, 602, 629–636). Preparation modes are `LegacyTwoPass`, `OnePass` (default) and `Cached`. — [REPO/crates/qenlo/src/lib.rs](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo/src/lib.rs)
- WGSL (152 lines, one file). `score` runs 256 threads per workgroup, eight rows per workgroup, 32 lanes per row, with a shared-memory tree reduction. The source says it "needs no subgroup extension". `select_topk` runs a 256-thread workgroup that, for each of k outputs, scans all `count` scores (count = E in row mode, N otherwise), tree-reduces, then overwrites the winner's score with FLT_MAX. Selection is dispatched with `queries.len()` workgroups, so one workgroup per query per chunk (gpu.rs:765). Tie-break is distance, then id. — [REPO/crates/qenlo/src/gpu_exact.wgsl](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo/src/gpu_exact.wgsl); [REPO/crates/qenlo/src/gpu.rs](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo/src/gpu.rs)
- `gpu.rs:19`: "ponytail: one workgroup selects each chunk's top-k; hierarchical selection if this dominates". `MAX_CHUNK_ROWS = 131_072`. Timestamp queries are used when supported. — [REPO/crates/qenlo/src/gpu.rs](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo/src/gpu.rs)
- `gpu.rs` also holds an `IvfIndex {nprobe, sq8_vectors, …}` with rerank, reported as `Algorithm::IvfFlat`/`IvfSq8`. `Algorithm` also lists `Hnsw` (USearch). — [REPO/crates/qenlo/src/gpu.rs](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo/src/gpu.rs); [REPO/crates/qenlo/src/usearch_backend.rs](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo/src/usearch_backend.rs)
- CPU: AVX2, AVX2+FMA and NEON dot kernels with runtime detection. The reference path accumulates in FP64. The "certified" path scores in FP32, keeps every row within `cutoff + 2·error_bound` (bound = `4·D·ε/(1−D·ε)·1.001`), and rescores those rows in FP64. It runs only when `eligible ≤ CERTIFIED_FP32_MAX_ROWS = 4_096`, a constant described as "conservative… replaced by the adaptive cost model later". There is no threading in qenlo-core (no rayon/std::thread in the scan). — [REPO/crates/qenlo-core/src/lib.rs](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo-core/src/lib.rs) (lines ~785–830, 936–1029)
- Metadata indexes: a per-user map and one `BTreeSet<(i64 timestamp, u32 slot)>`. A broad timestamp range "abandons the index after a bounded probe" (`RANGE_SCAN_PROBE_DIVISOR = 16`) and falls back to a sequential slot scan. — [REPO/crates/qenlo-core/src/lib.rs](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo-core/src/lib.rs) (lines ~285–303, 594–667, 936–940)
- The in-repo CPU baseline study (A40 cohort, AG News 100k, D=384, batch 1) measured the certified FP32 path at −17% p95 at E=1,000, −28% at E=4,000, and **+10% at E=100,000** against the FP64 baseline. That is why it is capped at 4,096. Single-thread FAISS `IndexFlatIP` (faiss-cpu 1.15.0, AVX2) at unfiltered 100k had p95 11.154 ms against Qenlo's 15.250 ms ("FAISS is therefore 26.9% lower-latency"). — [REPO/research/phase2-cpu-exact-progress.md](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/research/phase2-cpu-exact-progress.md)
- The repo's own decision memo already lists as "undisclosed confounds" that "the CPU arm switches kernels at E=4,096 … is single-threaded, and re-filters per query inside a batch". — [REPO/research/decision-memo-2026-09-25.md](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/research/decision-memo-2026-09-25.md)
- Persistence: a snapshot (`.qdb`) plus WAL (`QENLOWL\0`, version 1). Files are published by rename, `sync_all` is called on files, and directories are fsynced explicitly ("A child fsync is not enough"). `MANIFEST.pending` is renamed to `MANIFEST`, with CRC32 checksums throughout. There is an integration test `tests/recovery.rs` (69 lines). — [REPO/crates/qenlo/src/storage.rs](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo/src/storage.rs); [REPO/crates/qenlo/tests/recovery.rs](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo/tests/recovery.rs)
- FFI/SDK: a C ABI (`qenlo_search_results_new` returns typed result buffers). The Python SDK uses ctypes (`_native.py`). `add_buffer` "Accepts NumPy arrays or another buffer provider without copying" (it falls back to a copy for read-only views), and a `torch.py` helper wraps a snapshot buffer with `torch.frombuffer`. It is not PyO3. — [REPO/crates/qenlo-ffi/src/lib.rs](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo-ffi/src/lib.rs); [REPO/sdk/python/src/qenlo/_collection.py](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/sdk/python/src/qenlo/_collection.py)
- The benchmark harness assigns timestamps after shuffling rows for `--distribution independent` (bench main.rs:556–596), and the DVFS cell runner passes `--distribution independent` with `nvidia-smi … power.draw … -lms 50` telemetry. The paper's eligible rows are therefore a random subset of slots, not a contiguous slot range. — [REPO/crates/qenlo-bench/src/main.rs](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo-bench/src/main.rs); [REPO/research/scripts/run_dvfs_cell.py](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/research/scripts/run_dvfs_cell.py)
- Raw data in the repo totals about 2.0 GB (`research/data/raw/`), including DVFS D1/D1R/H100/lock/split archives and the alpha5 held-out router archive (measured with `du` on the clone). — [REPO/research/data/raw](https://github.com/a3ro-dev/qenlo/tree/142e96cf43684a13e95513a161bda4515ad6f1ca/research/data/raw)

### Inferences
- **Router discrepancy for the code auditor.** The paper's Appendix C describes the earlier rule as "GPU if E·D·B ≥ 10⁶", which at D=384, B=1 means E ≥ ≈2,604. Current code routes on E < 4,096 and B < 8, independent of D. Both are scalar thresholds, but they are different rules. Whoever audits the code should confirm which rule produced which archive.
- **Possible heavy-path transfer discrepancy.** The paper says the heavy path "uploads per-row liveness (4N bytes)", but the planner sets `transfer_bytes = 0` for `ShaderPredicate`. Either liveness is device-resident (uploaded at rebuild) and the paper's text is stale, or the planner under-reports. Not verified here.
- `RouterProfile` is keyed by `adapter_name`. The paper shows a 4.6× light-kernel spread across five pods carrying the *same* adapter (RTX 4090). A profile keyed on the adapter name cannot capture that variance, which is direct motivation for online calibration (A/B).
- `contiguous_runs` is computed and reported but nothing uses it to choose a cheaper encoding. That is a ready-made hook for direction D/J.

### Gaps
- I did not determine which commit produced each paper table; that is the code-audit researcher's job.
- I did not check whether any crash-injection or fault-injection harness exists beyond `tests/recovery.rs`. A grep for "crash"/"fault" matched several files, but I did not inspect them.

---

## Q-A. GPU-aware adaptive execution: can measured, state-aware, uncertainty-aware dispatch beat a fixed threshold without costing more than it saves?

### Takeaway
Adaptive CPU/co-processor operator placement and bandit-based plan steering are well established (HyPE/Ocelot, Breß SIGMOD'16, Bao, Neo, Balsa), so "learn the router" is not novel by itself. What is new for Qenlo is the paper's evidence that the GPU arm's cost depends on the GPU's power state, and that the router's own recent decisions plausibly move that state. In the language of performative prediction or restless bandits, the action changes the environment. I found no prior work on a CPU/GPU operator router that models that feedback. That makes it the strongest candidate for a novel research contribution, but it depends on first fixing the CPU baseline and the top-k kernel (see Q-J1 and Q-C).

### Cited Findings
- HyPE learns operator cost models from observed execution times (STEMOD estimator, ALRIC algorithm selector, HOPE optimizer) and places operators on CPU or GPU. — [Breß et al., PVLDB 6(12) 2013](https://www.vldb.org/pvldb/vol6/p1398-bress.pdf)
- Ocelot/HyPE feeds measured operator times back to HyPE "to adjust the learned cost models" and accounts for "the load condition across (co-)processors". — [Breß et al., PVLDB 7(13) 2014 demo](https://www.vldb.org/pvldb/vol7/p1609-bress.pdf)
- Breß, Funke, Teubner (SIGMOD 2016) show that co-processor placement must be robust. Heap contention can slow queries "by up to a factor of six"; their fixes are data-driven operator placement and query chopping. The paper also notes that a learning-based estimate "can take a non-negligible amount of time (in the order of several micro seconds)". — [ACM DL](https://dl.acm.org/doi/10.1145/2882903.2882936); [preprint PDF](https://www.tu.berlin/fileadmin/www/10002226/Publications/Papers/p1891-breb_preprint.pdf)
- Bao frames hint-set selection as a contextual multi-armed bandit solved with Thompson sampling over a tree-convolution model, and reports better tail performance and fast adaptation. — [Marcus et al., SIGMOD 2021 PDF](https://people.csail.mit.edu/hongzi/content/publications/BAO-Sigmod21.pdf); [arXiv 2004.03814](https://arxiv.org/abs/2004.03814)
- Neo is a deep-learning query optimizer bootstrapped from an expert optimizer. — [Marcus et al., PVLDB 12(11) 2019](https://www.vldb.org/pvldb/vol12/p1705-marcus.pdf)
- Balsa learns without expert demonstrations, using simulation, safe exploration and on-policy learning. Its reported speedups over PostgreSQL are 2.1×/1.7× (JOB train/test), against Bao's 1.6×/1.8×. — [Yang et al., SIGMOD 2022, arXiv 2201.01441](https://arxiv.org/html/2201.01441v2)
- Performative prediction formalizes predictions that "influence the outcome they aim to predict". A stateful extension models the response as depending on both the deployed model and the current state. — [Perdomo et al., ICML 2020](https://proceedings.mlr.press/v119/perdomo20a.html); [Brown et al., AISTATS 2022](https://proceedings.mlr.press/v151/brown22a.html)
- Restless bandits, where arm state evolves with actions, have Whittle-index Q-learning methods. — [Avrachenkov & Borkar, arXiv 2004.14427](https://arxiv.org/abs/2004.14427)
- The paper's evidence that GPU-arm cost is state-dependent: the light kernel varied up to 4.2× between fresh processes on the laptop and 4.6× across five RTX 4090 pods, against ≤1% and 7% for the heavy path. Locking clocks at boost made the light kernel 2.9–5.0× faster on the laptop. Fig. 1b shows the light path settling near 500 MHz SM clock while the heavy path holds boost. — QENLO-RESEARCH-PAPER.pdf, Abstract, Table 1, §7–8, Fig. 1 (local file /home/claude/QENLO-RESEARCH-PAPER.pdf)
- Mageirakos et al. (2026) conclude that "the fastest method … can depend on many parameters: index type, batch size, scale factor, target recall, host hardware", and expect DBMS optimizers to "estimate data movement, operator runtimes, batch size, GPU memory budget, and recall target". — [arXiv 2605.15957](https://arxiv.org/html/2605.15957v1)
- Hedged requests (issue a duplicate after a delay and use whichever finishes first) are an established tail-latency technique. — [Dean & Barroso, CACM 2013](https://cacm.acm.org/research/the-tail-at-scale)
- A DVFS-aware latency model for DNN inference on GPUs exists (offline modelling, edge partitioning), but it models frequency as a chosen knob, not as state induced by the scheduler. — [arXiv 2502.06295](https://arxiv.org/abs/2502.06295)

### Inferences
1. **Limitation today.** The router is a compile-time constant (E < 4,096, B < 8) with an optional static profile keyed by adapter name. It cannot see host variance, power state, recent GPU activity or prediction uncertainty. The paper's held-out failure (8/16 wrong, max regret 235.7%) and the 4.6× same-model spread show that any static calibration encodes one host's power behaviour at one moment.
2. **Proposed change (PROPOSED).** Replace the threshold with a three-layer policy.
   - (a) An analytic per-arm cost model: `t_cpu(E, D, k, threads)`, and `t_gpu(E, D, k) = t_fixed + t_upload(bytes) + t_kernel(E·D, state)`.
   - (b) Online calibration that updates the coefficients from every executed query (the system already records device timestamps, upload bytes and materialization time). Add a short per-process probe at open time (e.g. 50 forced alternating queries at two E values).
   - (c) A risk-aware decision rule. Choose GPU only if the upper conformal bound of `t_gpu − t_cpu` is negative, or minimize expected cost plus λ·CVaR. Run a small exploration budget (Thompson or ε-greedy) restricted to the uncertainty band near the crossover.
   - State features: time since the last GPU submission, an EWMA of recent GPU kernel time relative to its locked-boost reference, recent duty cycle, and (where cheap) a direct NVML clock read.
3. **What is novel.** Learned placement and bandit routing are not novel. Possibly novel: (i) treating GPU power state as endogenous to the routing policy (routing light work to the GPU keeps it warm; routing to CPU lets it cool), with the policy evaluated on that feedback; (ii) conformal risk control to bound regret on new hosts. I found no system paper doing (i) for CPU/GPU placement; see Gaps.
4. **Mechanism.** Low regret comes from (a) tracking each host's actual GPU state instead of a training-time average, and (b) declining the GPU when its latency distribution is wide and the CPU's is tight. The paper's CPU path has no bimodality; the light GPU path does.
5. **Benefits / regressions.** Benefits: workloads with E spread across the crossover band, multi-host fleets, bursty arrivals, and hosts where co-located GPU work changes the state. Regressions: exploration costs on stationary workloads, per-query overhead (feature collection must stay well under the ~8–10 µs CPU latency at E=100 in Table 8), and oscillation if the policy chases its own induced state.
6. **Smallest falsifying experiment (PROPOSED).** On the laptop and two 4090 pods, replay the alpha5 16-workload held-out suite plus a log-uniform E stream (100–30k) with Poisson arrivals at three mean gaps (1 ms, 10 ms, 100 ms). Compare static (4,096), offline-fitted, and online-calibrated routers. **Refuted if** the online router's max regret is not below the static router's, or its p50 overhead exceeds 5 µs, on ≥2 of 3 hosts.
7. **Baselines/datasets.** Static 4,096; the paper's E·D·B ≥ 10⁶; forced-CPU; forced-GPU; an offline per-host fit (the RouterProfile approach); and the oracle (best forced arm in the same block). Data: AG News MiniLM 100k already in the harness, plus a D=768/1024 set to vary D.
8. **Cost/risk.** Medium. The router lives in one function, and telemetry already exists. Risk: the effect becomes moot if the strengthened CPU (Q-J1) dominates at N=100k, so the study must also scale N.
9. **Publishable?** Yes, if the endogenous-state effect is shown to change routing decisions and the online router cuts held-out max regret across hosts. Venue fit: DaMoN, EuroMLSys, or a SIGMOD/VLDB short or industrial paper.
10. **Evidence required.** Leave-one-host-out evaluation on ≥3 GPU models and ≥2 CPU families; fresh-process replication; regret distributions with block-bootstrap intervals; and an ablation showing the state feature matters (router with and without state features).

### Gaps
- I found no prior work on CPU/GPU operator placement that treats GPU DVFS state as induced by the placement policy. "Not found" is not proof of absence; the novelty-audit researcher should run a dedicated priority search.
- No public data shows how fast NVIDIA's governor ramps up or down after an idle gap at the µs–ms scale relevant here. The paper's 50 ms nvidia-smi sampling cannot resolve it.

---

## Q-B. Robust CPU/GPU crossover prediction across hosts

### Takeaway
The crossover is a difference of two noisy, host-specific latency distributions. A point threshold fitted on development hosts will keep failing on new hosts. The literature supports per-host online calibration plus distribution-free uncertainty (conformal prediction or conformal risk control) over a richer model that pools across hosts. The paper's data also suggests that much of the measured crossover comes from the CPU kernel switch at E = 4,096 (see Q-J1). Any crossover model fitted before fixing the CPU baseline is modelling an artifact.

### Cited Findings
- Split conformal prediction gives finite-sample, distribution-free coverage under exchangeability. — [Angelopoulos & Bates, "A Gentle Introduction to Conformal Prediction…", arXiv 2107.07511](https://arxiv.org/html/2107.07511v6)
- Conformal risk control generalizes this to bounding the expected value of a monotone loss at a user-chosen level α. — [Angelopoulos et al., conformal-risk repo / arXiv 2208.02814](https://github.com/aangelopoulos/conformal-risk)
- Exchangeability is violated for time series. Conformal methods for non-exchangeable data reweight calibration data or adapt residual distributions over time. — [arXiv 2511.13608](https://arxiv.org/abs/2511.13608)
- Repo evidence that labels near parity are noise-dominated: in the alpha5 held-out archive, "the automatic arm differed from the forced arm of the same backend by more than 25% in 9 of 16 cells", and "two of the eight 'wrong' labels were within 3.2% regret". — QENLO-RESEARCH-PAPER.pdf, Appendix C
- The repo's decision memo: "Same-cell A/A reaches ~2× (E0; R1 9/16 cells >25%)". — [REPO/research/decision-memo-2026-09-25.md](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/research/decision-memo-2026-09-25.md)
- WebGPU per-dispatch API overhead is 24–36 µs on Vulkan and 32–71 µs on Metal, against 3–10 µs CUDA launch latency, and is "consistent across four GPU vendors on the same backend". — [Maczan, arXiv 2604.02344](https://arxiv.org/abs/2604.02344)
- The paper's end-to-end medians (Table 8) give a GPU fixed cost of ≈0.10 ms on the 4090 pods and ≈0.17 ms on the laptop at E=100 (light path 0.103 / 0.172 ms against CPU 0.008 / 0.010 ms). — QENLO-RESEARCH-PAPER.pdf, Table 8

### Inferences
1. **Limitation.** The routing input is E alone (plus B < 8). D, k, the host's CPU class, upload bandwidth, adapter, process state and predicate shape are all missing.
2. **Proposed change (PROPOSED).**
   - Use a factored, mostly analytic model: `t_cpu = a_c + b_c·E·D/threads_eff`, piecewise by CPU kernel (certified FP32 vs FP64). `t_gpu = a_g + u·bytes + c_g(state)·E·D + s_g(E, k)`, where `s_g` is the selection term that currently dominates.
   - Fit a_c and b_c from a CPU micro-probe at open (microseconds; deterministic). Fit a_g and u from a dispatch/upload probe. Treat c_g(state) as a latent variable updated online.
   - Calibrate a conformal interval on residuals of the *difference* `t_gpu − t_cpu` from the most recent W queries (a sliding window, to handle drift), and route GPU only if the upper bound is < 0.
   - For cross-host transfer: a hierarchical prior over coefficients (host as a random effect) learned from the fleet archive, with per-host posterior updates from the probe. This is few-shot calibration, not a global model.
3. **Novel?** Mostly engineering. Conformal-guarded routing for CPU/GPU placement might be new in application, but the method is standard. The scientific contribution would be showing *which* features transfer across hosts and which must be measured per host.
4. **Mechanism.** The fixed GPU cost (dispatch, sync, readback) and the CPU slope are host constants that a probe measures cheaply. Only the state-dependent GPU slope needs online tracking.
5. **Benefits / regressions.** Benefits: new hosts and new D or k. Regressions: very short-lived processes, where the probe overhead is not amortized. Mitigation: cache the probe result keyed by a hardware fingerprint (Q-I) with a TTL.
6. **Smallest falsifying experiment (PROPOSED).** Leave-one-host-out on the existing archives (laptop, 4090 host A, 5 pods, H100). Fit on n−1 hosts, predict the winner per (E, host) on the held-out host. Compare against the static rules, measuring classification error weighted by regret. **Refuted if** the factored model plus probe does not cut weighted regret against the static rules on the held-out host in ≥3 of 4 folds. This can run on already-collected data, apart from probe features the archives lack, which need a small new collection.
7. **Baselines/datasets.** As in Q-A, plus a global gradient-boosted model on raw features (to show the "fits training hosts, fails new hosts" failure mode).
8. **Cost/risk.** Low-medium. The risk is that, once the CPU is fixed, the crossover at N=100k moves beyond E=30k and the problem changes; so include N ∈ {100k, 1M}.
9. **Publishable?** As part of the flagship (Q-Flagship), not standalone.
10. **Evidence required.** Leave-one-host-out, leave-one-D-out and leave-one-arrival-process-out results; coverage of conformal intervals reported against nominal.

### Gaps
- Conformal guarantees assume exchangeability. Power-state dynamics are autocorrelated, and the policy itself shifts the distribution (performativity), so coverage claims need empirical validation. I found no source applying conformal control to self-influencing systems routing; adaptive conformal variants for drift exist (see the arXiv 2511.13608 survey).

---

## Q-C. Better exact GPU top-k in WGSL

### Takeaway
Qenlo's selection kernel is the textbook worst case. It makes k sequential passes over all candidates inside one workgroup, so device work and latency scale as O(k·E) on a single SM. It dominates light-path device time (laptop, locked boost, E=10k: selection 0.113 ms against scoring 0.034 ms). Every published GPU top-k design uses many workgroups with local selection plus merge, radix/bucket selection, or threshold filtering. A fused "score + per-workgroup register top-k + small merge" fits WGSL without subgroups, because k = 10 is tiny. It also raises GPU occupancy, which may itself change the power state that drives the paper's main result. That interaction is a research question in its own right (R2).

### Cited Findings
- AIR Top-K (Adaptive and Iteration-fused Radix) and GridSelect (NVIDIA, SC'23). At batch 1, AIR Top-K reports 1.98–21.48× speedup over the previous radix top-K and 1.44–7.34× over state-of-the-art methods. It "employs an iteration-fused design to minimize CPU-GPU communication and device data access". — [Zhang, Naruse, Li, Wang, SC23](https://dl.acm.org/doi/10.1145/3581784.3607062); [SC23 program page](https://sc23.supercomputing.org/proceedings/tech_paper/tech_paper_pages/pap294.html)
- RadiK (ICS'24) is a radix top-k supporting large k: "up to 2.5× speedup over the prior art for non-batch queries and up to 4.8× … for batch queries". It notes prior merge-based methods "rely heavily on the fast but size-limited on-chip memory". — [Li et al., ICS 2024](https://dl.acm.org/doi/10.1145/3650200.3656596); [arXiv 2501.14336](https://arxiv.org/abs/2501.14336)
- Dr. Top-k uses delegate-centric filtering (maximum delegate, delegate-based filtering, β-delegate) that can "reduce the workload for top-k up to more than 99%". — [Gaihre et al., SC21](https://dl.acm.org/doi/10.1145/3458817.3476141); [arXiv 2109.08219](https://arxiv.org/abs/2109.08219)
- FAISS GPU k-selection (WarpSelect/BlockSelect) keeps per-warp priority queues in registers, using odd-size merging networks and thread-local buffers sorted by bitonic networks. It runs "at up to 55% of theoretical peak performance". — [Johnson, Douze, Jégou, IEEE TBD 2021 (summary)](https://zhangtemplar.github.io/faiss); [NVIDIA GTC slides on WarpSelect/BlockSelect and radix select](https://live.nvidia.cn/gtc-od/attachments/CNS20315.pdf)
- cuVS exposes `select_k(…, SelectAlgo algo = kAuto, …)` for batched row-wise top-k and cites AIR top-K as a core primitive. — [cuVS selection API](https://docs.rapids.ai/api/cuvs/stable/cpp_api/selection/); [cuVS references](https://docs.nvidia.com/cuvs/user-guide/references)
- wgpu `FeaturesWGPU::SUBGROUP` "Allows compute and fragment shaders to use the subgroup operation built-ins … (except barriers)" on Vulkan, DX12, Metal and WebGPU. The `subgroups` feature "has been added to WebGPU, but there may be differences … so it has not yet been moved to FeaturesWebGPU". `SUBGROUP_BARRIER` is separate. — [wgpu docs: FeaturesWGPU](https://wgpu.rs/doc/wgpu/struct.FeaturesWGPU.html); [wgpu_types docs](https://wgpu.rs/doc/wgpu_types/features/struct.FeaturesWGPU.html)
- WGSL defines `subgroupMin` and related built-ins (§17.12), which require `enable subgroups;`. — [W3C WGSL](https://www.w3.org/TR/WGSL)
- Chrome shipped WebGPU subgroups in Chrome 134 (post dated 2025-02-26), per a third-party validation note that cites the Chrome blog. Chrome 151–152 (Aug 12, 2026) added `subgroup-size-control` with a WGSL `@subgroup_size` attribute; without it "the subgroup size is selected dynamically by the GPU driver and remains unknown prior to execution". — [vercel-labs/vgpu issue #9](https://github.com/vercel-labs/vgpu/issues/9); [Chrome 151–152 blog](https://developer.chrome.com/blog/new-in-webgpu-151-152); [Chrome 129 blog: subgroupMin etc.](https://developer.chrome.com/blog/new-in-webgpu-129)
- WGSL atomics all use relaxed memory ordering. `workgroupBarrier`/`storageBarrier` "cannot be used to synchronize memory accesses between different workgroups" (described as a Metal limitation). — [W3C WGSL](https://www.w3.org/TR/WGSL); [gpuweb discussion #3935](https://github.com/gpuweb/gpuweb/discussions/3935)
- wgpu has had `SHADER_INT64_ATOMIC_MIN_MAX` (64-bit atomic min/max in storage) and `SHADER_INT64_ATOMIC_ALL_OPS` as native-only features since v22. A gpuweb draft proposes `atomic<vec2u>` 64-bit min/max for WebGPU. — [wgpu v22.0.0 release notes](https://newreleases.io/project/github/gfx-rs/wgpu/release/v22.0.0); [gpuweb proposal atomic-64-min-max](https://github.com/gpuweb/gpuweb/blob/main/proposals/atomic-64-min-max.md)
- Reducing dispatch count matters on WebGPU. Cutting dispatches from 876 to 564 with the same shaders improved LLM throughput by 53%. — [Maczan, arXiv 2608.08730](https://arxiv.org/pdf/2608.08730)
- Paper's E-SPLIT, laptop, light path, both clocks high (HH): selection 0.024 / 0.044 / 0.113 ms against scoring 0.007 / 0.012 / 0.034 ms at E = 1k / 3k / 10k. Heavy path selection is ≈0.95 ms at every E. — QENLO-RESEARCH-PAPER.pdf, Table 5
- The IVF-RaBitQ GPU work uses "a fused cluster-local search kernel … which integrates distance filtering, refinement, and top-K". — [arXiv 2602.23999](https://arxiv.org/html/2602.23999v1)

### Inferences
1. **Limitation (derived from code).** `select_topk` does about k·count global reads plus k·(8 shared-memory barrier rounds + global/storage barriers) in **one** workgroup. At E=10k, k=10 that is 100k score reads, 100k id-gather reads and about 80 barrier rounds, all on one SM. An RTX 4050 Laptop has 20 SMs, an RTX 4090 128 and an H100 SXM 132 (my recollection, not verified this session), so the selector leaves about 95–99% of the GPU idle. The score buffer is also written to global memory and then reread k times.
2. **Proposed change (PROPOSED). "Light path v2" kernel family:**
   - **Fused score + local top-k.** Each workgroup scores a tile of T rows (e.g. T = 256–2,048) and keeps a per-thread or per-workgroup top-k in registers or shared memory. With k = 10, a sorted insertion list of 10 (distance, id) pairs per thread is register-resident. The workgroup then reduces 256·k to k with a bitonic or merge network in shared memory and writes k candidates per workgroup. The full score array never goes to global memory.
   - **Merge.** W·k candidates (W = ⌈E/T⌉; E=10k, T=1,024 gives ≈100 candidates) are merged by a second tiny dispatch or by the host, which already merges chunk candidates. Host merge removes a dispatch but adds readback bytes (≈ W·k·16 B = 16 KB at W=100: negligible).
   - **Threshold filtering (Dr. Top-k / AIR-style).** Optionally publish a global running k-th-best bound through `atomicMin` on a u32 key. For non-negative distances the IEEE-754 bit pattern is order-preserving as u32, and cosine distance 1 − dot lies in [0, 2]. Workgroups then skip insertions above the bound. This needs only core 32-bit atomics. Packing (distance, row) into one 64-bit key needs native-only `SHADER_INT64_ATOMIC_MIN_MAX`.
   - **Subgroups (optional fast path).** `subgroupMin`/shuffles for intra-warp reduction, gated on `Features::SUBGROUP`, with the shared-memory version as the portable fallback. Do not assume a subgroup size; the WGSL value is only known at runtime unless size control is available.
   - **Radix select** only if k grows (Qenlo caps k at `MAX_K = 64`). For k ≤ 64, register/shared-memory selection is the published choice (FAISS WarpSelect/BlockSelect support k up to ~1–2k; RadiK targets large k).
3. **Novel?** The algorithms are not novel; this is porting known designs to WGSL. Possibly novel and publishable: (i) a characterization of portable WGSL top-k across vendors, with and without subgroups, at batch 1; (ii) the **power-state interaction**. A multi-workgroup kernel keeps more SMs busy for less time, and whether that raises or lowers the clock the governor picks is unknown and directly relevant to the paper's thesis (R2).
4. **Mechanism.** Work drops from O(k·E) serialized on one SM to O(E) spread over W workgroups, plus O(W·k) merge. Score-buffer global traffic is removed. Dispatches go from 2 to 1–2 per chunk, saving up to ~24–36 µs each by the Maczan estimate.
5. **Benefits / regressions.** Benefits: every light-path query, and the heavy path if moved to the same selector. The heavy-path naive selector is the paper's negative control, so keep the naive kernel available as a named control arm, not the default. Regressions: tiny E (≤64, `TinyRows`), where one workgroup is already enough and a second merge stage adds overhead; and register pressure for k = 64 (use shared memory there).
6. **Smallest falsifying experiment (PROPOSED).** Laptop, both clocks locked at boost (existing E-LOCK infrastructure), E ∈ {1k, 3k, 10k, 30k}, k ∈ {10, 64}, five fresh-process blocks. Compare the current `select_topk` against fused local-top-k with host merge. **Refuted if** device selection+scoring time is not at least 2× lower at E ≥ 3k, or result ids are not bit-identical to the FP64-oracle top-k under the same tie-break.
7. **Baselines.** Current kernel, cuVS `select_k`/brute-force on the same GPU via CUDA (an upper-bound reference showing what tuned CUDA achieves), and the FAISS GPU flat index.
8. **Cost/risk.** Low-medium: about 150–300 WGSL lines plus host-merge changes. Risks: WGSL compiler differences across backends (naga to SPIR-V/MSL/HLSL), and correctness of tie-breaking when merging. Property tests against the existing FP64 oracle tests mitigate the latter.
9. **Publishable?** The engineering alone fits a workshop at most. Combined with R2 (does the better kernel change power state?) it becomes a meaningful extension of the paper.
10. **Evidence required.** Device and end-to-end distributions on ≥3 GPU vendors/backends (NVIDIA Vulkan, AMD or Intel Vulkan/DX12, Apple Metal); locked and unlocked clocks; exactness proof (bitwise id equality against the oracle on all test queries).

### Gaps
- I found no published WGSL/WebGPU top-k benchmark comparable to AIR/RadiK; WebGPU top-k performance is undocumented in the literature I found.
- Subgroup support quality in wgpu 30 on specific drivers (Windows NVIDIA Vulkan, the laptop's path) is not verified.
- SM counts above come from memory, not a source checked this session.

---

## Q-D. Filter-aware storage and execution: bitmaps, id-lists, predicates, ranges

### Takeaway
Qenlo already chooses among id-list, dense mask and in-shader predicate, but the encodings are wasteful. DenseMask uses a u32 per row (4N bytes), where a bitmap needs N/8 (32× less), and nothing exploits contiguity. The relational literature (Ngom et al. DaMoN'21; Roaring; zone maps/SMAs) gives clear rules. If rows were physically clustered by timestamp, a range predicate would become one (start, len) pair: zero host materialization, eight bytes of upload, and a dense, coalesced scan. In the paper's benchmark the eligible rows are a random subset of slots, because timestamps are assigned after a shuffle, so this benefit applies only after a layout change or to workloads with insertion-ordered timestamps (e.g. agent memory).

### Cited Findings
- Ngom et al.: "bitmaps perform better for operations that can be vectorized using SIMD instructions and … selection vectors perform better on all other operations"; the optimal choice "is a function of many factors". — [DaMoN 2021](https://dl.acm.org/doi/10.1145/3465998.3466009); [CMU PDL abstract](https://www.pdl.cmu.edu/PDL-FTP/Database/ngom-damon2021_abs.shtml)
- Roaring bitmaps (compressed hybrid containers) are described in Chambi et al. 2016; a Rust port, `roaring` (roaring-rs), is on crates.io. — [Softw. Pract. Exper. 46:709–719](https://onlinelibrary.wiley.com/doi/abs/10.1002/spe.2325); [roaring-rs](https://github.com/RoaringBitmap/roaring-rs); [crates.io/roaring](https://crates.io/crates/roaring)
- Weaviate resolves "the filter against the inverted index into a roaring bitmap before the vector search starts". — [Weaviate PathSeer blog, 2026-10-14](https://weaviate.io/blog/pathseer-filtered-vector-search)
- cuVS brute force accepts a device bitmap prefilter, "one bit per (query, vector) pair". — [cuVS brute-force guide](https://docs.nvidia.com/cuvs/user-guide/api-guides/indexing-guide/brute-force)
- Small Materialized Aggregates (zone maps) exploit implicit clustering to skip buckets on selection predicates. — [Moerkotte, VLDB 1998](https://www.vldb.org/conf/1998/p476.pdf). Oracle zone maps and partition pruning — [Ziauddin et al., PVLDB 10(12)](https://www.vldb.org/pvldb/vol10/p1622-ziauddin.pdf); Snowflake min/max pruning — [arXiv 2504.11540](https://arxiv.org/pdf/2504.11540)
- Milvus "clustering compaction" redistributes entities into segments by a scalar clustering key and records a PartitionStats mapping, so searches with a key filter prune segments; "as the search range narrows … the prune ratio increases". — [Milvus docs](https://milvus.io/docs/clustering-compaction.md)
- Qenlo's planner computes `contiguous_runs` but uses only `TinyRows`/`SortedRows`/`DenseMask`/`ShaderPredicate`; DenseMask transfer is `corpus_size * 4` bytes. — [REPO/crates/qenlo/src/lib.rs](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo/src/lib.rs) (≈lines 540–571)
- The benchmark shuffles rows before assigning timestamps for the "independent" distribution, which the DVFS runs use. — [REPO/crates/qenlo-bench/src/main.rs](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo-bench/src/main.rs) (556–596); [REPO/research/scripts/run_dvfs_cell.py](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/research/scripts/run_dvfs_cell.py)

### Inferences
1. **Limitation.** (i) DenseMask is 400 KB per query at N=100k where a bitmap is 12.5 KB. (ii) A row list costs 4E bytes plus a host BTree traversal and sort. The paper times host materialization separately; Table 8's CPU times include it. (iii) Contiguity is measured but never used.
2. **Proposed change (PROPOSED).**
   - (a) Add `Runs` (list of (start, len) u32 pairs) and `Bitmap` (N/8 bytes) representations. Pick by estimated cost: `bytes_upload + host_build + device_gather_penalty(runs, E)`.
   - (b) An optional **clustered layout**: keep slots ordered by timestamp, using sorted segments with periodic compaction (an LSM-style merge, like Milvus clustering compaction). A timestamp range then maps to O(#segments) runs, ideally one.
   - (c) For range-only predicates on a clustered layout, the shader iterates `row = start + i` directly: no eligibility buffer, and fully coalesced vector reads.
   - (d) Keep the id-list for scattered predicates (user equality on interleaved users).
3. **Novel?** Low; these are standard database techniques. A modest new element is joint selection of filter encoding *and* compute device under one cost model. The paper itself notes this mirrors Ngom et al.
4. **Mechanism.** It removes host materialization (O(E log N) BTree walk plus sort), cuts upload from 4E to 8·runs bytes, and turns gathers into streaming reads. That improves memory coalescing on GPU and hardware prefetching on CPU.
5. **Benefits / regressions.** Benefits: time-range-dominated workloads (agent memory, logs), and append-mostly data where insertion order ≈ timestamp order. Regressions: write-heavy workloads with out-of-order timestamps (compaction cost and write amplification), user-equality predicates on a timestamp-clustered layout (rows scatter by user), and slot-order stability assumptions in the WAL or ids.
6. **Smallest falsifying experiment (PROPOSED).** Same AG News 100k, but with a `--distribution clustered` variant (timestamps monotone in slot order). Compare SortedRows against Runs for E ∈ {100, 1k, 10k, 30k} on CPU and GPU light path, measuring host materialization, upload bytes, device and end-to-end time. **Refuted if** end-to-end p50 improves by < 10% at E ≥ 3k on both CPU and GPU.
7. **Baselines/datasets.** Current SortedRows/DenseMask, and the `roaring` crate bitmap for the scattered case. For real time-correlated embeddings, use any timestamped corpus (e.g. news embeddings with publish dates). AG News has no real timestamps; the paper's are synthetic.
8. **Cost/risk.** Runs/Bitmap encodings: low. Clustered layout: medium-high, because it touches storage format, compaction, WAL replay and slot-id stability (needs Q-G testing).
9. **Publishable?** Not alone. As part of R3 (exact range-filtered top-k with zero materialization plus a layout cost model) it could be a workshop paper.
10. **Evidence required.** Mutation-cost and recovery-time measurements alongside query gains; correctness under concurrent writes; and a write-amplification curve against timestamp disorder.

### Gaps
- I did not measure what fraction of real agent-memory workloads have insertion-ordered timestamps; no source found.

---

## Q-E. Hybrid and approximate retrieval: when does ANN help, and when does exact brute force win under restrictive filters?

### Takeaway
At Qenlo's scale (N = 100k, E ≤ 30k), the literature and production systems agree that exact brute force over the filtered set is the right answer for restrictive filters. Qdrant, Weaviate and Milvus all switch to a full or flat scan below a threshold, and iRangeGraph concedes that pre-filtering is optimal for consistently small ranges. Range-filtered ANN is a crowded, fast-moving area (SeRF, iRangeGraph, UNIFY, WinFilter, WoW, RangePQ+, dynamic RFANNS, SIEVE). Qenlo should not build a new filtered-ANN index. It can usefully position its exact path as the ground truth or fallback and keep its existing IVF/HNSW as opt-in for large N.

### Cited Findings
- Qdrant: "if the vectors of the estimated matches take up less than `full_scan_threshold`, Qdrant scores each match directly and returns exact results". `full_scan_threshold: 10000` is in KB ("1Kb = 1 vector of size 256"). ACORN is opt-in for strict combined filters. — [Qdrant filtering guide](https://qdrant.tech/documentation/search-patterns/vector-search-filtering); [Qdrant indexing config](https://search.qdrant.tech/md/documentation/manage-data/indexing?s=vector-index); [Qdrant ACORN article 2026-08-07](https://qdrant.tech/articles/filtered-vector-search-acorn/)
- Weaviate: `flatSearchCutoff` defaults to 40,000 and switches to flat brute force when a filter is restrictive. `filterStrategy` is `acorn` by default as of v1.34, with `sweeping` and `pathseer` (added v1.40). — [Weaviate vector index config](https://docs.weaviate.io/weaviate/config-refs/indexing/vector-index); [Weaviate filtering concepts](https://docs.weaviate.io/weaviate/concepts/filtering)
- pgvector post-filters approximate index results: "If a condition matches 10% of rows, with HNSW and the default `hnsw.ef_search` of 40, only 4 rows will match on average". Iterative index scans arrived in 0.8.0. — [pgvector README](https://github.com/pgvector/pgvector)
- Milvus "automatically falls back to a brute-force scan" for extremely selective filters. — [Milvus blog 2025-05-12](https://milvus.io/blog/how-to-filter-efficiently-without-killing-recall.md)
- ACORN is predicate-agnostic filtered HNSW. — [Patel et al., PACMMOD 2024, arXiv 2403.04871](https://arxiv.org/html/2403.04871v1) (as linked from Weaviate docs)
- Range-filtered ANN:
  - SeRF segment graph — [PACMMOD 2024](https://dl.acm.org/doi/10.1145/3639324)
  - iRangeGraph, which states "unless the query ranges are consistently small (in this case, the RFANN query is intrinsically simple and Pre-filtering is the optimal solution)" — [SIGMOD 2025, arXiv 2409.02571](https://arxiv.org/abs/2409.02571)
  - UNIFY (SIG/HSIG) — [PVLDB 18 p1118](https://www.vldb.org/pvldb/vol18/p1118-yao.pdf)
  - WinFilter / β-Window Search Tree, a segment tree with an ANN index per node — [Engels et al., ICML 2024](https://proceedings.mlr.press/v235/engels24a.html)
  - WoW incremental window graph — [arXiv 2508.18617](https://arxiv.org/abs/2508.18617)
  - Dynamic RFANNS — [PVLDB 18 p3256](https://www.vldb.org/pvldb/vol18/p3256-deng.pdf)
  - RangePQ+ — [CUHK PDF](https://www1.se.cuhk.edu.hk/~swang/V3mod-152-RangePQ.pdf)
  - SIEVE index collection with a brute-force-vs-index strategy picker — [PVLDB 18 p4723](https://www.vldb.org/pvldb/vol18/p4723-li.pdf)
- An experimental study finds "segmented graph indexing excels in range filtering scenarios" over selectivities 0.1%–100%. — [Li et al., arXiv 2508.16263 (PACMMOD 2025)](https://arxiv.org/html/2508.16263v1)
- Survey: pre-filtering "is preferred when the filter result is small". — [Caminal et al., PVLDB 18 p5488](https://www.vldb.org/pvldb/vol18/p5488-caminal.pdf)
- cuVS brute-force guide: brute force "works well when exact results matter, the dataset is small enough to scan, or filters remove most of the dataset". — [cuVS brute-force](https://docs.nvidia.com/cuvs/user-guide/api-guides/indexing-guide/brute-force)

### Inferences
1. **Limitation.** None important at N = 100k. Qenlo already has IVF-Flat/SQ8 (GPU module) and USearch HNSW. The gap is that no policy routes between exact and ANN by estimated E and recall target, as SIEVE and Qdrant do.
2. **Proposed change (PROPOSED, low priority).** (a) Add an "exact-unless" policy: run exact whenever estimated E·D ≤ a budget calibrated like Qdrant's `full_scan_threshold`, otherwise ANN with exact rerank, and report the recall risk. (b) Publish Qenlo's exact filtered path as a filtered-ANN ground-truth generator (it already has FP64 certification), which is a credible niche.
3. **Novel?** No.
4. **Benefits / regressions.** Benefits only for large N with broad filters. Regressions: recall loss, and index build and maintenance cost under mutation.
5. **Smallest falsifying experiment (PROPOSED).** At N = 1M, sweep E from 0.1% to 100%. Find the E at which HNSW (USearch, ef tuned to recall@10 ≥ 0.99) beats the exact CPU scan (multithreaded) end to end. **If** that E exceeds 30% of N, ANN routing is not worth building for Qenlo's target regime.
6. **Cost/risk.** Medium. Publishability: none, unless as part of a ground-truth-generator contribution.

### Gaps
- No published numbers for exact filtered brute force against filtered HNSW at N = 100k, D = 384 specifically.

---

## Q-F. Workload-aware GPU state and co-location (batching, queues, persistent kernels, command reuse, energy)

### Takeaway
The production-grade tools for GPU state (dynamic batching, persistent kernels, graph/command reuse) exist in CUDA ecosystems but are limited or absent in WebGPU/wgpu. WebGPU command buffers are single-submit, and there are render bundles but no compute bundles. WGSL cannot synchronize across workgroups, and persistent kernels risk OS watchdogs. The paper's own data argues against an artificial "heater" in production: the end-to-end prediction P1 failed on every GPU, the heater slowed heavy calls 6–9% on consumer GPUs and 25–27% on H100, slowed host materialization 40–48% on the H100 pod, and its energy cost was never measured. The defensible research version reverses the heater. Use load that is already there (co-located model inference) and route to the GPU only when it is already warm, or micro-batch so GPU work arrives in denser bursts.

### Cited Findings
- cuVS dynamic batching groups small concurrent requests into larger batches with multiple queues and streams to overlap filling and launches. It is recommended "for high-concurrency serving workloads" and to be avoided "when a single thread submits work serially". It is currently a C++ API, with a `conservative_dispatch` latency/occupancy knob. — [cuVS dynamic batching](https://docs.nvidia.com/cuvs/user-guide/api-guides/other-ap-is/dynamic-batching)
- The CAGRA persistent search kernel keeps "a persistent search kernel resident on the GPU and feed[s] it incoming work". Parameters: `persistent_lifetime` (default 2.0 s idle before stopping) and `persistent_device_usage` (fraction of grid, to leave SMs for other work). — [cuVS CAGRA guide](https://docs.nvidia.com/cuvs/user-guide/api-guides/indexing-guide/cagra.md); [cuVS C API](https://docs.nvidia.com/cuvs/api-reference/c-api-neighbors-cagra.md)
- NVIDIA reports that CAGRA persistent search improves high-volume online throughput "by 8x or more while maintaining low one-at-a-time latencies". — [NVIDIA blog 2025-07-24](https://developer.nvidia.com/blog/optimizing-vector-search-for-indexing-and-real-time-retrieval-with-nvidia-cuvs)
- WebGPU: "A GPUCommandBuffer can only be submitted once, at which point it becomes invalidated." Reusable command buffers were rejected partly because "marking command buffers reusable could make them more expensive (Vulkan)". Compute bundles are only a discussed future feature. — [W3C WebGPU](https://www.w3.org/TR/webgpu); [gpuweb #4138](https://github.com/gpuweb/gpuweb/issues/4138); [gpuweb #1971](https://github.com/gpuweb/gpuweb/issues/1971)
- WGSL barriers cannot synchronize across workgroups (see Q-C). — [gpuweb discussion #3935](https://github.com/gpuweb/gpuweb/discussions/3935)
- Paper heater results: light-path end-to-end ratio with the heater was 0.94–0.96 on the 4090 pods at E = 3k–10k (1.07 at 30k) and 1.10–1.24 on H100. Heavy calls were 6–9% slower on consumer GPUs and 25–27% slower on H100. P1 (≥10% e2e speed-up at E ≤ 3k) failed everywhere. Host materialization was 40–48% slower on H100. — QENLO-RESEARCH-PAPER.pdf §6, Tables 6–7
- The paper's telemetry runner already logs `power.draw` every 50 ms. — [REPO/research/scripts/run_dvfs_cell.py](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/research/scripts/run_dvfs_cell.py)
- nvidia-smi power and clock readings are sampled "part-time". — [Yang, Adamek, Armour, arXiv 2312.02741](https://arxiv.org/abs/2312.02741) (cited by the paper)
- Validated NVML-based measurement can be stable within a kernel run when the kernel is looped long enough. AccelWattch reports 0.0018–1.9% variance across measurements for long-running validation kernels. — [AccelWattch, MICRO'21](https://dl.acm.org/doi/fullHtml/10.1145/3466752.3480063)
- Latency-critical services suffer at low load from power management, which motivated latency-aware controllers (Meisner ISCA'11, Kanev IISWC'14, Lo ISCA'14 "Pegasus", Kasture MICRO'15 "Rubik", Hsu HPCA'15 "Adrenaline", Chou HPCA'19 "µDPM"). — cited in the paper's §9, e.g. [Rubik](http://dx.doi.org/10.1145/2830772.2830797), [Adrenaline](http://dx.doi.org/10.1109/HPCA.2015.7056039)

### Inferences
1. **Limitation.** Each query creates and submits new command buffers (WebGPU semantics) and blocks on readback. There is no batching across concurrent callers, the GPU idles between queries, and Qenlo has no knowledge of co-located GPU load.
2. **Proposed changes (PROPOSED), ranked by evidence.**
   - (a) **Micro-batching across concurrent callers** (a cuVS-style dynamic batcher in Rust, with a max-wait budget such as 100–300 µs). Qenlo already batches within one call (`batch_size`), and the router already sends B ≥ 8 to GPU.
   - (b) **Pre-recorded bind groups and pipelines** (already likely; verify), with `queue.write_buffer` to parameter buffers instead of re-creating resources. Command buffers must still be re-encoded each time, because WebGPU has no reuse.
   - (c) **Warmth-aware routing**: an inverted heater. Route to GPU when recent GPU activity (Qenlo's own or observed co-located load) suggests a high clock; otherwise CPU. This is a feature of the flagship router, not a separate system.
   - (d) **Energy-latency objective**: report J/query (GPU power integral plus CPU RAPL) per route. Let applications choose `min latency` or `min energy s.t. p99 ≤ SLO`.
   - **Do not ship** a heater or busy-spin keep-alive. The paper's own evidence is negative end to end, and the energy cost is unmeasured.
   - **Persistent WGSL kernel**: high risk (H2).
3. **Novel?** Micro-batching and persistent kernels are not novel (cuVS). Warmth-aware placement plus energy accounting for exact vector search on WGPU might be new as an empirical study.
4. **Mechanism.** Batching amortizes the fixed ~0.1–0.17 ms GPU overhead (Table 8, E = 100) and raises duty cycle, which the paper links to higher clocks. Warmth-aware routing avoids sending a query to a GPU in a deep power state.
5. **Benefits / regressions.** Batching helps concurrent servers and hurts single-threaded serial callers (the dominant embedded use case), because max-wait adds latency. Warmth-aware routing helps co-located deployments. Energy accounting is neutral.
6. **Smallest falsifying experiment (PROPOSED).** Laptop with a real co-located GPU load: a small embedding model (e.g. MiniLM on GPU) generating at a fixed QPS. Measure the light path's device and end-to-end latency with the co-load off/on and clocks unlocked/locked, recording power. **Refuted (for the co-location hypothesis)** if light-path e2e latency under real co-load is never lower than with an idle GPU in ≥4 of 5 blocks. If refuted, the paper's "isolated benchmarks may misstate co-located latency" stays speculation.
7. **Baselines.** Idle GPU, heater (as the paper's crude stand-in), real model co-load, and locked clocks.
8. **Cost/risk.** Batching: medium. Energy accounting: low. Persistent kernels: high.
9. **Publishable?** A co-location study with real models and energy is a credible workshop paper (HotPower, EuroMLSys, DaMoN), and it directly tests the paper's untested claim.
10. **Evidence required.** Real co-load (not a heater); energy per query with uncertainty; ≥2 GPU families.

### Gaps
- No source on how wgpu 30 handles pipeline or bind-group caching internally. Verify in code.
- RAPL availability and accuracy on the Windows laptop are unknown. I found no source for this session.

---

## Q-G. Persistence and reliability

### Takeaway
Qenlo's protocol already follows the correct patterns (write-to-temp + fsync + rename + directory fsync, CRC32, a WAL with generations). The gap is verification, not design. The literature offers cheap, high-value testing methods that fit a small embedded store: ALICE-style syscall-trace crash-state exploration, CrashMonkey/ACE bounded black-box workloads, fsync-failure injection (Rebello ATC'20), and deterministic simulation. Mostly engineering; publishable only if combined with something new.

### Cited Findings
- ALICE converts application update protocols into crash states and found "a total of 60 vulnerabilities" across eleven widely used systems. "7 of the 11 tested applications" had trouble recovering when syscall ordering is not maintained, and "10 of the 11 applications expect atomicity of file-system updates". — [Pillai et al., OSDI 2014](https://www.usenix.org/system/files/conference/osdi14/osdi14-paper-pillai.pdf)
- B³ (CrashMonkey, Ace): bounded black-box crash testing found 24 of 26 crash-consistency bugs reported in the prior five years. "Most reported bugs can be reproduced using small workloads of three or fewer file-system operations". — [Mohan et al., OSDI 2018](https://www.usenix.org/conference/osdi18/presentation/mohan)
- fsync failures: across ext4, XFS and Btrfs, "pages are always marked clean" after failure, and none of PostgreSQL, LMDB, LevelDB, SQLite or Redis handle fsync failures sufficiently. Outcomes include data loss and corruption. — [Rebello et al., USENIX ATC 2020](https://www.usenix.org/conference/atc20/presentation/rebello)
- Deterministic simulation: FoundationDB "built a deterministic database simulation framework" before the database. TigerBeetle's VOPR stubs out "the clock, network, and disk operations" and reproduces bugs from a seed plus commit. — [FoundationDB paper](https://www.foundationdb.org/files/fdb-paper.pdf); [TigerBeetle VOPR doc](https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/internals/vopr.md)
- Qenlo: `sync_all` on WAL and snapshot files, `MANIFEST.pending` → rename → `sync_directory`, directory fsync for newly created parents ("A child fsync is not enough"), CRC32 checksums, and an error variant stating "snapshot was published but durability confirmation failed; close and reopen to resolve commit". — [REPO/crates/qenlo/src/storage.rs](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo/src/storage.rs)

### Inferences
1. **Limitation.** There is one recovery test file (69 lines). I found no evidence of systematic crash-state enumeration or fsync-error injection, though my check was not thorough.
2. **Proposed change (PROPOSED).**
   - (a) A storage I/O trait seam (a `Vfs` with a real and a simulated implementation) so every write/fsync/rename goes through an injectable layer.
   - (b) An ALICE-like recorder that logs the op sequence for each mutation API, then enumerates prefixes and legal reorderings under a conservative persistence model (e.g. ext4 data=ordered rules) and checks recovery invariants: last acknowledged generation readable, no phantom rows, checksums valid.
   - (c) fsync-failure injection. After a failed fsync, the store must poison the handle, not retry, following the PostgreSQL post-"fsyncgate" approach. The existing error message suggests Qenlo already intends this.
   - (d) A seed-deterministic simulation of add/delete/close/open sequences against an in-memory oracle (`qenlo-testkit` may host it).
   - (e) Format-evolution tests (open v1 WAL with a v2 reader).
3. **Novel?** No. The same protocol applied to a vector store's derived GPU and index state might be worth a short note: the GPU-resident copy and IVF/HNSW are derived state that must be invalidated by generation after recovery.
4. **Benefits / regressions.** All users benefit; no runtime regression, since this is test-only, apart from the trait-object indirection.
5. **Smallest falsifying experiment (PROPOSED).** Run the enumerator on add_batch → close and delete → close. **The protocol is falsified** (a bug is found) if any crash state recovers to a state that is not a prefix of acknowledged operations.
6. **Cost/risk.** Low-medium (≈1–2 weeks). Publishability: low on its own.
7. **Evidence required.** The number of crash states explored, invariants checked, and bugs found and fixed. If the clustered layout (Q-D) is adopted, include compaction.

### Gaps
- Windows fsync and rename semantics (the paper's laptop is Windows 11) differ from Linux. ALICE and CrashMonkey are Linux-centric. I found no Windows equivalent in this session.

---

## Q-H. Embedded and multi-language architecture

### Takeaway
Qenlo already has a C ABI, typed result buffers and zero-copy intake of Python buffers. The substantive upgrades are standard interchange formats: the Arrow C Data/Stream Interface for record batches in and results out, and DLPack for device-resident query vectors (e.g. a query embedding already on the GPU from PyTorch). These reduce copies and make Qenlo composable. They are engineering, not research, with one exception: zero-copy device query hand-off could remove the per-query upload of the query vector from a co-located model, which ties into the co-location direction.

### Cited Findings
- The Arrow C data interface is "Zero-copy by design", ABI-stable and "Easy to reimplement from scratch". The C stream interface handles streaming; the C Device data interface (experimental) extends it to non-CPU memory. — [Arrow C data interface](https://arrow.apache.org/docs/18.0/format/CDataInterface.html); [C stream interface](https://arrow.apache.org/docs/format/CStreamInterface.html); [C Device data interface](https://arrow.apache.org/docs/format/CDeviceDataInterface.html)
- DLPack is "a stable in-memory data structure" for tensor exchange with cross-device support. It was selected as the Python array API interchange standard, and DLPack 1.3 adds a C Exchange API. — [Arrow DLPack docs](https://arrow.apache.org/docs/python/dlpack.html); [DLPack Python spec](https://dmlc.github.io/dlpack/latest/python_spec.html)
- The Python buffer protocol gives zero-copy access to a producer's memory; PyO3 exposes it via `PyBuffer`; `pyo3-arrow` implements zero-copy FFI between Python Arrow objects and Rust `arrow` via the Arrow PyCapsule Interface. — [CPython buffer protocol](https://docs.python.org/3/c-api/buffer.html); [PyO3 PyBuffer](https://pyo3.rs/main/doc/pyo3/buffer/struct.pybuffer); [pyo3-arrow](https://docs.rs/pyo3-arrow/latest/pyo3_arrow/index.html)
- Qenlo's Python `add_buffer` copies when the view is read-only (`from_buffer_copy`) and shares otherwise. — [REPO/sdk/python/src/qenlo/_collection.py](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/sdk/python/src/qenlo/_collection.py)

### Inferences
1. **Proposed (PROPOSED).** (a) `search_batch_arrow(ArrowArray queries) → ArrowArrayStream results`. (b) `add_arrow(RecordBatch)` with columns id/user/timestamp/FixedSizeList<f32, D>. (c) Accept DLPack query tensors; CPU-device first. A device-pointer import into wgpu is not generally possible: wgpu has no standard external-memory import for CUDA pointers. That is my understanding and unverified, so treat device zero-copy as a gap.
2. **Smallest falsifying experiment (PROPOSED).** Python batch of 1,024 queries: the ctypes list path against Arrow/NumPy zero-copy. **Refuted if** the FFI share of end-to-end time is < 5% at batch 1 and < 10% at batch 1,024 (then there is no point).
3. **Cost/risk.** Medium (ABI surface across 5 SDKs). Publishability: none.

### Gaps
- Whether wgpu 30 supports importing external (CUDA/Vulkan external memory) buffers. Not verified.

---

## Q-I. Research tooling and reproducibility

### Takeaway
Qenlo's research practice is already above average: fresh-process blocks, pre-registered protocols for E-LOCK/E-SPLIT, released raw samples, continuous telemetry and a claim-artifact ledger. The highest-value additions are (i) a machine-readable evidence manifest linking every paper number to data files, scripts, commit and hardware fingerprint, so claims can be checked automatically; (ii) a per-query decision trace for routing research; and (iii) a power-state fingerprint in each run header, for example the locked-boost reference time of a fixed probe kernel. These directly support the flagship and the paper revision.

### Cited Findings
- Hoefler & Belli surveyed 120 papers. "Most papers report details about the hardware but fail to describe the software environment", "Only 15 mention some measure of variance" and "Only two (!) report confidence intervals". They also argue that CIs let you compute the number of required measurements. — [SC15 paper](https://dl.acm.org/doi/10.1145/2807591.2807644); [slides](https://htor.inf.ethz.ch/publications/img/hoefler-scientific-benchmarking_wide_HLRS.pdf)
- ReproZip traces system calls to pack the binaries, files and dependencies an experiment needs, and unpacks it on other platforms (Docker, Vagrant, chroot). Packing works only on Linux. — [ReproZip docs](https://docs.reprozip.org/); [SIGMOD 2016 demo](https://dl.acm.org/doi/abs/10.1145/2882903.2899401)
- Principles for automated and reproducible benchmarking. — [UCL/SC-W 2023 PDF](https://discovery.ucl.ac.uk/id/eprint/10215101/1/3624062.3624133.pdf)
- Variability must be sampled across executions — Kalibera & Jones ISMM'13; Maricq et al. OSDI'18 — [Maricq OSDI'18](https://www.usenix.org/conference/osdi18/presentation/maricq) (both cited in the paper)
- The repo already contains `research/claim-artifact-ledger.md`, `research/sota-claim-matrix.md`, `methodology-audit.md` and per-experiment `protocol.md` + `protocol.sha256`. — [REPO/research](https://github.com/a3ro-dev/qenlo/tree/142e96cf43684a13e95513a161bda4515ad6f1ca/research); [REPO/research/experiments/dvfs-lock](https://github.com/a3ro-dev/qenlo/tree/142e96cf43684a13e95513a161bda4515ad6f1ca/research/experiments/dvfs-lock)

### Inferences
1. **Proposed (PROPOSED).**
   - (a) `evidence.json` per paper table cell: `{claim_id, value, unit, CI, n_blocks, data_paths+sha256, script+commit, binary hash, hardware fingerprint}`, plus a CI job that regenerates each table and diffs it against the manuscript numbers. That makes claim verification automatic.
   - (b) A hardware/power fingerprint: CPU model, microcode, governor, GPU model, VBIOS, driver, PCIe link, OS power plan, clocks-locked flag, and a 1-second probe measuring the fixed-workload kernel time relative to a stored locked-boost reference. That gives a "power-state index" per process.
   - (c) A per-query trace (sampled): features, route chosen, predicted cost and interval, actual device and e2e time, a clock sample. Required by Q-A/B.
   - (d) Report templates for block-bootstrap CIs and A/A noise floors.
   - (e) Make the binary build reproducible, including the failed `gpu-wgpu` feature build in E-SPLIT: assert enabled features at startup and write them into the header.
2. **Novel?** No, but it would let the paper make a cheap and defensible "artifact" claim.
3. **Cost.** Low. Validation: does an independent rerun reproduce the manifest checks?

### Gaps
- None of significance for this direction.

---

## Q-J. Unanticipated directions

### Takeaway
Three findings matter most.
- **J1, the CPU baseline.** The paper's CPU/GPU crossover between E = 3k and 10k coincides with Qenlo's CPU kernel switch at E = 4,096 (certified FP32 → FP64 accumulation), and the CPU scan is single-threaded. Per-row CPU cost roughly triples across that boundary, by my own arithmetic from Table 8. A stronger CPU path (multithreaded, certified FP32 extended or tiled) could move the crossover past E = 10k at N = 100k. This is the single most important threat to the paper's application-level framing, and the most important near-term engineering fix.
- **J2, segment trees.** A "segment tree of precomputed top-k lists" cannot give exact range-filtered top-k, because vector scores depend on the query. Published range top-k structures assume static scores. The vector analog (WinFilter) is approximate, and exact geometric pruning in 384-D is expected to be weak (Weber et al. 1998). This is a high-risk hypothesis, not a plan.
- **J3, certified bounds.** Qenlo's certified-bound machinery can be generalized: GPU FP32 with a provable boundary set rescored in FP64 (exact by construction), and int8/SQ8 scoring with deterministic bounds (VA-file-style filter-and-refine). This keeps the "identical results" contract with fewer bytes per row.

### Cited Findings
- Paper Table 8 (median-block e2e P50, ms), CPU column. Laptop: 0.010 / 0.083 / 0.308 / 3.332 / 9.187 at E = 100 / 1k / 3k / 10k / 30k. 4090 pods: 0.008 / 0.048 / 0.133 / 1.495 / 4.907. Light GPU on the laptop: 0.548 at 3k and 1.682 at 10k; on the pods: 0.178 at 3k and 0.430 at 10k. — QENLO-RESEARCH-PAPER.pdf, Table 8
- Paper §2: the CPU scan "For E ≤ 4,096 … scores in FP32 and rescores boundary candidates in FP64; above that it accumulates in FP64 throughout. The kernel switch … applies equally to every condition we compare." — QENLO-RESEARCH-PAPER.pdf §2
- `CERTIFIED_FP32_MAX_ROWS = 4_096`; the in-repo study measured the certified path −28% at E = 4,000 and +10% at E = 100,000. — [REPO/crates/qenlo-core/src/lib.rs](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/crates/qenlo-core/src/lib.rs); [REPO/research/phase2-cpu-exact-progress.md](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/research/phase2-cpu-exact-progress.md)
- "When searching a single vector, Faiss does not spawn multiple threads." Faiss relies on OpenMP. — [The Faiss Library, arXiv 2401.08281](https://arxiv.org/html/2401.08281v3)
- In-repo: single-thread FAISS `IndexFlatIP` is 26.9% lower-latency than Qenlo at unfiltered 100k. — [REPO/research/phase2-cpu-exact-progress.md](https://github.com/a3ro-dev/qenlo/blob/142e96cf43684a13e95513a161bda4515ad6f1ca/research/phase2-cpu-exact-progress.md)
- The laptop CPU (Intel Core i5-13420H) has 8 total cores. — [Intel ARK](https://www.intel.com/content/www/us/en/products/sku/232173/intel-core-i513420h-processor-12m-cache-up-to-4-60-ghz/specifications.html)
- Vector search on cloud CPUs is often memory-bound, and performance "depends not solely on their SIMD capabilities" but also on cache bandwidth and latency. AVX-512 processes 16 FP32 values per instruction. — [Bang for the Buck, DaMoN 2025](https://dl.acm.org/doi/10.1145/3736227.3736233)
- Range top-k over arrays (static values) has known encodings, e.g. Brodal et al. — [Dagstuhl OASIcs survey "Encoding Data Structures for Range Queries on Arrays", 2025](https://drops.dagstuhl.de/storage/01oasics/oasics-vol132-grossis-festschrift/html/OASIcs.Grossi.12/OASIcs.Grossi.12.html)
- WinFilter / β-WST: "a modular tree-based framework for transforming an index that solves the traditional c-approximate nearest neighbor problem into a data structure that solves window search", using segment-tree-like augmented nodes. — [Engels et al., ICML 2024](https://proceedings.mlr.press/v235/engels24a.html); [arXiv 2402.00943](https://arxiv.org/html/2402.00943v2)
- Weber, Schek, Blott (VLDB 1998) analyse the dimensionality curse for tree methods and propose the VA-file, which "scan[s] the entire approximation file" and excludes "the vast majority of vectors … based only on these approximations", then refines. HD-Index notes that in high dimensions "linear scan becomes the only feasible solution", which motivated VA-file-style compression. — [Weber et al., VLDB 1998](https://www.vldb.org/conf/1998/p194.pdf); [HD-Index, PVLDB 11](http://www.vldb.org/pvldb/vol11/p906-arora.pdf)
- RaBitQ gives an unbiased estimator with an error bound "sharp … asymptotic optimality" O(1/√D) that holds with a failure probability "viewed as a constant", i.e. probabilistic. — [Gao & Long, SIGMOD 2024, arXiv 2405.12497](https://arxiv.org/html/2405.12497v1)
- Production int8/binary quantization rescoring relies on oversampling heuristics, not guarantees. — [Qdrant binary quantization](https://qdrant.tech/articles/binary-quantization); [Azure AI Search quantization](https://learn.microsoft.com/en-us/azure/search/vector-search-how-to-quantization)
- WGSL `dot4I8Packed(u32, u32) -> i32` (DP4a) is available under the `packed_4x8_integer_dot_product` language feature. — [W3C WGSL](https://www.w3.org/TR/WGSL); [MDN WGSLLanguageFeatures](https://developer.mozilla.org/en-US/docs/Web/API/WGSLLanguageFeatures)

### Inferences
**J1. Is the single-threaded CPU baseline fair? (derived arithmetic from Table 8)**
- Per eligible row, CPU e2e/E on the laptop: 100 ns (E = 100), 83 ns (1k), 103 ns (3k), **333 ns (10k)**, 306 ns (30k). On the 4090 pods: 80, 48, 44, **150**, 164 ns.
- The ~3.2–3.4× jump between 3k and 10k lines up with the 4,096 switch to FP64 accumulation, and the crossover sits in the same interval.
- Counterfactual (derived, not measured). If the certified FP32 path's per-row cost at 3k held at 10k, the CPU would take ≈1.03 ms on the laptop (against GPU light 1.682 ms, so CPU wins) and ≈0.44 ms on the pods (against 0.430 ms, parity).
- The in-repo study shows certified FP32 losing 10% at E = 100k, so the per-row cost does not stay flat. Linear extrapolation is optimistic, and the boundary-candidate rescoring cost must be measured.
- Multithreading over row partitions with per-thread top-k and a k-way merge would divide the scan by up to the number of useful cores. The cost is thread wake-up latency (µs) and memory-bandwidth saturation. The i5-13420H has 8 cores of mixed type; my unverified recollection is 4 P-cores plus 4 E-cores and no AVX-512.
- Verdict: the single-threaded baseline matches FAISS's batch-1 convention (FAISS does not thread single queries), so it is "conventional". It is **not** a latency-optimal CPU baseline, and at least part of the crossover is created by Qenlo's own CPU kernel switch. The paper's claims about the light path being faster than "our CPU scan" at E = 10k should be scoped accordingly.
- PROPOSED fix (near-term N1):
  - (a) Run the certified path in blocked form at any E: process rows in tiles, maintain the running cutoff, and rescore only rows within 2·bound. Measure where it loses.
  - (b) Parallel scan with T threads (rayon or a persistent pool, to avoid spawn cost) when E·D exceeds a measured threshold.
  - (c) AVX-512 kernels where available.
  - (d) An "AVX2 single-thread" mode kept as a named control for comparability with the paper.

**J2. Segment tree of per-range top-k lists.**
- Exactness argument: for a fixed query q, the top-k over a union of nodes is the top-k of the per-node top-k's. That is true, but per-node top-k depends on q, so it cannot be precomputed. Precomputation works only for query-independent scores, the setting of the range top-k literature.
- What *can* be precomputed per node is a geometric summary: centroid c, radius r, or a cone. For unit vectors, `q·x ≤ q·c + r·‖q‖`, an exact upper bound. Best-first traversal over the O(log n) canonical nodes covering the range can then prune nodes whose bound falls below the running k-th best. Exact.
- Prior expectation (Weber 1998; HD-Index): in 384-D the bounds are loose, so pruning will be small. MiniLM embeddings are anisotropic and clustered, which may help. Unknown.
- Smallest falsifying experiment (PROPOSED): compute, offline in Python with no engine changes, the fraction of rows the bound excludes for 1,000 AG News queries at k = 10, with nodes formed by timestamp ranges over randomly ordered rows (the paper's setting) and over semantically clustered rows. **Abandon if** < 30% of E is pruned on average at E = 10k. Cost: one day.

**J3. Certified quantized and certified GPU scoring.**
- GPU certified exactness (N3): return the top-k plus every candidate within 2·bound of the k-th (same bound formula as the CPU, adjusted for the GPU's summation order: a shared-memory tree of 32 lanes, then a 32-wide tree), and rescore in FP64 on the host. Today the GPU path's "identical results" is an empirical recall@10 ≥ 0.99 gate with observed 1.0, not a proof.
- Quantized (R4): store int8 (SQ8 already exists in the IVF code) with per-row scale and offset. A deterministic bound on `|q·x − q·x̂|` follows from the quantization step (≤ Σ|q_i|·δ_i/2). Score with `dot4I8Packed` on GPU or VNNI/AVX2 on CPU. Keep candidates whose upper bound ≥ the k-th lower bound; rescore exactly in FP32/FP64. That is VA-file-style, exact by construction, and moves 4× fewer bytes per row.
- Novelty is low (VA-file, 1998). The value is bandwidth: the light path at E = 10k reads 15.4 MB of FP32 vectors (10k × 384 × 4 B).
- Falsifier: if the candidate set after filtering exceeds ~10% of E at k = 10, the second pass dominates and the gain disappears.

**J4. Register-resident heaps (k = 10).** Covered in Q-C. CPU equivalent: replace `BinaryHeap` with a fixed array plus branchless insertion for k ≤ 16. Low effort; gain unknown and possibly small, since the scan is memory-bound.

**J5. GPU-resident persistent query loop.** See H2 in the portfolio. Constrained by WGSL's lack of cross-workgroup sync and forward-progress guarantees, and by OS GPU watchdogs. High risk.

### Gaps
- **No published batch-1 latency numbers for FAISS-GPU or cuVS brute-force kNN at N ≈ 100k, D = 384.** Published GPU vector-search results emphasize throughput at large batches (e.g. CAGRA at batch 10 on DEEP-100M — [NVIDIA blog](https://developer.nvidia.com/blog/accelerating-vector-search-using-gpu-powered-indexes-with-rapids-raft)). Mageirakos et al. report GPU ENN 2–69× faster than CPU inside SQL+VS queries ([arXiv 2605.15957](https://arxiv.org/html/2605.15957v1)), but that is not batch-1 kernel latency at this size. A direct measurement is needed (proposed in the flagship as an external baseline).
- **Published state of the art for exact range-filtered top-k.** I found no work achieving exact, sublinear-in-E range-filtered kNN in high dimensions. The standard exact method is pre-filter plus brute force, which iRangeGraph, Qdrant, Weaviate and cuVS all recommend for small ranges. All range-filtered *indexes* found (SeRF, iRangeGraph, UNIFY, WinFilter, WoW, RangePQ+, dynamic RFANNS) are approximate.
- AVX-512 absence and the P/E-core split for the i5-13420H come from my recollection, not a verified source.
- The 4050/4090/H100 SM counts are recollection.

---

## Q-Portfolio. Scored portfolio (Phase 7)

### Takeaway
Recommended order: first fix the measurement baselines (strong CPU, better top-k, certified GPU exactness, evidence manifest). Then run the research questions those fixes make answerable. Does the power-state effect survive a well-designed kernel (R2)? Does a state-aware, uncertainty-aware router cut held-out regret across hosts (flagship R1)? Avoid new filtered-ANN indexes, heaters as features, and learned deep optimizers for a two-arm decision.

### Cited Findings
- The scores below are judgments based on the evidence cited in Q-A through Q-J. They are not measured facts. Sources for each mechanism appear in the relevant section above.

### Inferences

**Rubric (1–5, higher is better for every column).** Impact = expected practical impact. Novelty = novelty supported by the preliminary literature review. Evidence = strength of motivating evidence. Feasibility = feasibility for a small team. Cost = implementation cost (5 = cheapest). Validation = ease of rigorous validation. Publish = potential for a publishable result. NegRisk = risk of a negative or inconclusive result (5 = low risk). Total = unweighted sum (max 40), for orientation only.

**Near-term engineering improvements (high confidence)**

| ID | Item | Impact | Novelty | Evidence | Feasibility | Cost | Validation | Publish | NegRisk | Total |
|---|---|---|---|---|---|---|---|---|---|---|
| N1 | Strong CPU baseline: tiled certified FP32 at all E, intra-query multithreading, optional AVX-512; keep the paper's AVX2-1T as a named control | 5 | 1 | 5 | 5 | 4 | 5 | 2 | 5 | 32 |
| N2 | Fused score + per-workgroup register top-k + host/second-pass merge (portable, subgroup fast path) | 5 | 2 | 5 | 4 | 3 | 4 | 3 | 4 | 30 |
| N3 | GPU certified exactness (FP32 bound → boundary set → FP64 host rescore) | 3 | 2 | 4 | 5 | 5 | 5 | 2 | 5 | 31 |
| N4 | Compact eligibility encodings: bitmap (N/8), runs (start, len) chosen by cost; device-resident liveness | 3 | 1 | 4 | 5 | 4 | 5 | 1 | 4 | 27 |
| N5 | Evidence manifest + hardware/power fingerprint + per-query trace + CI claim check | 4 | 2 | 4 | 5 | 4 | 5 | 2 | 5 | 31 |
| N6 | Crash-consistency harness (Vfs seam, ALICE-style enumerator, fsync-failure injection, seeded simulation) | 4 | 1 | 3 | 4 | 3 | 4 | 1 | 4 | 24 |

Rationale. N1 scores top on evidence because the repo's own memo flags the confound, and the Table 8 per-row arithmetic shows the 4,096 step. N2's evidence is Table 5 (selection is 3–4× scoring) plus the code itself. N3 is cheap and converts an empirical gate into a guarantee. N4's gains are limited in the paper's own workload (random slot subsets), hence moderate impact. N6 is valuable for users but not for the paper.

**Research-grade opportunities**

| ID | Item | Impact | Novelty | Evidence | Feasibility | Cost | Validation | Publish | NegRisk | Total |
|---|---|---|---|---|---|---|---|---|---|---|
| R1 | **State-aware (performative) online CPU/GPU router with conformal risk control** (FLAGSHIP) | 4 | 4 | 4 | 3 | 3 | 3 | 4 | 3 | 28 |
| R2 | Does a well-parallelized top-k change the power state? Occupancy × DVFS interaction study (light-path v2 vs v1, locked/unlocked) | 3 | 4 | 4 | 5 | 4 | 4 | 4 | 3 | 31 |
| R3 | Clustered timestamp layout + run eligibility: exact range-filtered top-k with zero materialization, with write-amplification/compaction cost model | 3 | 2 | 3 | 3 | 2 | 4 | 3 | 4 | 24 |
| R4 | Certified quantized exact search (SQ8/int8 + deterministic bounds + exact rerank; `dot4I8Packed` on GPU) | 4 | 2 | 3 | 4 | 3 | 4 | 3 | 3 | 26 |
| R5 | Cross-host transfer of crossover models (leave-one-host-out; hierarchical prior + per-host probe) | 3 | 3 | 4 | 4 | 4 | 4 | 3 | 3 | 28 |
| R6 | Real co-location study: retrieval kernel under co-running model inference, latency + energy | 3 | 3 | 3 | 4 | 4 | 3 | 3 | 3 | 26 |

Rationale. R2 is cheap (it reuses E-LOCK infrastructure) and either strengthens the paper or reveals that its effect is a property of single-workgroup kernels. Both outcomes are publishable as an extension. R1 has the highest novelty but depends on N1, N2 and N5. R5 can partly run on archived data.

**Ambitious, high-upside hypotheses (may fail)**

| ID | Item | Impact | Novelty | Evidence | Feasibility | Cost | Validation | Publish | NegRisk | Total |
|---|---|---|---|---|---|---|---|---|---|---|
| H1 | Exact range-filtered top-k by best-first traversal of a segment tree over timestamp blocks with geometric (ball/cone) bounds, sublinear in E | 4 | 3 | 1 | 3 | 3 | 4 | 4 | 1 | 23 |
| H2 | GPU-resident persistent query loop in WGSL (spin on a queue-written buffer; no per-query dispatch) | 3 | 3 | 2 | 2 | 2 | 3 | 3 | 1 | 19 |
| H3 | Hedged dual execution near the crossover (CPU and GPU concurrently, first wins, cancel/discard the other), with energy accounting | 3 | 2 | 3 | 4 | 4 | 4 | 3 | 2 | 25 |
| H4 | Endogenous-warmth exploitation: deliberately order or coalesce queries so GPU-bound work arrives in bursts that keep the clock high without artificial load | 3 | 4 | 2 | 3 | 3 | 3 | 4 | 2 | 24 |

Rationale. H1 has prior evidence against it (curse of dimensionality) but a one-day offline falsifier. H2 faces platform constraints (no cross-workgroup sync, watchdogs). H3 roughly doubles CPU and GPU work for queries in the band. H4 is the scheduling twin of R1; it fails if clock ramp-up is slower than realistic burst lengths.

**Dependencies**
- N5 → everything (traces and fingerprints are needed to evaluate anything).
- N1 → R1, R5, H3 (a router evaluated against a weak CPU arm is invalid).
- N3 → N2, R4 (new kernels must keep the exactness contract; the certification machinery is shared).
- N2 → R2 (the treatment arm), R1 (the GPU arm the router chooses).
- N4 → R3; R3 → N6 (layout and compaction changes need crash testing).
- R2 → R1. If v2 kernels remove the power-state bimodality, R1's state feature matters less and R1 reduces to R5. The flagship design branches on this (see Q-Flagship).
- R6 ↔ H4 (both need real co-load infrastructure).

**Combinations**
- "Light path v2" = N2 + N3 + N4 (runs/bitmap + fused selection + certified rescore). One kernel family and one validation campaign.
- "Strong baselines pack" = N1 + external FAISS-CPU (threaded and 1T) + cuVS/FAISS-GPU brute force at batch 1. Needed by the paper revision and the flagship alike.
- R1 + R5 + H3 in one router study (hedging as one router arm).

**Projects to avoid**
- A new filtered or range-filtered ANN index. Duplicates SeRF, iRangeGraph, UNIFY, WinFilter, WoW, ACORN, SIEVE, and Qenlo's regime favours exact search (Q-E).
- A production "heater" or keep-alive busy loop. The paper's own P1 failed on every GPU, it caused host slowdowns, it slowed the heavy path, and its energy cost is unmeasured (Q-F).
- Neo/Balsa-style deep RL optimizers for a 2–3-arm placement decision. A massive cost-to-value mismatch; Bao-style bandits over a calibrated analytic model are enough (Q-A).
- Optimizing the heavy path in place. It is the paper's negative control. If a tuned heavy path is wanted, add it as a new named arm.
- "Segment tree of precomputed top-k lists". Mathematically inapplicable to query-dependent scores (Q-J2). Only the bound-based variant (H1) is coherent.
- More SDK breadth (new languages). The brief asks for substance; Arrow/DLPack interop (Q-H) is the substantive version.

### Gaps
- Scores are not calibrated against any external rubric and should be read as an ordinal ranking.

---

## Q-Flagship. Flagship direction and executable experiment specification (Phase 8)

### Takeaway
Flagship: **"State-aware, uncertainty-aware CPU/GPU routing for exact filtered top-k on heterogeneous hosts"** (R1). It is staged behind a mandatory Stage 0 that strengthens both arms (N1, N2, N3) and tests whether the power-state effect survives better kernels (R2). It turns the paper's observation (GPU-arm cost depends on a power state the workload itself induces) into a system contribution with a falsifiable claim: lower held-out max and p95 regret across hosts than static or offline-fitted routers, at negligible overhead. All of it is PROPOSED.

### Cited Findings
- Motivating evidence, from QENLO-RESEARCH-PAPER.pdf: held-out router failure (8/16 wrong, max regret 235.7%); same-model cross-host light-kernel spread 4.6×; locked-boost light kernel 2.9–5.0× faster; heavy path stable; P1 failed; CPU/GPU winners in Table 8.
- Method precedents: HyPE/Ocelot feedback-calibrated placement ([PVLDB 2014](https://www.vldb.org/pvldb/vol7/p1609-bress.pdf)); Bao contextual bandit with Thompson sampling ([SIGMOD 2021](https://people.csail.mit.edu/hongzi/content/publications/BAO-Sigmod21.pdf)); conformal prediction and risk control ([arXiv 2107.07511](https://arxiv.org/html/2107.07511v6), [conformal-risk](https://github.com/aangelopoulos/conformal-risk)); performative prediction ([ICML 2020](https://proceedings.mlr.press/v119/perdomo20a.html)); hedged requests ([CACM 2013](https://cacm.acm.org/research/the-tail-at-scale)).
- Methodology precedents: fresh-process replication and variance reporting ([Maricq OSDI'18](https://www.usenix.org/conference/osdi18/presentation/maricq); [Hoefler & Belli SC15](https://dl.acm.org/doi/10.1145/2807591.2807644)); part-time nvidia-smi sampling ([arXiv 2312.02741](https://arxiv.org/abs/2312.02741)).
- WebGPU dispatch overhead of 24–36 µs per dispatch on Vulkan bounds the GPU fixed cost ([arXiv 2604.02344](https://arxiv.org/abs/2604.02344)).

### Inferences

#### Research questions
- **RQ0 (Stage 0, prerequisite).** After strengthening both arms (N1 CPU; N2 + N3 GPU), where is the CPU/GPU crossover at N = 100k and N = 1M, and does the light-path power-state bimodality persist with the v2 kernel? (This also answers R2.)
- **RQ1.** Does an online, state-aware, conformal-guarded router achieve lower held-out regret (max, p95, mean) than (a) static thresholds and (b) an offline per-host-fitted router, on hosts and workloads unseen during design?
- **RQ2.** Does the router's own recent GPU usage measurably change subsequent GPU-arm latency (endogeneity), and does modelling it reduce regret?

#### Hypotheses (falsifiable) and nulls
- **H0a (Stage 0).** With v2 kernels and clocks unlocked, the light-path device-time max/min across fresh processes at E = 10k is ≤ 1.15 on the laptop. *Alternative:* > 1.5 (the effect persists). *Null for the comparison:* the v2/v1 ratio of max/min spreads equals 1.
- **H1.** Online state-aware router (OSR) max regret ≤ 0.5 × static-router max regret, and p95 regret lower, on ≥ 2 of 3 held-out hosts, with OSR p50 decision overhead ≤ 5 µs. *Null:* the median-over-blocks log-ratio of regret (OSR vs static) = 0.
- **H2 (endogeneity).** The GPU-arm latency of a probe query depends on the preceding routing history (k_prev GPU queries within Δ ms) after controlling for E and host: the slope on "recent GPU duty" is non-zero, with a 95% CI excluding 0. *Null:* slope = 0.
- **H3.** Removing state features from OSR increases held-out p95 regret by ≥ 20% (ablation). *Null:* no difference.

#### Independent variables
- Router policy ∈ {forced-CPU, forced-GPU-light-v2, static-4096 (current code), static E·D·B ≥ 10⁶ (paper's earlier rule), offline per-host fit (RouterProfile), OSR, OSR-without-state, OSR+hedging (H3)}.
- E: log-uniform stream over [10, 0.3·N], plus fixed cells {100, 1k, 3k, 10k, 30k} for comparability.
- N ∈ {100k, 1M}; D ∈ {384, 768 or 1024}; k ∈ {10, 64}; batch ∈ {1, 8}.
- Arrival process ∈ {closed-loop back-to-back (the paper's), Poisson with mean gap {1, 10, 100} ms, bursty (MMPP: 10 ms bursts at 1 kHz, then 500 ms idle)}.
- Co-load ∈ {idle GPU, real co-located embedding model at fixed QPS}. Optional third level: the heater, for continuity with the paper.
- Clock state ∈ {unlocked (default), locked-boost where admin rights exist}.
- Predicate ∈ {timestamp range on random-order rows (the paper's), timestamp range on clustered rows, user-equality ∧ range}.

#### Dependent variables
- Per-query end-to-end latency (host call → ordered results), with full distribution: p50/p95/p99/max.
- Device time (GPU timestamp queries) for scoring and selection; host materialization, upload, readback and merge times.
- Regret per query = t_router / t_oracle − 1, where t_oracle = min over forced arms for the same (query, block). See Oracle.
- Router decision overhead (ns, measured with the TSC around the decision function).
- Energy per query: GPU via an NVML power integral; CPU via RAPL on Linux. Report with uncertainty and the part-time sampling caveat.
- CPU utilization and GPU duty cycle; peak memory (RSS and GPU allocation); startup and probe overhead.

#### Controls and confounders
- Fixed: binary hash and features (assert `gpu-wgpu` at startup and record it; cf. the E-SPLIT failed build), dataset bytes, query set, k, thread-pool size, CPU governor (record it; on Linux set `performance` in one arm and `schedutil` in another rather than silently fixing it), OS power plan (record it), and the display/Optimus configuration on the laptop.
- Confounders to measure and model: host CPU speed (the paper replaced a pod with a 14× slower CPU, so add a pre-registered CPU micro-probe with exclusion bounds), driver branch, GPU temperature, background processes (log the process list at block start), co-tenant activity on cloud pods (unobservable, so use blocks and a host random effect), and turbo/thermal state of the CPU.
- Order effects: within a block, randomize policy order. Do not run the oracle arms consecutively with the same policy; interleave them.

#### Hardware and software
- Hosts (≥3 GPU families, ≥2 CPU microarchitectures):
  - (1) The paper's laptop (RTX 4050 Laptop, i5-13420H, Windows 11, Vulkan) as a continuity anchor with clock locking.
  - (2) RTX 4090 cloud pods: ≥3 distinct hosts, each running ≥4 blocks so host and process variance can be separated (fixing the paper's one-block-per-pod confound).
  - (3) H100 SXM pod.
  - (4) One non-NVIDIA GPU (AMD Radeon/Vulkan or Apple M-series/Metal), to test wgpu portability. Telemetry will differ, so record vendor tools.
  - (5) A CPU with AVX-512 (Zen 4 or Sapphire Rapids) paired with any GPU, to test N1's effect on the crossover.
- Software: wgpu 30.0.1 (pinned), Rust toolchain from `rust-toolchain.toml`, driver versions recorded, faiss-cpu (pinned) and faiss-gpu/cuVS (pinned) for external baselines.

#### Workload generation and datasets
- Public data: AG News MiniLM-L12 embeddings 100k (the paper's: DTU dataset, doi 10.11583/DTU.21286923.v1, as cited in the paper) for continuity. A 1M-row D = 768/1024 corpus, for example the 500k Cohere v3 BEIR mix referenced by Weaviate ([PathSeer blog](https://weaviate.io/blog/pathseer-filtered-vector-search)) or a Wikipedia embedding set on Hugging Face; the exact choice is a Gap. Held-out queries must be disjoint from the corpus.
- Synthetic stress: adversarial near-tie distances (duplicate vectors with distinct ids, to test tie-breaking); E = 0 and E < k; extreme timestamps (i64 min/max; the code has tests for these); skewed user distribution (the harness's `skewed`).
- Query streams: generated from a seeded RNG and recorded as a replay file (`qenlo-bench` has `replay.rs`), so every policy sees the identical stream.

#### Competitors and baselines
- Internal: forced arms, static rules, RouterProfile offline fit, OSR ablations.
- External exactness and latency references: FAISS `IndexFlatIP` CPU in both 1-thread and all-threads configurations (note that FAISS does not thread single queries by default), FAISS-GPU flat and cuVS brute force at batch 1 and 8 with a bitmap prefilter for filtered cells. These benchmark the GPU arm's quality, not the router.
- Also include the paper's original v1 light and heavy kernels as named control arms, for continuity.

#### Correctness oracle
- An FP64 exhaustive scan over the eligible set with tie-break (distance, id). The existing harness oracle uses FP64 truth.
- Requirement: for every exact path (CPU certified, GPU v2 + certified rescore), the returned id list must be **identical** to the oracle's on 100% of queries. Any mismatch is a correctness failure, not a recall statistic.
- Recall@10 is reported only for ANN arms, if any.
- Additional property-based tests (randomized D ∈ [1, 1024], k ∈ [1, 64], deletions, mutations between queries, stale-plan invalidation), building on the existing core suite.

#### Warm-up and execution protocol
- Fresh process per (policy, condition).
- Dataset load, then the **probe phase** (OSR's online calibration: ≤ 100 queries or ≤ 50 ms, counted in OSR's cost and reported separately).
- Then 200 warm-up queries (the paper's convention) for forced arms only. OSR must not get a free warm-up beyond its probe, because warm-up is part of what it is being judged on.
- Then 3 passes over the recorded query stream (≥ 5,000 queries).
- Telemetry starts before process launch and ends after exit: nvidia-smi at 50 ms (continuity), plus direct NVML polling at ≥ 10 ms on Linux where possible, plus per-query traces (N5).

#### Replication unit, blocking, randomization
- Replication unit = a **block**: one execution of every policy × condition cell on one host, each in a fresh process, in randomized order (seeded and recorded).
- Blocks are nested in hosts, so host is a random effect.
- Calibration/evaluation split: router design and hyperparameters (conformal α, window W, exploration budget) are frozen using **design hosts** (the laptop plus one 4090 pod) and design workloads. Evaluation uses **held-out hosts** (other 4090 pods, the H100, the non-NVIDIA GPU, the AVX-512 host) and held-out workloads (the D = 768/1024 corpus, bursty arrivals, co-load). This fixes the paper's development/held-out leakage concerns.

#### Sample size / power rationale (a judgment, not a measurement)
- The paper's light-path between-process log-spread reaches ln 4.6 ≈ 1.5 and is bimodal, so assume a between-block SD of log regret-ratio of σ ≈ 0.5 unpaired. Pairing within blocks plausibly reduces this to σ_d ≈ 0.25.
- To detect a median log-ratio of 0.2 (≈ 22%) at α = 0.05 with power 0.8: n ≈ ((z₀.₉₇₅ + z₀.₈) · σ_d / δ)² ≈ (2.8 × 0.25 / 0.2)² ≈ 12 blocks per host class.
- Plan: ≥ 12 blocks per held-out host class (e.g. 3 hosts × 4 blocks for the 4090s), and ≥ 5 blocks on single hosts (H100, non-NVIDIA, AVX-512 host).
- Pre-register a sequential check after 6 blocks. Stop early for futility only if the 95% CI of the OSR/static log-ratio lies entirely above −0.05; otherwise continue to the planned n. No early stopping for success.

#### Measurement methodology
- Device times come from GPU timestamp queries (nanosecond period from the adapter).
- E2E uses a monotonic host clock.
- Decision overhead uses rdtsc/`Instant` around the routing function only, aggregated per process.
- Energy integrates power samples over each pass and is divided by queries. Report "energy per pass" with the sampling limitation stated (Yang et al.).
- Record the per-query route, features, prediction and conformal bound in traces.

#### Statistical analysis plan (pre-registered)
- Primary: the median over blocks of per-block log(regret_OSR / regret_static) for max-regret and p95-regret across the held-out workload suite, with a 95% percentile block bootstrap (20,000 draws), stratified by host. Secondary: a linear mixed model `log t ~ policy × E-bin × arrival + (1 | host) + (1 | host:block)`.
- H2 endogeneity: a regression of probe-query GPU latency on preceding GPU duty within windows of {1, 10, 100} ms. Use an interventional design rather than observational: in dedicated blocks, force prefix sequences (0, 1, 5, 20 GPU queries at fixed gaps) before each probe, then randomize prefix length. This makes the effect causal.
- Report full latency CDFs and tail metrics per policy and host. Never report only the best run.
- Multiple comparisons: Holm correction across the 4 primary hypotheses.
- Effect sizes come with CIs. With n < 10 per stratum, intervals are labelled descriptive (as the paper does).

#### Ablations
- OSR without state features; without conformal guard (point estimate only); without exploration; without the open-time probe (prior only); probe-only (no online updates).
- GPU arm v1 vs v2 kernels (the R2 interaction).
- CPU arm 1T-AVX2 (paper) vs N1-strong.
- Hedging on or off.

#### Stopping and exclusion rules (pre-registered)
- Exclude a block if the host CPU micro-probe is > 3× slower than the host-class median (cf. the paper's 14× pod), if any clock-lock assertion fails in locked arms, if GPU temperature exceeds a pre-set limit, if the binary hash or features mismatch, or if correctness fails (also logged as a bug).
- Excluded blocks are archived, never deleted, and reported with their counts.
- Any amendment is committed and hashed before data collection resumes (mirroring E-SPLIT's amendment practice).

#### Expected failure modes
- (i) After N1, the CPU wins for all E ≤ 0.3·N at N = 100k, so routing is trivial. Mitigated by including N = 1M and batch 8, and reported as a finding in its own right.
- (ii) With v2 kernels the bimodality disappears, so state features add nothing. Then R1 collapses to R5, and H0a confirmed becomes the main result: "the paper's effect is kernel-design-dependent".
- (iii) OSR oscillates because routing to CPU cools the GPU, raising predicted GPU cost and keeping the GPU cold: a self-fulfilling cold state. Mitigate with periodic forced exploration, and report it as evidence of performativity.
- (iv) Telemetry is too coarse to observe ramp dynamics, so rely on the interventional H2 design and device timestamps.
- (v) Cloud host heterogeneity dominates. A host random effect accounts for it, and per-host conclusions are reported.

#### Reproduction commands (specification)
- `qenlo-bench run --dataset <path> --backend auto --router {static4096|edb1e6|profile|osr|osr-nostate|osr-noconformal|osr-hedge} --stream <replay.jsonl> --eligible-dist loguniform:10:0.3N --arrival {closed|poisson:1ms|poisson:10ms|poisson:100ms|mmpp:spec} --k 10 --batch 1 --order-seed <s> --diagnostics detailed --trace <out/trace.parquet> --fingerprint <out/fp.json>`. New flags marked here are PROPOSED; `--backend`, `--k`, `--batch`, `--order-seed`, `--diagnostics` and `--eligible-count` exist today per `run_dvfs_cell.py`.
- A block driver extending `research/scripts/run_dvfs_cell.py`: shuffle cells with a recorded seed, start telemetry, then hash and check outputs.
- Analysis: `research/scripts/analyze_router_heldout.py`, committed with `protocol.md` and `protocol.sha256` before the first run (E-LOCK/E-SPLIT practice).

#### Criteria
- **Success:** H1 holds on ≥ 2 of 3 held-out host classes with the primary CI excluding 0, OSR overhead ≤ 5 µs p50, 100% exactness, and the H3 ablation shows that state features contribute (or H2 shows significant endogeneity).
- **Failure:** OSR is not better than static on held-out hosts (CI includes 0 or favours static) on ≥ 2 host classes, or overhead > 20 µs p50.
- **Inconclusive:** CIs straddle 0 at the planned n, or the CPU dominates all cells (routing moot). Report the Stage 0 results as the contribution and narrow the claim.

### Gaps
- The 1M-row D ≥ 768 public corpus is not yet chosen and its licence is unchecked.
- Clock locking is unavailable in cloud containers (per the paper), so locked-clock arms are laptop-only unless bare-metal hosts are rented.
- NVML polling faster than nvidia-smi's 50 ms, and its accuracy for µs-scale kernels, is unverified. Device timestamps remain the primary timing source.

---

## Q-Extra. Per-direction experiment specs (condensed, PROPOSED) for the other recommended research items

### Takeaway
R2, R4, R5 and R6 each have a compact specification below, using the same infrastructure as the flagship: fresh-process blocks, FP64 oracle, block bootstrap, traces and fingerprints.

### Cited Findings
- The infrastructure exists: E-LOCK/E-SPLIT protocols and scripts, the heater, telemetry and FP64 oracle tests. — [REPO/research/experiments](https://github.com/a3ro-dev/qenlo/tree/142e96cf43684a13e95513a161bda4515ad6f1ca/research/experiments); QENLO-RESEARCH-PAPER.pdf Appendix A

### Inferences
- **R2 (occupancy × DVFS).**
  - RQ: does the light path's low-clock mode come from its single-workgroup selection? H: with the v2 multi-workgroup kernel, unlocked light-path device time max/min ≤ 1.15 at E = 3k/10k, and the median active SM clock rises by ≥ 30%. Null: no change in spread or clock.
  - Design: kernel {v1, v2} × clock {U, Lhi, Llo} × E {1k, 3k, 10k, 30k} × 5 blocks on the laptop (150+ processes, as E-LOCK). Replicate the unlocked arm on ≥ 3 4090 pods × 4 blocks.
  - DVs: device time, active SM/mem clock, utilization.
  - Analysis: the v2/v1 ratio of the across-block max/min, and the paired per-block log ratio, with bootstrap CIs.
  - Exclusions: as E-LOCK.
  - Success: either outcome is informative. Report it as "kernel design determines the power state" or "the effect survives better kernels".
- **R4 (certified quantized).**
  - H: SQ8 scoring with a deterministic bound plus exact rerank gives ≥ 1.5× lower GPU device time at E ≥ 10k and ≥ 1.3× lower CPU time at E ≥ 10k, with 100% oracle-identical ids. Null: no speed-up.
  - DVs: candidate-set fraction after filtering, scoring and rerank times, memory.
  - Falsifier: candidate fraction > 10% of E at k = 10. Datasets: AG News 100k plus the 1M/D768 corpus (higher D tightens RaBitQ-type estimators but loosens deterministic L1 bounds; measure both).
  - Baselines: FP32 certified (N3), RaBitQ (probabilistic; reported separately as non-exact).
- **R5 (cross-host transfer).**
  - Leave-one-host-out on archived data (laptop, 4090 host A, pods, H100), then a new probe-feature collection.
  - Models: static, global ML model, factored analytic + probe, hierarchical Bayesian.
  - Metric: regret-weighted misrouting on the held-out host. Success: the factored model with probe beats the static rules in ≥ 3 of 4 folds.
- **R6 (co-location + energy).**
  - Co-load {idle, MiniLM-embedding inference at {10, 100} QPS on the same GPU, heater} × path {CPU, light v2} × E {1k, 10k} × clocks {U, Lhi (laptop)} × ≥ 5 blocks per host on 2 GPU families.
  - DVs: e2e and device distributions, J/query (GPU NVML + CPU RAPL), co-load latency impact (does the retrieval kernel slow the model?).
  - H: under real co-load, light-path device time is closer to its locked-boost value than when idle. Null: no difference.
  - Report energy and both workloads' latencies (retrieval *and* the co-located model), because a retrieval gain that slows the model is not a win.

### Gaps
- None beyond those listed in the flagship.
