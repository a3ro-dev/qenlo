# review checklist (endorsement feedback, 2026-10-25)

grouped by cost. ✅ = done, ☐ = open. "push back" = the point is partly wrong; answer it in the text, don't just concede.

## A. text-only (no new data, do first)

- [x] **retitle.** drop "The Efficient Kernel Runs Slow". candidate: "Power-State Variance in One WGPU Vector-Search Kernel" (reviewer's) or "Driver Power States Make a Light WGPU Search Kernel Irreproducible". title must not imply a law.
- [x] **downgrade "reproducibility asymmetry"** from contribution to observation (main.tex:28, :41). make the contributions list say "in one engine, one kernel pair".
- [x] **drop "pre-specified" everywhere except E-LOCK.** abstract (main.tex:21), intro (:37, :42), setup (sec_static.tex:26), scorecard heading (sec_results.tex:69). call D1/D1R/H100 "planned analyses, protocol committed after collection". appendix already says this; the body must match.
- [x] **K2: stop the rescue.** sec_results.tex:95 "direction held, tolerance was wrong" → state plainly: K2 failed. separately, as a post-hoc observation, locked-high beat every unlocked run.
- [x] **lead with the regime that matters (push back, partly).** the reviewer says the variance only exists where the CPU wins. table 7 shows the CPU wins at E ≤ 3,000, but at E=10,000 the GPU wins on the laptop (1.68 vs 3.33 ms) and the pods (0.43 vs 1.50 ms), and the pods' light spread there is **4.61×** (cross_host.tex). move the headline cell from E=3,000 to E=10,000. also be honest: 0.46 ms of device variance is a small share of 1.68 ms end to end.
- [x] **reframe the heavy path (push back, partly).** it is a load contrast, not a baseline we claim is good. say so in sec_static.tex:11 and the intro: "deliberately naive selection (k passes in one workgroup)". the claim is about these two paths in qenlo, not GPU search in general.
- [x] **say what the paper is for.** the pitch is qenlo as a cheap instrument for AI-systems research: one laptop plus ~US$6 of cloud found, explained and causally tested (E-LOCK) a 4× benchmark artifact, with all data released. one paragraph in the intro and conclusion. this also makes the narrow scope a feature, not a hole.
- [x] **concede DVFS is known.** related work already cites it. put "GPUs downclock under light load; benchmarkers lock clocks for this reason" in the intro's first paragraph, then say what is new: (a) it reorders/rescales the margin between two implementations of the same op, (b) magnitude 4× rather than 8–22%.
- [x] **headline stats, not medians-of-five.** "3.8× across hosts" (main.tex:28, sec_results.tex:38, :102) is a median of 5 draws from a bimodal distribution. replace with per-process points (strip plot) plus min–max; drop bootstrap CIs where n ≤ 5, or label them "descriptive only".
- [x] **state the host/process confound.** pods: one block per host means host and process variance cannot be separated. say it in setup, not only in limitations.
- [x] **"six 4090 hosts" → "host A (Sep 24 archive) + five pods (Sep 25)".** note the two driver branches (570/580) wherever they are pooled (sec_results.tex:102).
- [x] **fix 1% / 7% / 4%.** 1% = within one machine, 7% = max/min across pods, 4% = host A median vs pod median (0.711 vs 0.685). say which is which everywhere (abstract, main.tex:28, :41, sec_results.tex:38).
- [x] **ρ = −0.66:** move it to the appendix or drop it. it's exposed to regression to the mean and adds nothing table 3 doesn't show.
- [x] **heater: own the failure.** say up front that the heater is a CPU+GPU perturbation and that P1 failed on every GPU. stop calling it "the intervention" as if it were clean; the clean test is E-LOCK.
- [x] **mechanism: concede or test.** (tested: E-SPLIT, SM clock) "we do not know what the governor responds to" stays, but the claim scope shrinks to "observed clock states correlate with and (on one laptop) cause the variance". see B1 for the test.
- [x] **laptop environment:** state Optimus/hybrid graphics (iGPU + dGPU), Windows power plan, AC, driver 616.56. one line in setup.
- [x] **telemetry caveat in body:** 50 ms nvidia-smi is part-time sampling; say so where table 2 is introduced, not only in the caption.
- [x] **D2 "not yet run":** remove it from the protocol list or move it to future work.
- [x] **slow pod (14× CPU):** already excluded; keep it, and say once in the main text why it doesn't affect GPU device timings.
- [x] **de-AI pass.** the reviewer flagged the prose. read every section aloud, cut parallel triads and "what differs is…" constructions, vary sentence length. (humanizer skill.)

## B. laptop experiments (free, you run from an admin shell)

- [x] **B1. SM vs memory 2×2 lock (E-SPLIT, done: slow mode is the SM clock).** extend `research/scripts/run_dvfs_lock.py` with two more conditions: SM hi + mem lo (2670/810) and SM lo + mem hi (585/8001). light + heavy, E ∈ {1k, 3k, 10k}, 5 blocks → +60 processes (~same wall time as E-LOCK's 150 × 0.4). **commit + hash the protocol before running** (repeat what E-LOCK did). this answers the "biggest hole".
- [ ] **B2. more laptop blocks.** rerun D1 unlocked with 15+ blocks (shuffled, fresh processes) so the bimodality is visible as a distribution, not 5 points. pre-commit protocol first.
- [ ] **B3. 1 ms or CUPTI-free clock sampling** (optional): log `nvidia-smi --query-gpu ... -lms 10` or NVML via pynvml at 10 ms during a few processes to show the 50 ms sampling isn't hiding anything.
- [ ] **B4. dGPU-only check** (optional): if the BIOS/NVIDIA panel allows "dGPU only" / MUX mode, rerun one block to rule out Optimus.

## C. out of scope (say so in the paper, don't run)

no FAISS/cuVS baseline and no more cloud runs. the paper shows what qenlo can do as a cheap research component, not a survey of GPU search libraries. put both in limitations / future work: "a tuned library kernel and more hosts per GPU model need compute we don't have".

## D. what to send back

- [ ] short reply: thank them, list what changed, say which points you answered with existing data (E=10k regime, heavy path as load contrast) and with B1, and that the baseline and cloud points are now scoped out explicitly instead of claimed around.
- [ ] resend once A + B1 are in.
