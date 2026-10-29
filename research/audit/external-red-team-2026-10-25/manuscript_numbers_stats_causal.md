# Qenlo manuscript audit: claim reconstruction, numerical integrity, statistics, causal identification, logical consistency

Scope: Phase 1, 2A, 2B, 2C, 2F and Phase 4 inputs of the audit brief, done **from the PDF only** (no raw repository data; a separate researcher recomputes from raw data). Manuscript: `/home/claude/QENLO-RESEARCH-PAPER.pdf` (13 pp., pdfTeX, CreationDate "Sun Oct 25 09:41:31 2026 Asia"). All 13 pages were read as text (`pdftotext -layout`) and rendered to PNG; Figure 1 panels (a), (b), (c) and pages 11-13 (Appendices, Tables 6-8) were inspected visually at 400 dpi.

Citation convention: `[MS p.N]` = the manuscript PDF at /home/claude/QENLO-RESEARCH-PAPER.pdf, page N (a primary source for what the author claims, not for whether it is true). `[Script]` = my recomputation, `/home/claude/research_notes/Qenlo paper red team audit/scripts/manuscript_recompute.py`, full output in `.../scripts/manuscript_recompute_output.txt` (run with `python3 -I`, numpy 2.5.3, scipy 1.18.1). Every number labelled "recomputed" is computed by me from the manuscript's own table entries; every number labelled "reported" is only the author's.

---

## Q1. Central claim, strongest implied version, and the gap (Phase 1)

### Takeaway
The evidence supports a narrow claim: on one Windows laptop (RTX 4050 Laptop, driver 616.56), the light GPU path's top-k kernel time varies across fresh processes, and fixing the SM clock removes that variation; this is a single-machine causal result. The cross-host story, the "benchmarks understate the efficient implementation" generalization and any application-level significance go well beyond it. The paper's own tables show the kernel effect barely reaches end-to-end latency.

### Cited Findings
- Research question as posed: what default (unlocked) GPU power management does to a comparison between two GPU paths of one exact filtered vector-search engine that return identical top-k results [MS p.1, Abstract].
- Mechanism proposed: a work-efficient kernel "that leaves the GPU mostly idle can be clocked down; a wasteful one that keeps it busy is not. The measured gap between them then partly reflects the power state each one induced" [MS p.1, Introduction].
- Contributions claimed: (1) an observation in one engine (light path less reproducible: up to 4.2x within machine, 4.6x across five pods; heavy path at most 1% / 7%); (2) a locked-clock test on one laptop, plus an SM-vs-memory split; (3) "weaker evidence across hosts" (telemetry + heater, failed end-to-end prediction); (4) reporting rules and released data [MS p.2].
- Author's own hedges: "The magnitudes belong to this engine and these machines" [MS p.1]; "We treat the cross-host account as supported by telemetry and the heater, not as tested" [MS p.1]; "We do not expect these magnitudes to carry over" [MS p.8].
- Stronger implied claims: the Section 8 heading "Unlocked-clock benchmarks can understate the efficient implementation" and "Optimizations that remove GPU work ... can be measured at a discount the benchmark does not show" [MS p.6]. "Isolated benchmarks may misstate co-located latency" [MS p.6-7]. The conclusion says a small instrumented engine "should find similar effects in other kernels for little money" [MS p.8]. The related-work claim "We have not found a report that this changes the measured margin between two implementations ... or that an unrelated co-running GPU load speeds up a latency-bound GPU kernel" [MS p.7].

### Inferences
- **Central claim, one sentence, as the evidence supports it:** On one RTX 4050 Laptop GPU under Windows/Vulkan/WGPU, the driver's default clock selection is the dominant source of process-to-process variation in the top-k kernel time of Qenlo's compact-row (light) GPU path at batch 1. Locking SM+memory clocks removes that variation, and a low SM lock reproduces the slow mode. Telemetry on RTX 4090 and H100 cloud hosts and a crude load-raising "heater" are consistent with, but do not establish, the same mechanism there.
- **Strongest version the author implies:** Default GPU power management systematically penalizes work-efficient (latency-bound) GPU implementations relative to wasteful ones. Unlocked benchmarks therefore understate efficient implementations by large factors (2.9-5.0x), and this distorts CPU/GPU routing and co-location latency estimates across machines.
- **Gap:** (i) The causal result has n=1 machine, 1 driver, 1 OS, 5 blocks. (ii) The "understatement" factor depends on which unlocked baseline is used: 2.9-5.0x against E-LOCK's own unlocked arm, 1.25-4.08x against D1's unlocked medians, and roughly 1.45-2.0x against the best unlocked (heater) configuration [Script; see Q2]. (iii) The device-time effect barely reaches end-to-end latency, and no end-to-end latency is reported under clock locking at all [MS p.5-6, Table 4 is device-only]. (iv) The heavy comparator is "deliberately naive", so "margin between implementations" generalizes only to a strawman pair [MS p.3]. (v) The phrase "regime where an application would choose it" at E=10,000 depends on the CPU baseline switching to FP64 above E=4,096 (Q2, item N14). (vi) On the cloud hosts, the clock-telemetry-to-device-time link is quantitatively inconsistent (Q4).

### Gaps
- The intended venue is not stated. The paper reads as a measurement/experience paper (workshop or short-paper category) rather than a full systems contribution. This is a judgment, not a sourced fact.

---

## Q2. Numerical integrity: every recomputed number (Phase 2A)

### Takeaway
Table arithmetic is internally clean. All 24 Table 1 max/min entries, all 12 Table 5 m values, the Table 6 counts, Appendix C's 3.36x and the Spearman p reproduce within display rounding. The problems are about **scoping and baselines**:
- The "4.2x" is E=3,000, not the E=10,000 cell the abstract sentence attaches it to.
- E-LOCK's unlocked baseline is 1.2-2.3x slower than D1's unlocked baseline on the same laptop.
- The heater effect at E=1,000 is 0.57 in E-LOCK but 1.00 in D1.
- Host A is described as 5 blocks in Table 1 and as "ten identical blocks" in Section 4.
- Pods E=30,000 is a tie scored as a hit.
- The H100 is absent from Table 8, though the abstract and Figure 1 caption imply that the light path beats the CPU at E=10,000 on all GPUs shown.

### Cited Findings (manuscript inputs; recomputations from [Script])
**N1. Table 1 max/min vs min-max ranges [MS p.4].** All 24 (12 heavy, 12 light) max/min values are consistent with the printed min-max ranges, given 3-decimal rounding of the endpoints. Examples (recomputed):
- 0.230/0.055 = 4.182, rounding-feasible [4.135, 4.229], reported 4.16, OK.
- 0.589/0.128 = 4.602, feasible [4.580, 4.624], reported 4.61, OK.
- 0.065/0.051 = 1.275, feasible [1.252, 1.297], reported 1.27, OK.
- 0.056/0.027 = 2.074, feasible [2.018, 2.132], reported 2.08, OK.
- 0.216/0.050 = 4.320, reported 4.30, OK.
- 0.512/0.337 = 1.519, reported 1.52, OK.

Heavy within-machine max/min ≤ 1.0053 (claim "≤1%") and across pods ≤ 1.0736 (claim "7%"), OK. The light path is faster than heavy in all 12 cells, OK.

**N2. Scoping of the "up to" spreads [MS p.1, p.2, p.2 caption, p.4].** Recomputed from Table 1 light max/min:

| Cell | Value |
|---|---|
| Laptop E=1k | 1.00 |
| Laptop E=3k | **4.16** |
| Laptop E=10k | 1.40 |
| Host A E=1k / 3k / 10k | 1.27 / 2.56 / 1.52 |
| Pods E=1k / 3k / 10k | 2.08 / 4.30 / 4.61 |
| H100 | ≤ 1.18 |

- The abstract says "The light kernel's varied up to 4.2× on the laptop and 4.6× across the pods, **including at E = 10,000**, where the light path is faster than our CPU scan" [MS p.1]. The 4.2x is the E=3,000 cell, where the CPU wins (Table 8, laptop E=3,000: CPU 0.308 ms vs light 0.548 ms). At E=10,000 the laptop spread is 1.40x. The sentence is literally true ("up to ... including") but juxtaposes a 4.2x figure from a CPU-winning regime with the GPU-winning regime.
- The introduction scopes correctly: "1.4× between fresh processes on the laptop and 4.6× across five RTX 4090 cloud pods" at E=10,000 [MS p.1].
- The Figure 1 caption says "the light path varies up to 1.5× on a machine and 4.6× across the pods" [MS p.2]. This is panel (a) only (E=10,000; host A 1.52). It is consistent with the panel but conflicts on its face with Contribution 1, "up to 4.2× between fresh processes on one machine" [MS p.2]. These are three different "up to" numbers for "one machine" (4.2, 1.5, 1.4), each from a different E/GPU slice.
- Section 4 says "4.3-4.6× across the five 4090 pods" [MS p.4]. This omits E=1,000 (2.08x); "reaches 2.6× on 4090 host A" = E=3,000 (2.56), OK.

**N3. Section 8 device-time advantage (heavy/light, E=10,000, Table 1) [MS p.6].** Recomputed: host A 0.703/0.440 = 1.60x; laptop 0.941/0.461 = 2.04x; pods 0.673/0.142 = 4.74x; H100 0.977/0.205 = 4.77x. Reported 1.6/2.0/4.7/4.8, OK. Locked at boost (Table 4): 0.941/0.113 = 8.33x, reported 8.3x, OK.

**N4. "2.9-5.0× faster at fixed boost than with default clocks" [MS p.6].** Recomputed Table 4 U/Lhi: 0.069/0.024 = 2.88, 0.176/0.045 = 3.91, 0.567/0.113 = 5.02, OK as arithmetic. But:
- **E-LOCK's unlocked (U) medians are much slower than D1's unlocked medians on the same laptop** (Table 1 laptop light): 0.069 vs 0.030 (2.30x), 0.176 vs 0.106 (1.66x), 0.567 vs 0.461 (1.23x) [Script].
- Against D1's unlocked medians the "understatement" is 1.25x / 2.36x / 4.08x.
- Against the best unlocked configuration (heater on), my approximation (U × heater ratio 0.57/0.37/0.40 from Sec. 7; a ratio of medians, so approximate) gives about 1.64x / 1.45x / 2.01x.
- The headline factor therefore depends heavily on which unlocked run is the comparator, and the comparator chosen is the slowest available.

**N5. E-LOCK variation "2.8× to 1.00×" [MS p.1, p.5, Table 4 p.6].** 2.84 is the unlocked E=1,000 max/min. D1 measured 1.00 max/min at the same laptop and E=1,000 (Table 1). In D1, E=1,000 was the cell with **no** variation. The variation that locking removed in E-LOCK is therefore not the same variation documented in D1. Unlocked behaviour of this laptop changed between D1 (data 2026-09-25) and E-LOCK (2026-10-19) [MS p.11 for dates]. The manuscript does not discuss this drift.

**N6. K2: "32-50% less time" vs "0.50-0.68 of the best unlocked median" [MS p.6, p.11].** These are mutually consistent: 1-0.68 = 0.32 and 1-0.50 = 0.50. My reconstruction, using the best unlocked = U+heater approximation, gives Lhi/best of about 0.61, 0.69, 0.50 [Script]. This matches 0.50-0.68 within the approximation error (0.69 vs 0.68). Against heater-off U the ratio would be 0.35/0.26/0.20, so "best unlocked" must be the heater-on arm. The U+heater and Lhi+heater cells (2 of the 5 E-LOCK conditions) are **not tabulated anywhere** (Table 4 shows U, Lhi, Llo and heavy Lhi only) [MS p.6]. K2, K4 and the 0.57/0.37/0.40 figures are therefore unverifiable from the PDF.

**N7. K3: "0.444 ms, within 4% of the 0.461 ms that D1 measured" [MS p.5].** Recomputed |0.444/0.461 − 1| = 3.7%, OK. Against E-LOCK's **own** unlocked arm at E=10,000 (0.567 ms), the low lock differs by 21.7% and is *faster* than the unlocked median (Llo/U = 0.78). K3 was registered against D1, so using D1 follows protocol. Still, "within 4%" is the most favourable of the two available references, and both should be shown. Recomputed Llo/U at E=1k/3k/10k: 1.39 / 1.01 / 0.78. The unlocked median at E=10,000 sat *below* the "low state" lock, so the lock does not bound the slow mode from below.

**N8. Heavy-path low-lock slowdown "3.8× (0.95 to 3.6 ms)" [MS p.5-6].** The heavy Llo value is not in Table 4 [MS p.6]. It appears only in E-SPLIT Table 5 (heavy LL 3.615 / HH 0.947 = 3.817 at E=1,000) [MS p.7]. 3.6/0.95 = 3.79, OK. This is a cross-reference defect: the Section 7 sentence cites E-LOCK, but the number is shown only for E-SPLIT. K5 ("Lhi/U = 1.000, 1.000, 1.002") is unverifiable because heavy U is not tabulated.

**N9. Table 5 memory share m = ln(tHL/tHH)/ln(tLL/tHH) and SM share s = ln(tLH/tHH)/ln(tLL/tHH) [MS p.7].** All 12 reproduce (recomputed, with the range over ±0.0005 rounding of each input):

| Path | E | m sel | m score | s sel | s score | m+s score |
|---|---|---|---|---|---|---|
| Light | 1k | 0.000 [−0.031, 0.030] | 0.000 [−0.130, 0.115] | 1.000 | 1.000 | 1.000 |
| Light | 3k | 0.016 [0.000, 0.032] | 0.000 | 1.000 | 1.000 | 1.000 |
| Light | 10k | 0.068 (rep 0.07) | **0.479 (rep 0.48) [0.468, 0.490]** | 0.983 | 0.870 | **1.349 (rep 1.35)** |
| Heavy | 1k, 3k | 0.000 | 0.000 | 1.000 | 1.000 | 1.000 |
| Heavy | 10k | 0.020 (rep 0.02) | 0.243 (rep 0.24) | 0.995 | 0.908 | 1.151 |

- "m = 0.48, just inside the registered limit of 0.5" survives rounding (upper bound 0.490).
- "doubled light scoring (0.034→0.069)" = 2.03x, OK. "raised heavy scoring by 44%" = 0.282/0.196 = 1.439, OK. "slowed the light path by 10% at E=10,000" = 0.124/0.113 = 1.097, OK.
- Light E=1,000 and E=3,000 m values for scoring have very wide rounding ranges (±0.12) because the times are 0.007-0.044 ms printed to 3 decimals. m=0.00 there means only "not resolvable at the printed precision".
- LH light at E=1,000 has n=1 [MS p.7 caption]. The s for that cell rests on one process, and S1 (max/min ≤ 1.15) is vacuous with n=1.

**N10. "Re-running E-LOCK's two corners reproduced its medians within 0.2%" [MS p.6].** At printed precision, light E=3,000 is HH 0.044 (E-SPLIT) vs Lhi 0.045 (E-LOCK), −2.2%. This is within rounding of a ≤0.2% difference only if the true values sit near 0.0445. Heavy HH vs Lhi: −0.11%, 0.00%, −0.11%. Not contradicted, but not verifiable from the printed precision.

**N11. Heavy scoring time grows with E** (Table 5 HH: 0.126 → 0.140 → 0.196 ms, 1.56x from E=1k to 10k) [MS p.7]. Section 2 says heavy "device work is proportional to N regardless of E" [MS p.3]. The selection kernel is flat (0.947-0.951), but the scoring kernel is not. The description should say "selection work"; the cause (predicate divergence, writes of eligible scores) is unexplained.

**N12. Table 3 percentages vs n [MS p.5].** All are achievable fractions: 52% = 13/25, 28% = 7/25, 16% = 4/25, 11% = 1/9 (11.1%). Descriptive Fisher exact tests (ignoring pairing and non-independence; illustration only): laptop 13/25 vs 0/25 p = 2.9e-5; pods 7/25 vs 4/25 **p = 0.50**; H100 1/9 vs 0/9 **p = 1.0** [Script]. The claim that slow runs "become rarer on every GPU" rests, on pods and the H100, on differences of 3 runs and 1 run. "With a tail reaching 3.3×" [MS p.4] is the pods p90 (3.29), not the maximum. Figure 1c shows heater-off pod squares up to about 4.8x (visual read), consistent with Table 1's 4.61x.

**N13. Table 6 counts vs scorecard vs Table 7 [MS p.5, p.12].** Recomputed counts from Table 6 entries: P3 end-to-end yes = 10/13 (claim "10 of 13"), P3 device yes = 13/13 ("every cell"), P4 yes = 6/13 ("6 of 13"), all OK. "Light path fared better than the heavy path in 10 of 13 cells" equals the P3-call count, OK. Every Table 6 P3 entry follows from Table 7 point estimates except **pods E=30,000**:
- Light device 1.00 vs heavy device 1.00 is a tie at printed precision, yet scored P3-device "yes".
- Light call 1.07 vs heavy call 1.07 is a tie, scored "no".
- P4 is "yes" there although light device ratio = 1.00 [0.46, 1.00], so no fall is visible at printed precision.

"P3 held in every cell for device time" therefore includes a tie. Pods E=100 has light device 0.92 but P4 "no", so the host-materialization arm must have failed; consistent with the rule but not shown.

**N14. Table 8 winners and CPU baseline [MS p.13].**
- Winners (light vs CPU) recomputed as labelled in all 10 rows; "heater never changed a winner", OK.
- The H100 and 4090 host A are absent from Table 8. The abstract's "including at E = 10,000, where the light path is faster than our CPU scan" (in a sentence listing the H100) and Figure 1a caption "At this E the light path is faster than the CPU scan" (panel includes host A and H100) are unsupported for those two GPUs. The introduction correctly limits the claim to the laptop and pods [MS p.1].
- **CPU per-row cost jumps 3.25x (laptop) and 3.37x (pods) between E=3,000 and E=10,000** (0.103 → 0.333 µs/row; 0.044 → 0.149 µs/row). This coincides with the CPU scan switching from FP32 + FP64 boundary rescoring (E ≤ 4,096) to FP64 throughout [MS p.2].
- Extrapolating the E=3,000 per-row rate linearly gives a CPU time at E=10,000 of about 1.03 ms on the laptop (vs light GPU 1.682 ms) and about 0.443 ms on pods (vs 0.430 ms). This is my inference, not measured; the linear-scaling assumption is untested.
- So the "regime where an application would choose it" at E=10,000 is plausibly **created by the CPU baseline's precision switch**, not by the GPU's intrinsic advantage.

**N15. Table 8 vs Table 7 aggregation [MS p.12-13].** Light-call heater on/off as a ratio of the Table 8 "median-block" values vs Table 7's median of per-block ratios:

| Cell | Table 8 on/off | Table 7 |
|---|---|---|
| Laptop E=100 | 1.47 | 1.11 |
| Laptop E=1,000 | 1.63 | 1.13 |
| Laptop E=3,000 | 1.02 | 0.96 |
| Laptop E=10,000 | 1.00 | 0.97 |

These differences are legitimate (different estimators), but their size shows strong block heterogeneity. Read through Table 8, the heater made laptop light calls 47-63% slower at small E. "Median-block" is undefined (which block, and is it chosen per column?).

**N16. Busy fraction "light path spends 13-43% of call inside kernels"; heavy "52-84%" [MS p.4].**
- From Table 1 (top-k only) / Table 8 (light call), laptop: 0.030/0.230 = 13.0%, 0.106/0.548 = 19.3%, 0.461/1.682 = 27.4%. The 13% end reproduces.
- 43% presumably needs scoring time and/or E=30,000, which are not tabulated for D1. **Unverifiable.**
- Heavy laptop top-k alone 0.95/2.0 = 47%, so 52% plausibly includes scoring. Unverifiable.
- **Key recomputation:** at *locked boost* (E-LOCK Lhi selection + E-SPLIT HH scoring) over the D1 laptop light call, the busy fraction is 13.5% (E=1k), 10.4% (3k), 8.7% (10k). The *exogenous* GPU busy fraction falls as E grows, because host-side work (materializing, sorting and uploading E ids) grows faster than device work. See Q4: this contradicts the paper's statement that "a simple busy-fraction account does not explain" the clock falling with E [MS p.4].

**N17. Laptop E=10,000: device vs end to end.** The heater cut light device time to 0.46 of heater-off (Table 7), about 0.25 ms of 0.461 ms. Yet the median-block light call went 1.682 → 1.685 ms (Table 8) [MS p.12-13]. The kernel speed-up does not reach the call at all on this cell.

**N18. Appendix C [MS p.11-12].** "235.7% regret" → ratio 1 + 2.357 = 3.357, "3.36×" OK. "Median regret 0.4%" with 8 of 16 wrong: with the usual even-n median and zero regret for correct choices, the median is (0 + smallest non-zero regret)/2, so the smallest wrong-route regret ≈ 0.8%. Plausible; per-workload regrets are not in the PDF.

The argument "the maximum-regret rejection survives: a 3.36× ratio exceeds every within-host spread ... except the laptop light path's 4.2×" is self-undermining. The held-out suite ran **on the RTX 4050 laptop** [MS p.11], and that is exactly the host whose light-kernel spread (4.16x) exceeds 3.36x. Kernel-time spread is also not the same quantity as end-to-end route regret.

**N19. Spearman ρ = −0.83, p = 0.003, n = 10 [MS p.4].** Recomputed: t-approximation t = −4.209 on 8 df, two-sided p = 0.0030, matches. Exact permutation (no ties): two-sided P(|ρ| ≥ 0.83) = 0.0047; one-sided 0.0024 [Script]. The reported p is the asymptotic approximation; the exact value (0.005) should be reported for n=10.

**N20. Host A block count.** Table 1 caption: "RTX 4090 host A is the retained September 24 archive (one host, 5 blocks)" [MS p.4]. Section 4: "The retained archive shows the same pattern within one host over time. Across ten identical blocks ..." [MS p.4]. The text does not reconcile 5 vs 10 blocks, nor say which E the "2.44×" refers to.

**N21. Table 2 clocks [MS p.5].**
- "Heavy path holds boost SM 2,670-2,820 MHz consumer, 1,980 H100" OK.
- Memory: heavy laptop E=3,000 shows 7,001 MHz, while E=1k and 10k show 8,201. The heavy path's *memory* clock also varies, and the text does not mention it.
- E-LOCK/E-SPLIT lock memory at 8,001 MHz, described as "the heavy path's boost (2,670 and 8,001 MHz)" [MS p.5]. Telemetry shows heavy at 8,201 MHz, so the memory lock is not the observed heavy boost value.
- The light path at E=10,000 on the H100 falls to 645 MHz SM, consistent with "1,605 to 645 MHz" in the text.

**N22. Section 6: "With the heater it [heavy] becomes uniformly slower on the laptop (1.19×)" [MS p.4].** Table 7 laptop heavy device at E=30,000 = 1.00 [1.00, 1.00], and at E=100 the lower bound is 1.00 [MS p.12]. So the slowdown is not uniform.

**N23. Section 7: heater "cut unlocked light time to 0.57, 0.37 and 0.40" (E-LOCK) vs D1 Table 7 laptop light device 1.00 / 0.65 / 0.46 at E = 1k / 3k / 10k** [MS p.5, p.12]. Same laptop, same nominal intervention. At E=1,000 the heater had no effect in D1 (interval [0.98, 1.01]) and a 43% effect in E-LOCK. Like N5, this points to a changed unlocked baseline; the heater's effect itself does not replicate across experiments.

**N24. Cost [MS p.12].** "About US$6" at $0.74/h (4090) and $3.49/h (H100). This cannot be checked without hours; it rests on the operator ledger.

### Inferences
- No arithmetic fabrication or transcription errors were found in the tables. The defects are selective scoping, an unstable unlocked baseline across experiments, untabulated cells that key claims rely on (U+heater, Lhi+heater, heavy U, heavy Llo in E-LOCK), and undefined aggregations ("median-block", "lower median of three per-pass percentiles").
- The between-experiment drift (N5, N7, N23) is the most consequential numerical finding. The phenomenon's magnitude and E-dependence on the *same* laptop changed within about 3.5 weeks, and the paper presents numbers from the two experiments side by side as if interchangeable (e.g. K3 compares E-LOCK Llo to D1 U).

### Gaps
- Per-block / per-process values are not in the PDF, so Table 3 p90s, Table 7 medians and the Figure 1 points cannot be recomputed from the PDF. The raw-data researcher should check them.
- Whether the laptop driver, Windows build or power settings changed between D1 and E-LOCK is not stated. The paper lists one driver version (616.56) [MS p.3, p.8].

**N25. Telemetry vs device time (Table 2 vs Table 7), recomputed.** Assume the selection time scales as 1/SM clock; E-SPLIT supports this on the laptop with s ≈ 1 for selection. Then the heater's clock change predicts the following device ratios:

| GPU | E | SM off → heater (MHz) | Predicted | Observed | Observed/predicted |
|---|---|---|---|---|---|
| Laptop | 3k | 1080 → 1912 | 0.56 | 0.65 | 1.15 |
| Laptop | 10k | 585 → 1215 | 0.48 | 0.46 | 0.96 |
| Pods | 1k | 2520 → 2550 | 0.99 | 0.90 | 0.91 |
| Pods | 3k | 2505 → 2550 | 0.98 | 0.87 | 0.89 |
| Pods | 10k | 2235 → 2520 | 0.89 | 0.82 | 0.92 |
| H100 | 1k | 1605 → 1980 | 0.81 | 0.86 | 1.06 |
| H100 | 3k | 1275 → 1980 | 0.64 | 0.71 | 1.10 |
| H100 | **10k** | **645 → 1980** | **0.33** | **0.81** | **2.49** |

The laptop fits well. On the **pods, the median light-path SM clock is 2,235-2,520 MHz** (79-89% of the 2,820 boost) [MS p.5]. A clock-only account allows at most 2820/2235 = 1.26x slowdown at the median, while Table 1 shows a 4.61x spread and a slow-mode median of about 0.14-0.59 ms. On the **H100 at E=10,000**, the light path's median clock is 645 MHz (33% of 1,980 MHz), yet:
- its process-to-process spread is only 1.10x (Table 1);
- the heater, which restored 1,980 MHz, improved device time only to 0.81 (a 3.07x clock increase gave a 1.23x speed-up).

The cloud-host telemetry is therefore quantitatively inconsistent with "kernel time tracks SM clock". Possible reasons:
- Table 2 pools all blocks and includes warm-up and data-loading samples [MS p.8, Limitations];
- 50 ms sampling misses the clock during the short kernels [MS p.3];
- on the H100 something other than the SM clock dominates (fixed dispatch latency, a different clock domain).

Figure 1 caption's "the light kernel's speed tracks the GPU clock" is demonstrated **only for the laptop**. Panel (b) is a single laptop process per condition [MS p.2].

---

## Q3. Statistical methodology critique (Phase 2B)

### Takeaway
- The design correctly makes the fresh process (block) the replicate and refuses to treat 15,000 in-process calls as independent. That is a real strength, in line with Kalibera & Jones.
- The inferential layer does not hold up. With n = 3-5 blocks, the "95% percentile-bootstrap interval" of a median is mechanically the sample [min, max]. Its nominal coverage is impossible: about 75% at n=3, at most 94% at n=5.
- On pods, host is confounded with process. About 60+ prediction cells were tested without multiplicity control.
- The "relative to best observed" metric and medians of bimodal data hide what matters.
- Only E-LOCK and E-SPLIT qualify as pre-specified in any verifiable sense. Even they are commits to the author's own repository, not registrations in the standard sense.

### Cited Findings
- **Unit of replication.** "Every condition runs in a fresh process ... blocks are the independent replicates ... The 15,000 calls within a process are repeated measurements and are never treated as independent" [MS p.3]. Kalibera & Jones show that benchmark experiments are hierarchical (builds, executions, iterations) and that variance at the higher level must be sampled. They note few papers "mention that their experiments repeat at two levels", and those that do "treat the data as if it came from a one-level experiment" — [Kalibera & Jones, tech-report version](https://www.cs.kent.ac.uk/pubs/2012/3233/content.pdf); ISMM'13 paper [doi:10.1145/2464157.2464160](https://dl.acm.org/doi/10.1145/2464157.2464160). The manuscript's design follows this advice.
- **Pseudoreplication.** Hurlbert defines pseudoreplication as "the use of inferential statistics to test for treatment effects with data from experiments where either treatments are not replicated (though samples may be) or replicates are not statistically independent" — [Hurlbert 1984, Ecological Monographs 54(2):187-211, doi:10.2307/1942661](https://esajournals.onlinelibrary.wiley.com/doi/pdf/10.2307/1942661).
  - Applied to the laptop: one physical laptop means the *machine* is unreplicated. Five blocks replicate the process, not the device.
  - Any claim about "the RTX 4050 Laptop GPU" (rather than "this laptop") is pseudoreplicated in Hurlbert's sense.
  - Same for the H100 (one pod, n = 3 blocks) and host A (one host).
  - The paper reports "within 7% across the five pods" while also acknowledging "host and process variance are confounded on the pods" [MS p.3].
- **Bootstrap with n = 3-5.** The paper reports "95% percentile-bootstrap intervals over blocks (20,000 draws). With three to five blocks these intervals are coarse and we read them descriptively" [MS p.3]. Table 7's caption notes that "with the H100's n = 3 there are only ten distinct resamples" [MS p.12]. Recomputed [Script]:
  - With n = 3, P(bootstrap median = sample minimum) = 0.259 > 0.025. The 2.5th percentile is the minimum and the 97.5th the maximum, so the "95% interval" is exactly [min, max].
  - With n = 5, P = 0.058 > 0.025, so again [min, max].
  - The distribution-free coverage of [min, max] for a population median is 1 − 2·0.5^n = 0.75 (n=3) and 0.9375 (n=5).
  - Simulated coverage of the nominal 95% percentile-bootstrap CI for the median in a bimodal population: 0.746 (n=3), 0.928 (n=5) [Script; simulated population is my assumption, 60% fast mode / 40% slow mode].
  - Hesterberg: bootstrap percentile intervals "tend to be too narrow and under-cover in small samples" and perform "poorly in small samples, because of the narrowness bias" — [Hesterberg 2015, The American Statistician 69(4):371-386, doi:10.1080/00031305.2015.1089789](https://www.tandfonline.com/doi/full/10.1080/00031305.2015.1089789).
  - Several Table 7 intervals are degenerate or touch the point estimate (e.g. pods E=30,000 light device 1.00 [0.46, 1.00]; pods E=1,000 0.90 [0.70, 0.92]) [MS p.12]. The 3-decimal-looking intervals convey false precision.
- **Exact alternatives.** The exact two-sided sign test with all blocks in the same direction gives p = 0.25 (n=3), 0.0625 (n=5), 0.002 (n=10) [Script]. With n ≤ 5 no exact rank or sign test can reach p < 0.05, and Bonferroni for about 52 tests (α ≈ 0.00096) is unattainable. A rank-based approach without normality assumptions is the recommended practice in Hoefler & Belli's rules: "Rule 6: Do not assume normality of collected data (e.g., based on the number of samples) without diagnostic checking" — [Hoefler & Belli, SC'15, doi:10.1145/2807591.2807644](https://htor.inf.ethz.ch/publications/img/hoefler-scientific-benchmarking.pdf). Their Rule 5: report whether measurements are deterministic, and for nondeterministic data report confidence intervals — [slides](https://htor.inf.ethz.ch/publications/img/hoefler-scientific-benchmarking_wide_HLRS.pdf).
- **Multiple comparisons.**
  - The D1/D1R/H100 scorecard covers 13 cells × P1-P4, with P3 split into two: 65 cell-level verdicts [MS p.12].
  - E-LOCK has K1-K5 over 3 E values; E-SPLIT has S0-S3 over 6 path×E cells × 2 kernels.
  - No family-wise or false-discovery control is described.
  - Successes are counted per cell ("10 of 13", "6 of 13") without a stated decision rule for what fraction counts as "held".
- **Post-hoc framing.** The author concedes: "The cross-host table (Table 1) and the device-time framing were not planned; they were suggested by the retained September 24 archive and are therefore partly exploratory" [MS p.11]. The headline abstract numbers (4.2x, 4.6x, 1%, 7%) come from this exploratory table.
  - The confirmatory prediction that mattered (P1, end to end) failed [MS p.5].
  - The device-level framing was adopted after inspecting archive data.
  - The GPU-active clock median was added as a deviation "before inspecting any cross-host comparison" [MS p.11]. That is an author-attested ordering with no independent timestamp.
- **"Lower median of three per-pass percentiles"** [MS p.3]. Undefined in the PDF: which percentile is used for device time (P50?), and is "lower median" of three values just the middle value (for n=3 the lower median equals the median)? The phrase is either redundant or describes something else. It needs a formula.
- **Medians of bimodal distributions.** The author acknowledges: "The per-process values look bimodal (Figure 1a), so a median of three to five processes mostly reports which mode most of them landed in" [MS p.4]. Yet Table 1 medians drive the Section 8 "1.6×, 2.0×, 4.7×, 4.8×" advantage claims [MS p.6].
  - Given admitted bimodality, the Section 8 advantage numbers mostly reflect how many of 3-5 processes landed in the fast mode.
  - Mode-membership counts (k of n processes in slow mode) or mixture estimates would be the right summary.
- **Telemetry median while active (utilization > 0) and 50 ms sampling.**
  - nvidia-smi defines utilization.gpu as "Percent of time over the past sample period during which one or more kernels was executing on the GPU", with a sample period that "may be between 1 second and 1/6 second depending on the product" — [NVIDIA nvidia-smi documentation](https://docs.nvidia.com/deploy/nvidia-smi/index.html).
  - At the 50 ms polling interval, utilization > 0 selects samples whose trailing 1/6-1 s window contained any kernel. The resulting "active" clock population includes idle gaps and warm-up/data-load periods; the Limitations say clocks are "for active samples of a whole process, including warm-up" [MS p.8].
  - This is a selection effect that likely biases the light path's "active" clock in an unknown direction (inference).
  - Yang et al. (cited by the paper) report nvidia-smi samples only part of the time [MS p.3, ref. Yang 2023].
- **Table 3 "relative to best observed".**
  - The author argues a shared reference across arms prevents regression to the mean [MS p.4]. That is correct for the comparison between arms.
  - But the reference is a minimum over all runs of both arms. Its value depends on the number of runs, and a more variable arm contributes the extreme low values.
  - On the laptop the heater arm has a median of 1.07 and the off arm 1.59, so the best is probably set by a heater-on run (inference; per-run data not in PDF). That makes the heater-off ratios look worse than if each arm used its own best.
  - The ">1.5×" threshold is arbitrary and was not one of the registered predictions; the registered predictions were P1-P4 [MS p.11].
  - With pods 7/25 → 4/25 and H100 1/9 → 0/9, the "share" contrast is not statistically distinguishable (Fisher p = 0.50 and 1.0, descriptive) [Script].
- **Exclusions and replacements.**
  - One pod (14x slower CPU) was replaced "before any of its results were inspected" [MS p.3, p.11].
  - Five E-SPLIT runs were excluded "under the registered 5% rule", leaving n = 1 at LH light E=1,000 [MS p.7, p.11].
  - The exclusions are documented, which is good. But "before inspected" rests on the author's assertion.

### Registration terminology
- The paper calls E-LOCK and E-SPLIT "pre-registered", and D1/D1R/H100 "planned, not pre-registered" [MS p.11]. Its basis: E-LOCK was "committed and hashed on 2026-10-19, 12 minutes before its first run"; E-SPLIT was "committed and pushed with its analysis script on 2026-10-25 before its first run" [MS p.11].
- Nosek et al.: the commitment "is usually accomplished by posting the analysis plan to an independent registry such as https://clinicaltrials.gov/ or https://osf.io/. The registry preserves the preregistration and makes it discoverable" — [Nosek et al. 2018, PNAS 115(11):2600-2606, doi:10.1073/pnas.1708274114](https://www.pnas.org/doi/10.1073/pnas.1708274114).
- OSF: "Preregistration is the practice of posting a time-stamped, read-only version of your study plan to a public repository before beginning data collection or analysis" — [OSF Support](https://help.osf.io/article/330-welcome-to-registrations).
- Haroz (MetaArXiv) treats "a timestamp, a registry, and persistence" as the bare minimum and finds that "GitHub and AsPredicted fail to meet all three" — [Haroz 2022, Comparison of Preregistration Platforms](https://osf.io/preprints/metaarxiv/zry2u).
- Git commit dates are author-controllable: git's `--date` option will "Override the author date used in the commit", and `GIT_AUTHOR_DATE` / `GIT_COMMITTER_DATE` environment variables set the dates — [git-commit documentation](https://git-scm.com/docs/git-commit). A local commit (E-LOCK: "committed and hashed", no mention of a push before the run) therefore has **no independently verifiable timestamp**.
- E-SPLIT was pushed to GitHub before its first run. The GitHub server-side push/receive record (Events API, reflog) is independent of the commit metadata, but a force-push could rewrite history (inference). I found no primary NVIDIA/GitHub doc in this session proving the push-event retention policy; listed as a gap.
- E-SPLIT's push was "2026-10-25 before its first run". The PDF's CreationDate is also 2026-10-25 09:41 [pdfinfo]. So E-SPLIT's 120 processes, analysis and manuscript integration all happened on the same day.

### Inferences
- Recommended wording:
  - E-SPLIT: "protocol and analysis script committed and pushed to the public repository (commit <SHA>, GitHub push time <UTC>) before data collection".
  - E-LOCK: "protocol committed locally (commit <SHA>, hash <SHA-256>) 12 min before the first run; the commit was not independently timestamped".
  - Reserve "pre-registered" for future work posted to OSF/AsPredicted/Zenodo, or at least OpenTimestamps / an RFC 3161 timestamp of the protocol hash.
  - For D1/D1R/H100, "planned" is itself generous. "Protocol written before data collection (unverified)" is accurate.
- Recommended statistics:
  - Show every per-block value (dot plots; there are only 3-5).
  - Replace percentile-bootstrap CIs with exact sign/permutation p-values or plain [min, max] labelled as such.
  - Report mode membership (k/n in slow mode) for bimodal cells.
  - Use a hierarchical (random-effects) model when more hosts exist.
  - Pre-specify a single primary outcome per experiment.
  - Increase blocks to at least 10 per machine for the primary claims (n=10 makes p ≈ 0.002 attainable for a perfectly consistent sign).
- The single strongest statistic in the paper is E-LOCK's 5/5 blocks with max/min = 1.00 at boost vs 1.51-2.84 unlocked, on the same laptop. Even this rests on n = 5 processes, one device.

### Gaps
- Whether any per-block data show the heater-on arm setting the Table 3 reference for every GPU/E needs the raw data.
- The exact percentile used in the "per-pass percentiles" is not stated in the PDF.

---

## Q4. Causal identification: heater and clock lock (Phase 2C)

### Takeaway
- **E-LOCK (and E-SPLIT) on the laptop is a genuine intervention.** It shows that the device time of the light kernel *at fixed clocks* is stable, and that the SM clock rather than the memory clock sets its level. It does **not** show that the driver's clock *choice* is caused by low utilization, nor that the paper's (rather than some other) mechanism moves the clock in the unlocked case.
- **The heater is a confounded multi-channel perturbation.** It loads host CPU, the GPU queue, the driver/scheduler and the power state at once, so it cannot isolate power state.
- **Everything on the cloud hosts is observational or confounded.**
- The paper's own data are consistent with a simpler busy-fraction mechanism that the paper dismisses: at higher E, host-side materialization dominates the call, so the GPU idles more and is clocked lower.

### Causal diagram (text / mermaid)
```mermaid
graph LR
  E[E eligible rows] --> HostWork[host materialize+sort+upload 4E bytes]
  E --> DevWork[device work ~E (light)]
  Path[light vs heavy path] --> DevWork
  HostWork --> IdleGap[GPU idle gap between kernels / queries]
  DevWork --> BusyFrac[GPU busy fraction]
  IdleGap --> BusyFrac
  BusyFrac --> Gov[driver DVFS governor (undocumented)]
  OSPolicy[Windows power plan / NVIDIA power mgmt mode / Optimus / WDDM] --> Gov
  Thermal[thermal & power limit] --> Gov
  Gov --> SM[SM clock] --> KT[kernel device time]
  Gov --> MEM[memory clock] --> KT
  Heater[heater process] --> BusyFrac
  Heater --> HostCPU[host CPU contention] --> HostWork
  Heater --> Queue[GPU queue/SM contention] --> KT
  Heater --> Submit[submission/readback contention] --> Call
  Lock[nvidia-smi -lgc/-lmc] -. sets .-> SM
  Lock -. sets .-> MEM
  Lock -. may also change .-> Gov
  KT --> Call[end-to-end latency]
  HostWork --> Call
  Proc[fresh-process / init state] --> Gov
  Proc --> KT
  Host[cloud host identity / driver branch] --> Gov
  Host --> HostWork
```

### Cited Findings
- **Clock-lock semantics.** NVIDIA documents `--lock-gpu-clocks (-lgc)` as specifying "<minGpuClock,maxGpuClock> clocks as a pair (e.g. 1500,1500)", "Supported on Volta+. Requires root". `--lock-memory-clocks (-lmc)` likewise "Requires root" and "does not work on GPUs based on NVIDIA Hopper architectures" — [nvidia-smi docs](https://docs.nvidia.com/deploy/nvidia-smi/index.html). The paper locks both on the laptop [MS p.5-6].
  - The docs do not say whether a min = max lock also changes P-state transitions, idle entry or the latency of waking from idle.
  - The paper's "observed SM clock matched the lock in every locked run" [MS p.5] uses 50 ms nvidia-smi samples. It cannot exclude sub-50 ms dips between queries.
  - The E-SPLIT exclusion rule (memory held at 7,001 instead of 8,001 MHz in 5 runs) shows the lock is not always honoured [MS p.11].
- **P-states.** NVIDIA: "States range from P0 (maximum performance) to P12 (minimum performance)" — [nvidia-smi docs](https://docs.nvidia.com/deploy/nvidia-smi/index.html). The paper cites P5 for the laptop light path at E=10,000 [MS p.4]. nvidia-smi exposes "Clocks Event Reasons", which "retrieves information about factors that are reducing the frequency of clocks" (e.g. Idle, power cap, thermal) — [same](https://docs.nvidia.com/deploy/nvidia-smi/index.html). The paper does not report clock event (throttle) reasons. These would directly separate "idle-driven" from "power/thermal-driven" down-clocking.
- **Windows driver power policy.** NVIDIA Control Panel "Power management mode" lets the user set "how your graphics card's performance level changes"; "Adaptive: Save power by letting the graphics driver reduce GPU performance depending on the needs" — [NVIDIA Control Panel help](https://www.nvidia.com/content/Control-Panel-Help/vLatest/en-us/mergedProjects/nv3d/Manage_3D_Settings_(reference).htm). NVIDIA explains that when the GPU "is not saturated with work there is an opportunity to save power by reducing GPU clocks", and that "Prefer Maximum Performance mode overrides the power savings features in the GPU and allows the GPU to always run at higher clocks" — [NVIDIA Reflex article](https://www.nvidia.com/en-us/geforce/news/reflex-low-latency-platform/).
  - The manuscript reports the Windows "Balanced" plan [MS p.3] but **not the NVIDIA Power management mode**. That is a documented, user-settable driver policy that directly targets the phenomenon.
  - A no-admin intervention ("Prefer maximum performance") exists, and testing it would strengthen external validity.
- **Timestamp semantics.**
  - Vulkan: the number of nanoseconds per timestamp increment is `timestampPeriod`. Timestamps "must be in the VK_TIME_DOMAIN_DEVICE_KHR time domain". "Implementations may write the timestamp at any stage that is logically later than stage" — [Vulkan spec, Queries chapter](https://docs.vulkan.org/spec/latest/chapters/queries.html).
  - WebGPU: timestamp values are written "in nanoseconds, but how the value is determined is implementation-defined"; "Timestamp values are implementation-defined"; the precision "must be reduced" (coarsened) for privacy; and "the physical device may reset the timestamp counter occasionally, which can result in unexpected values such as negative deltas" — [WebGPU spec §20.4](https://gpuweb.github.io/gpuweb/).
  - Implication (inference): device timestamps measure wall time in the device time domain, so a lower SM clock lengthens them, which is the intended effect.
  - Two caveats. (a) Pass-level timestamps include wake-up / ramp latency at the start of a pass if the GPU was idle, so "kernel time" at low clocks may include P-state exit latency, not just slower execution. (b) Native wgpu may not apply browser-level coarsening, but the paper does not state the timestamp resolution or `timestampPeriod`. At 0.024-0.03 ms per kernel this matters.
- **Heater confounds, from the paper itself.**
  - "It also uses host CPU, it slowed host-side work by 40-48% on the H100 pod" [MS p.1, p.4].
  - The heater "also competes for GPU submission and readback" [MS p.4].
  - It "keeps the GPU lightly busy" with "64 workgroups × 256 threads, 500 dependent FMAs each, waits for it, and sleeps 1 ms" [MS p.3].
  - The paper reasons: "On the GPU the heater can only compete with the measured process, so a speed-up it produces suggests that the measured process was held back by its power state" [MS p.3]. This is weaker than stated. A co-running process can also speed up a measured kernel by:
    1. preventing deep idle/low-power state entry and its exit latency, even at the same steady-state clock;
    2. keeping driver/queue structures warm (keep-alive; e.g. persistence-like effects; on Linux, NVIDIA documents that "the driver unloads when no active clients exist" without persistence mode — [nvidia-smi docs](https://docs.nvidia.com/deploy/nvidia-smi/index.html));
    3. raising memory clock or PCIe link state;
    4. on Windows/WDDM, changing scheduler timeslice behaviour.
  - All four are "power-state-like" but not "SM clock". The paper's binary dichotomy ("contention only" vs "power state") omits them.
- **Heater on the pods: telemetry does not support a clock mechanism.** Light-path SM medians are 2,235-2,520 MHz without the heater and 2,520-2,550 with it [MS p.5]. A 1-11% clock change cannot explain the heater's 0.82-0.90 device ratios at E ≤ 10,000, nor the 4.6x process spread (N25). The pods' evidence for "power state" is therefore indirect at best.
- **E-SPLIT interaction.** Light scoring at E=10,000 has m + s = 1.35 [MS p.7; recomputed 1.349]: lowering either clock alone recovers more than the joint log-slowdown. That means a sub-additive interaction (the two slowdowns overlap rather than multiply). The paper notes "the two clocks do not act independently" [MS p.7]. That is correct, but it also means a single "memory share" m is not a well-defined decomposition for that kernel.

### Experiments: causal vs observational
| Experiment | Type | What it identifies | What it does not |
|---|---|---|---|
| E-LOCK (laptop, 150 proc) | Randomized-order intervention on clocks (within-block shuffle) | At fixed SM+mem clocks, light kernel variance across processes ≈ 0; a low lock reproduces a slow level (K3) | Why the driver picks low clocks; whether unlocked slow mode = the same state as the lock (Llo/U = 0.78 at E=10k, N7); end-to-end effects; any other machine |
| E-SPLIT (laptop, 120 proc) | Factorial intervention | Selection kernel slowdown is SM-clock driven (m ≤ 0.07) | Same external-validity limits; n=1 cell |
| Heater (D1, D1R, H100) | Intervention, but multi-channel (host CPU, GPU queue, idle-state, clocks) | "Something about added load" reduces light-kernel slow runs on the laptop; weak and statistically indistinguishable on pods/H100 (Table 3) | That the channel is the SM clock (pods telemetry contradicts it); end-to-end benefit (P1 failed) |
| Table 1 cross-host | Observational, partly exploratory | Light kernel time differs across processes/hosts | Host vs process separation (confounded); driver-branch effect |
| Spearman ρ vs clock at process end (host A) | Observational correlation | Association between end-of-process clock and selection time | Direction (slow kernels → lower utilization → lower clock is endogenous, as the paper admits [MS p.4]) |
| Table 2 telemetry | Observational, pooled, includes warm-up | Light path's median sampled clock is lower than heavy's | Clock at kernel execution time |

### Alternative explanations to rule out (with the paper's coverage)
1. **Host-side domination of call time (busy-fraction mechanism).** At locked boost the light path's kernels occupy only 13.5% → 10.4% → 8.7% of the laptop call as E rises 1k → 3k → 10k (recomputed from Tables 4, 5, 8; N16). The exogenous duty cycle *falls* with E because host materialization and the 4E-byte upload grow faster than device work. A utilization-threshold governor would then lower the clock as E grows, which is exactly what Table 2 shows on the laptop and H100. The paper says "A simple busy-fraction account does not explain this" [MS p.4], using the unlocked (endogenous) busy fraction (13-43%). The exogenous, locked-clock busy fraction supports the busy-fraction account. **Test:** vary inter-query host delay (sleep or synthetic host work) at fixed E and see whether the clock and kernel time follow the GPU duty cycle.
2. **Windows/WDDM + Optimus + Vulkan.** The laptop's display is on the iGPU [MS p.3]. The dGPU may enter deeper idle states between batch-1 queries than on Linux servers. Not tested; Linux on the same laptop, or a desktop 4050/4060, would separate OS from device.
3. **NVIDIA Power management mode** (Adaptive / Optimal / Prefer maximum performance) on Windows was not reported or varied; see the citation above.
4. **Idle-exit latency vs steady clock.** A kernel launched after an idle gap pays P-state ramp time. The timestamp of the first dispatch in a call may include it (Vulkan allows the write at any logically later stage). Not separable with the reported data; would need back-to-back dispatches within a call, or timestamps around an empty dispatch.
5. **Thermal state.** The laptop "never exceeded 50 °C" in E-LOCK [MS p.5]. No temperatures are reported for D1 or the pods. Clock event reasons are not reported.
6. **Cloud co-tenancy / MIG / container.** Pods on 5 hosts with two driver branches (570.195 and 580.126-580.173) [MS p.3]. Driver branch is fully confounded with host; no co-tenant GPU telemetry is reported.
7. **Fresh-process initialization state.** Bimodality across processes but stability within a process suggests a per-process latent state set at startup: memory placement, pipeline compilation path, queue family selection, or a P-state ladder reached during data loading. Figure 1b shows the laptop light process dropping to about 210 MHz during loading before settling [MS p.2]. The paper attributes the per-process mode to the clock, but on pods the clock telemetry does not show a large difference (N25). Per-process kernel time should be cross-tabulated against per-process clock (the paper does this only for host A's 10 blocks: ρ = −0.83).
8. **Locked-clock semantics changing idle behaviour.** If `-lgc min=max` also prevents idle down-clocking, E-LOCK removes both the steady-state clock and idle-exit effects. E-LOCK cannot distinguish them, and "the SM clock accounts for most of it" should be read as "SM clock locking accounts for most of it".

### Inferences
- **Defensible causal claim:** "On this laptop, fixing the SM clock (with nvidia-smi -lgc) removes the light kernel's process-to-process variation; the driver's default clock management is therefore necessary for that variation."
- **Not identified:** that low GPU utilization (rather than host-side idle gaps, OS power policy or process init state) drives the clock choice; that the same mechanism operates on the pods/H100; and that it matters for end-to-end latency.
- The "heater can only compete, so speed-up implies power state" logic should be restated as "speed-up implies the measured process was not capacity-bound; power-state, idle-exit, and driver keep-alive effects remain candidate explanations".

### Gaps
- The NVIDIA documentation found does not specify whether `-lgc` with min = max disables idle P-state entry. This could not be verified from primary sources in this session.
- Whether wgpu's Vulkan backend applies timestamp quantization natively could not be verified here (the WebGPU spec mandates coarsening in browsers; native wgpu behaviour needs a code/doc check by the repository researcher).

---

## Q5. Logical consistency (Phase 2F)

### Takeaway
The paper is more candid than most about failed predictions (P1, K2) and about the heater's crudeness. The remaining problems are generality leaps in Section 8 and the Conclusion, abstract-level scoping, a self-undermining Appendix C argument, an empty Appendix B, and "reporting rules" presented as a contribution although they restate standard practice.

### Cited Findings
- **Failed-prediction framing.**
  - P1 is reported as failed in the abstract, Section 6 and the Conclusion [MS p.1, p.5, p.8]. This is good practice.
  - K2's failure is reframed: "After the fact, we note that the lock beat every unlocked configuration ... That suggests the driver's best unlocked state for light work is still below boost" [MS p.6]. This is honest. But the failure direction (locked high is 32-50% *faster* than the best unlocked) means unlocked runs never reach boost-equivalent performance for the light path. That is evidence for a level effect, not just a variance effect, and it weakens the Section 4 framing that only "which mode most of them landed in" differs.
  - The abstract describes E-LOCK's protocol as "committed before its data" and K-predictions as held, without mentioning K2's failure [MS p.1]. The introduction mentions it [MS p.1].
- **Generality leaps.**
  - The Section 8 heading "Unlocked-clock benchmarks can understate the efficient implementation" generalizes from one kernel pair where the comparator is "deliberately naive" and "far from the best available; a tuned top-k would narrow the gap" [MS p.3].
  - "Optimizations that remove GPU work (tighter filtering, smaller kernels, early exit) can be measured at a discount" [MS p.6] is not tested for any optimization.
  - "Isolated benchmarks may misstate co-located latency" [MS p.6-7]: "We did not test co-location with a real model" [MS p.7].
  - Conclusion: "A small engine with this much instrumentation should find similar effects in other kernels for little money" [MS p.8] is a prediction without evidence.
- **Claims stronger than n allows.**
  - "Slow runs become rarer on every GPU" (Fig. 1 caption) [MS p.2] rests on pods 7/25 → 4/25 and H100 1/9 → 0/9 (N12).
  - "The planned interaction appears on every GPU" [MS p.4] uses device-level P3, including a tied pods E=30,000 cell (N13).
- **Abstract vs results.**
  - "including at E = 10,000, where the light path is faster than our CPU scan" attaches a 4.2x figure from E=3,000 (N2).
  - The same clause is unsupported for the H100 (N14).
  - "Telemetry shows the driver running the light path at low clocks" [MS p.1]: on the pods, light-path median SM clocks are 79-89% of boost (Table 2), so "low" holds for the laptop and H100 but not for the 4090 pods.
- **Ambiguous baseline.** "On the laptop, the light kernel at fixed boost ran 2.9-5.0× faster than with default clocks" [MS p.6] uses E-LOCK's unlocked arm, which is 1.2-2.3x slower than D1's unlocked arm (N4).
- **Appendix C self-contradiction.** The text says the maximum-regret rejection "survives" because 3.36x exceeds all within-host spreads "except the laptop light path's 4.2×" [MS p.12]. But the held-out suite ran on that laptop [MS p.11]; see N18.
- **Orphaned content / cross-references.**
  - **Appendix B "CPU/GPU crossover" is a heading with no body**; Appendix C begins immediately below it (verified visually on page 11 render) [MS p.11].
  - Table 8 is referenced as "Appendix Table 8" [MS p.1, p.6] but sits on page 13 after Appendix D, with no section of its own; Appendix B was presumably meant to hold it.
  - Tables 6 and 7 are referenced as "Appendix Table 6/7" [MS p.4, p.5] and float on page 12 inside Appendix C's text flow.
  - Other cross-references resolve: Sections 3-8, Figure 1c, Appendix A, C, Tables 1-5. The repository paths cited in the Reproducibility section are split across the column with a stray space: "research/data/raw/2026-09-25-dvfs- *" and "research/scripts/analyze_dvfs_d1. py" [MS p.8], a typesetting defect.
  - Section 7 cites the heavy low-lock (3.6 ms) and K5 values that are not in Table 4 (N8).
- **Reporting rules novelty.**
  - Rules (i)-(v) [MS p.7] amount to: replicate fresh processes, log clocks continuously, report device and end-to-end, lock clocks, state power state.
  - These overlap with Kalibera & Jones (multi-level repetition) and Hoefler & Belli (report variation and CIs, document the setup) — [K&J](https://dl.acm.org/doi/10.1145/2464157.2464160); [Hoefler & Belli](https://htor.inf.ethz.ch/publications/img/hoefler-scientific-benchmarking.pdf). The paper itself cites clock-locking as established kernel-benchmarking practice [MS p.1, p.7].
  - Rule (iv)'s heater-plus-negative-control is the novel element, and it is the least validated (it failed end to end and confounds host load).
- **Novelty claim.** "We have not found a report that this changes the measured margin between two implementations ... or that an unrelated co-running GPU load speeds up a latency-bound GPU kernel" [MS p.7]. This is a negative-search claim with no documented search method. (The literature researcher should assess it.)
- **"Identical results" scope.** Section 2: "Every execution path below returns the same result set (all runs pass an FP64-oracle recall@10 gate of 0.99; observed recall is 1.0 except where noted)" [MS p.2]. A 0.99 gate permits non-identical results, and no exceptions are actually "noted" anywhere in the PDF (searched the text). "Identical top-k results" in the abstract [MS p.1] is stronger than a 0.99 recall gate.

### Inferences
- Defensible rewrite of the Section 8 heading: "On this laptop, unlocked-clock measurements understated the compact-row kernel by 1.3-5.0× depending on E and on which unlocked run is the reference; we did not test whether this holds for tuned kernels or other GPUs."
- The abstract should give the E=10,000 numbers (1.4x laptop, 4.6x pods) for the regime it describes, drop the H100 from the CPU-faster clause, and mention K2's failure.

### Gaps
- Whether Appendix B's content exists in the repository LaTeX (paper/v2) is for the repository researcher.

---

## Q6. Claim-evidence matrix (Phase 1)

Confidence = my confidence that the claim *as worded* is supported by the evidence in the PDF (High / Med / Low). Verification labels: **R** = recomputed from manuscript tables [Script]; **V** = visually checked in rendered PDF; **U** = unverifiable from PDF (needs raw data); **L** = logical/methodological assessment. All locations are [MS p.N].

| ID | Exact claim (abridged quote) | Location | Evidence provided | Independent verification | Confidence | Required action |
|---|---|---|---|---|---|---|
| A1 | "two GPU paths ... that return identical top-k results" | Abstract p.1; §2 p.2 | FP64-oracle recall@10 gate 0.99; "observed recall is 1.0 except where noted" | L: gate permits non-identity; no exceptions noted anywhere in PDF | Med | State observed recall per cell; say "identical in all runs" only if true; drop "except where noted" or note the exceptions |
| A2 | Heavy device time "varied by at most 1% between processes on one machine and 7% across the five pods" | Abstract p.1; Intro; C1 p.2; §4 | Table 1 | R: max 1.005 within machine, 1.074 pods | High | None (precise) |
| A3 | Light "varied up to 4.2× on the laptop and 4.6× across the pods, including at E = 10,000, where the light path is faster than our CPU scan" | Abstract p.1 | Table 1 | R: 4.2x is E=3,000 (CPU wins there); at E=10k laptop = 1.40x | Med (true but misleadingly scoped) | Give E-specific values; separate CPU-winning and GPU-winning regimes |
| A4 | Light faster than CPU at E=10,000 (implied for all GPUs listed) | Abstract p.1; Fig.1 caption p.2 | Table 8 (laptop + pods only) | R: true laptop, pods; H100 and host A absent | Low for H100/host A | Add H100 row to Table 8 or restrict claim |
| A5 | "Telemetry shows the driver running the light path at low clocks" | Abstract p.1; §5 | Table 2, Fig.1b | R/V: true laptop (585 MHz) and H100 (645 MHz at E=10k); pods 2,235-2,520 MHz (79-89% boost) | Med | Qualify per GPU; report clock event reasons |
| A6 | "locking the clocks removed the variation (2.8× to 1.00× across blocks)" | Abstract p.1; §7 | Table 4, n=5 blocks | R: 2.84 → 1.00 at E=1k; but D1's unlocked E=1k spread was 1.00 | Med-High (for this laptop) | Note D1 vs E-LOCK baseline drift; show per-block values |
| A7 | "a low lock reproduced the slow mode" | Abstract p.1; §7 K3 | Llo 0.444 vs D1 0.461 | R: 3.7% vs D1; 21.7% *faster* than E-LOCK's own U (0.567) | Med | Report both references; acknowledge Llo/U = 0.78 at E=10k |
| A8 | "a second registered experiment ... traced the slow mode mainly to the SM clock" | Abstract p.1; §7; Table 5 | E-SPLIT, 120 proc | R: selection m ≤ 0.07 reproduces; light scoring m = 0.48, m+s = 1.35 | High for selection kernel on this laptop | Say "selection kernel"; flag the n=1 LH cell |
| A9 | "Clocks could not be locked on the cloud hosts" | Abstract p.1; §3 | Statement | U (plausible: -lgc "Requires root" — [nvidia-smi docs](https://docs.nvidia.com/deploy/nvidia-smi/index.html)) | High | None |
| A10 | Heater "made slow light-kernel runs rarer on all three GPU types" | Abstract p.1; Intro; §6; Fig.1c | Table 3 | R: laptop 13/25 → 0/25 (strong); pods 7/25 → 4/25 (Fisher p = 0.50); H100 1/9 → 0/9 (p = 1.0) | Low for pods/H100 | Say "on the laptop; directionally on pods/H100 (3 and 1 runs)" |
| A11 | "its planned end-to-end prediction failed" | Abstract p.1; §6 scorecard | Table 6/7 | R: P1 "no" in all 8 testable cells; consistent with Table 7 | High | None (good practice) |
| A12 | "about US$6 of cloud time; protocols, raw data and telemetry are released" | Abstract p.1; App. D | Ledger file path | U | Med | Provide hours per pod; repository researcher to verify |
| I1 | "There its kernel time varied 1.4× ... on the laptop and 4.6× across five RTX 4090 cloud pods" (E=10k) | Intro p.1 | Table 1 | R: 1.40, 4.61 | High | None (correctly scoped; contradicts abstract's emphasis) |
| I2 | "light path is faster than both the heavy path and our CPU scan on the laptop and the RTX 4090 pods" at E=10k, "a regime where an application would choose it" | Intro p.1 | Table 8 | R: true; but CPU per-row cost jumps 3.25-3.37x at the FP32→FP64 switch above E=4,096 | Med | Report CPU FP32-only baseline at E=10k; discuss the switch's role |
| I3 | "locked at boost, the light kernel took 32-50% less time than the best unlocked median, outside the 15%" | Intro p.1; §7; App. A | Not tabulated (U+heater not shown) | R: 32-50% ⇔ 0.50-0.68 consistent; reconstruction ≈ 0.50-0.69 | Med | Tabulate U+heater and Lhi+heater cells |
| I4 | Heater: share >1.5× slow "fell from 52% to 0% on the laptop, 28% to 16% on the RTX 4090 pods and 11% to 0% on the H100" | Intro p.1 | Table 3 | R: 13/25, 7/25 → 4/25, 1/9 → 0/9 | High (arithmetic); Low (inference) | Give counts not just %; add exact tests |
| I5 | "the heavy kernel never got faster" | Intro; Fig.1c | Table 3/7 | R: heavy device ratios ≥ 1.00 in all cells | High | None |
| I6 | Heater "slowed host-side work by 40-48% on the H100 pod" | Intro; §6; App. A | Statement | U | Med | Table of host materialization ratios |
| I7 | "Its protocols were written before the data but committed only afterwards" | Intro; App. A | Author attestation | L: no independent timestamp | n/a | Keep (honest); avoid "planned" implying confirmation |
| I8 | "finding the effect, testing it ... and releasing every sample took days" | §1 p.2 | Statement | U | n/a | Optional |
| C1 | "Qenlo's work-efficient GPU path is the less reproducible one ... up to 4.2× ... on one machine" | Contributions p.2 | Table 1 | R: 4.2x at E=3k only | Med | Scope by E |
| C2 | "locking the clocks removed the light kernel's variation ... SM clock accounts for most of it; the memory clock matters only for the scoring kernel at the largest E" | Contributions p.2 | Table 4, 5 | R: consistent | High (laptop) | Add "on one laptop" |
| C3 | "Telemetry and the heater are consistent with the same account on RTX 4090 and H100 hosts" | Contributions p.2 | Tables 2, 3, 7 | R: pods clocks barely move; H100 645 → 1,980 MHz gives only 0.81 device ratio (predicted 0.33) | Low-Med | Report the quantitative inconsistency (N25) |
| C4 | "We give rules for benchmarking latency-bound GPU work" | Contributions p.2; §8 | List of 5 rules | L: largely standard (K&J 2013; Hoefler & Belli 2015) | Low (as a contribution) | Present as recommendations, cite prior rules |
| S2a | Heavy path "device work is proportional to N regardless of E" | §2 p.3 | Design argument | R: heavy scoring grows 1.56x from E=1k → 10k (Table 5) | Med | Qualify: selection work |
| S2b | "up to N/E times more device work ... (1,000× at E=100, N=10^5)" | §2 p.3 | Arithmetic | R: 10^5/100 = 1,000 | High (as work count) | None |
| S3a | Blocks are independent replicates; 15,000 calls never treated as independent | §3 p.3 | Design | L: correct practice (K&J) | High | None |
| S3b | 95% percentile-bootstrap intervals over blocks, read descriptively | §3 p.3 | Table 7 | R: with n = 3/5 these equal [min, max]; coverage 75%/≤94% | n/a | Replace with raw values + exact tests |
| S3c | "Because each pod ran one block, host and process variance are confounded on the pods" | §3 p.3 | Design | L: correct | High | Run ≥ 3 blocks per pod |
| S4a | Light spread "reaches 2.6× on 4090 host A, 4.2× on the laptop and 4.3-4.6× across the five 4090 pods" | §4 p.4 | Table 1 | R: 2.56; 4.16; pods 4.30/4.61 but 2.08 at E=1k | Med | "2.1-4.6×" for pods |
| S4b | "Across ten identical blocks, the light path's selection time spans 2.44× and correlates with the GPU clock recorded at process end (Spearman ρ = −0.83, p = 0.003)" | §4 p.4 | Archive | R: t-approx p = 0.0030; exact two-sided 0.0047; host A has 5 blocks per Table 1 caption | Med | Reconcile 5 vs 10 blocks; state E; report exact p; note endogeneity |
| S4c | "Heavy kernels in the same archive vary by at most 0.1% wherever their clock was at boost" | §4 p.4 | Archive | U | Med | Show data; "wherever at boost" is conditioning on the outcome's cause |
| S5a | Heavy holds boost SM "2,670-2,820 MHz on the consumer GPUs and 1,980 MHz on the H100" | §5 p.4 | Table 2 | R: matches | High | None |
| S5b | At E=10k memory cut to 810 MHz on laptop (from 8,201; P5) and pods (from 10,501) | §5 p.4 | Table 2 | R: matches; but heavy laptop E=3k memory is 7,001 MHz (unremarked) | High | Note heavy memory variation |
| S5c | "Light path spends 13-43% of each call inside kernels"; heavy "52-84%" | §5 p.4 | Not tabulated | R: 13% end reproduces from Tables 1/8; 43% and 52-84% unverifiable | Med | Tabulate busy fractions |
| S5d | "A simple busy-fraction account does not explain this" (clock falls as E grows) | §5 p.4 | Argument from endogenous busy fraction | L/R: locked-boost (exogenous) busy fraction falls 13.5% → 8.7% with E: supports busy-fraction account | Low | Retract or test with inter-query delay manipulation |
| S6a | "Without the heater, the light kernel runs a median 1.15-1.59× slower than its best, with a tail reaching 3.3×" | §6 p.4 | Table 3 | R: medians match; 3.3 is p90, max ≈ 4.8 in Fig.1c | Med | Say "p90 3.3×, max ≈ 4.8×" |
| S6b | Heavy becomes "uniformly slower on the laptop (1.19×)" with heater | §6 p.4 | Table 3/7 | R: laptop E=30k heavy device 1.00 [1.00, 1.00] | Med | Drop "uniformly" or exclude E=30k explicitly |
| S6c | Light call with heater 0.94-0.96 on pods at E=3k-10k (1.07 at 30k), 0.95-0.97 laptop E≥3k; H100 1.10-1.24; heavy 6-9% / 25-27% slower | §6 p.4-5 | Table 7 | R: all match | High | None |
| S6d | "light path fared better than the heavy path in 10 of 13 cells" | §6 p.5 | Table 6/7 | R: 10/13 | High | None |
| S6e | "The 1.9× end-to-end speed-up in our laptop pilot did not replicate" | §6 p.5; App. A | Pilot | U | High (honest) | None |
| S6f | Scorecard P1 failed everywhere; P2 every cell; P3 every cell device / 10 of 13 call; P4 6 of 13 | §6 p.5 | Table 6 | R: counts match; pods E=30k P3-device is a tie (1.00 vs 1.00) scored "yes" | Med-High | Mark ties; define tie rule |
| S7a | E-LOCK: 6 cells × 5 conditions × 5 blocks = 150 processes | §7 p.5 | Design | R: 6·5·5 = 150 (the "6 cells" are presumably 3 E × 2 paths, though the heavy path under Llo/U is not tabulated) | Med | Tabulate all 6 cells × 5 conditions |
| S7b | Unlocked varied "2.84×, 2.11× and 1.51×"; locked 1.00× (K1 ≤ 1.15) | §7 p.5 | Table 4 | R: match | High | None |
| S7c | Heater cut unlocked light to "0.57, 0.37 and 0.40" of heater-off; locked: ratio 1.00 (K4) | §7 p.5 | Not tabulated | U; conflicts with D1 laptop Table 7 (1.00 / 0.65 / 0.46) | Med | Tabulate; discuss inconsistency with D1 |
| S7d | Heavy Lhi/U = 1.000, 1.000, 1.002 (K5) | §7 p.5 | Not tabulated | U | Med | Tabulate heavy U |
| S7e | "observed SM clock matched the lock in every locked run, and the GPU never exceeded 50 °C" | §7 p.5 | Telemetry | U; 50 ms sampling cannot see sub-sample dips | Med | Report min sampled clock per run, clock event reasons |
| S7f | "Locked at 585/810 MHz, the light kernel at E = 10,000 took 0.444 ms, within 4% of the 0.461 ms that D1 measured" (K3) | §7 p.5 | Table 4 vs Table 1 | R: 3.7% | High (arithmetic) | Add same-experiment comparison (Llo/U = 0.78) |
| S7g | "The same lock slowed the heavy kernel by 3.8× (0.95 to 3.6 ms)" | §7 p.5-6 | Not in Table 4 | R: from Table 5, 3.615/0.947 = 3.82 | Med | Put heavy Llo in Table 4 |
| S7h | "The driver put the light path there and kept the heavy path out of it" | §7 p.6 | Table 2 + Table 4 | L: the driver chose a clock near 585 MHz on the laptop in D1 (Table 2) | Med | OK for the laptop |
| S7i | "fixing the clocks ... sets its level, so the clock the driver chooses accounts for the variation we measured" | §7 p.6 | E-LOCK | L: supported for this laptop; "accounts for" means "locking eliminates" | Med-High | Add "on this laptop" |
| S7j | E-SPLIT "protocol and analysis script were committed and pushed before its data"; re-run corners "within 0.2%" | §7 p.6 | Statement; Tables 4/5 | R: corners match at printed precision except light E=3k (0.044 vs 0.045); U for push time | Med | Give commit SHA + GitHub push timestamp |
| S7k | Selection: memory alone "changed nothing at E ≤ 3,000 and slowed the light path by 10% at E = 10,000 (m ≤ 0.07 everywhere; S2, m < 0.3)" | §7 p.6 | Table 5 | R: 1.097; m = 0.068 | High | None |
| S7l | Scoring at E=10k: memory alone "doubled light-path scoring time (0.034 to 0.069 ms; m = 0.48 ...)" "raised heavy-path scoring by 44%"; "m + s = 1.35" | §7 p.6-7 | Table 5 | R: 2.03x; 1.439; m = 0.479; m + s = 1.349 | High | None |
| S7m | "A memory-bound explanation of the slow mode does not hold at these sizes" | §7 p.7 | Table 5 | L: true for selection on the laptop | Med-High | Add "on this laptop" |
| S8a | "the light kernel at fixed boost ran 2.9-5.0× faster than with default clocks, so the unlocked number understates it" | §8 p.6 | Table 4 | R: 2.88-5.02 vs E-LOCK U; 1.25-4.08 vs D1 U; about 1.45-2.0 vs best unlocked (U+heater) | Low-Med | Report all three; the headline uses the slowest baseline |
| S8b | Device-time advantage at E=10k "1.6× ... 2.0× ... 4.7× ... 4.8×" and locked "8.3×" | §8 p.6 | Table 1/4 | R: 1.60/2.04/4.74/4.77; 8.33 | High (arithmetic); Low (interpretation, given bimodal medians of 3-5) | Show per-process ratios |
| S8c | "Optimizations that remove GPU work ... can be measured at a discount the benchmark does not show, and the discount differs between machines" | §8 p.6 | None direct | L: untested extrapolation | Low | Mark as hypothesis |
| S8d | RTX 4090 light times at E=10k "ran from 0.128 to 0.589 ms across processes, a 4.6× span" | §8 p.6 | Table 1 | R: 0.589/0.128 = 4.60 (both endpoints are pods; host A sits inside) | High | Say "across the five pods" |
| S8e | Prior study's router "chose the slower route in 8 of 16 workloads, several of them within the process-to-process variation measured here" | §8 p.6; App. C | Prior archive | U; App. C says 2 of 8 within 3.2% regret | Med | Quantify "several" |
| S8f | "the heater never changed a CPU/GPU winner (Appendix Table 8)" | §8 p.6 | Table 8 | R: true in all 10 rows | High | Add H100 |
| S8g | "Latency measured on an idle GPU is therefore a poor proxy for latency under co-location" | §8 p.6-7 | Heater only | L: no real co-location tested (admitted) | Low | Rephrase as hypothesis |
| S8h | Rules (i)-(v) | §8 p.7 | — | L: mostly standard practice | n/a | Cite K&J, Hoefler & Belli |
| S9a | "We have not found a report that this changes the measured margin ... or that an unrelated co-running GPU load speeds up a latency-bound GPU kernel" | §9 p.7 | None | L: negative-search claim, search method undocumented | Low | Document the search or remove |
| S9b | "The variation we measure is much larger (up to 4.6× rather than tens of percent), and it makes the less efficient implementation look relatively better" | §9 p.7 | Table 1 | R: 4.6x pods; comparison to Sinha (under load, same-model devices) is apples-to-oranges | Med | Note the different regimes (idle-ish vs loaded) |
| S10 | Limitations list | §10 p.7-8 | — | L: candid; omits NVIDIA Power-management-mode setting, timestamp resolution, D1 vs E-LOCK drift, Appendix C self-contradiction | n/a | Add those |
| S11 | "the faster GPU path for a light query was also the one whose kernel time varied most ... and the variation tracked the clock the driver chose" | Conclusion p.8 | All | L: tracking shown on laptop; pods inconsistent (N25) | Med | Scope to laptop |
| S11b | "made the light kernel's slow runs rarer at the device level but did not make calls faster end to end" | Conclusion p.8 | Tables 3/7 | R: consistent | High | None |
| S11c | Heavy path "looks more stable than it would at equal clocks, and its gap to the light path looks smaller" | Conclusion p.8 | Table 4 | R: 8.3x locked vs 2.0x unlocked at E=10k on laptop | Med-High (laptop) | Scope to laptop |
| F1 | Fig. 1 title "The light kernel's speed tracks the GPU clock" | Fig.1 p.2 | Panels a-c | V: panel (b) is 1 laptop process per condition; pods clocks contradict | Med (laptop) / Low (general) | Retitle; add a per-process clock vs time scatter for all GPUs |
| F1a | Panel (a): "heavy path does 10× more work and reproduces within 1% on a machine and 7% across the five RTX 4090 pods; the light path varies up to 1.5× on a machine and 4.6× across the pods" | Fig.1 caption p.2 | Panel (a) | V: points consistent with Table 1 E=10k (laptop 4 visible light squares ≈ 0.43-0.60; pods 0.13-0.59; H100 ≈ 0.19-0.21); R: N/E = 10 | High | Note host A also has 5 points, not 10 |
| F1b | Panel (b): "The heavy path stays at boost and the light path drops to about 500 MHz; a separate light GPU load (the heater) raises it" | Fig.1 caption p.2 | 1 process per condition | V: heavy at about 210 MHz from about 15-25 s (data load) then 2,670; light settles about 380-570 MHz; light+heater about 1,000-1,230 MHz. Heater does **not** restore boost on the laptop | Med | Say "raises it to about 1,100 MHz, well below boost"; n=1 trace per condition |
| F1c | Panel (c): "With the heater, the light kernel's slow runs become rarer on every GPU and the heavy kernel never gets faster" | Fig.1 caption p.2 | Table 3 | V: pods filled squares still reach about 1.9x; R: pods/H100 differences of 3 and 1 runs | Low-Med (pods/H100) | Add counts |
| T1 | Table 1 | p.4 | — | R: all 24 max/min consistent | High | Add per-process dots; state E-specific scoping |
| T2 | Table 2 | p.5 | — | R: matches text; pooled over blocks with warm-up | Med | Report per-process clock at kernel time; clock event reasons |
| T3 | Table 3 | p.5 | — | R: percentages achievable | Med | Counts + per-arm reference sensitivity |
| T4 | Table 4 | p.6 | — | R: ratios reproduce; missing U+heater, Lhi+heater, heavy U/Llo | Med | Complete the table |
| T5 | Table 5 | p.7 | — | R: all m reproduce within rounding | High | Add s column and n per cell |
| T6 | Table 6 | p.12 | — | R: counts match text; pods E=30k tie | High | Mark ties |
| T7 | Table 7 | p.12 | — | R: consistent with text; bootstrap CIs = [min, max] | Med | Replace CIs |
| T8 | Table 8 | p.13 | — | R: winners correct; H100/host A missing; "median-block" undefined | Med | Define; add H100; add CPU FP32 baseline |
| AppA | E-LOCK "committed and hashed ... 12 minutes before its first run"; E-SPLIT "committed and pushed ... before its first run"; "only their predictions are called pre-registered" | App. A p.11 | Repository history | L: local git dates author-settable ([git docs](https://git-scm.com/docs/git-commit)); no third-party registry ([Nosek 2018](https://www.pnas.org/doi/10.1073/pnas.1708274114); [Haroz 2022](https://osf.io/preprints/metaarxiv/zry2u)) | Low (terminology) | Use "protocol committed (SHA …) before data"; reserve "pre-registered" for registry/timestamped |
| AppA2 | Replaced pod "before any of its results were inspected"; stray process changed ratio "from 0.967 to 0.966" | App. A p.11 | Statement | U | Med | Keep; provide logs |
| AppB | "B CPU/GPU crossover" | p.11 | **Empty** | V: heading with no body | — | Fill (presumably Table 8 + discussion) or remove |
| AppC | "8 of 16 wrong, median regret 0.4%, max 235.7%" ... "a 3.36× ratio exceeds every within-host spread we measured except the laptop light path's 4.2×" | App. C p.11-12 | Prior archive | R: 1 + 2.357 = 3.357; L: suite ran on the very laptop whose spread exceeds it | Low (the "survives" argument) | Drop or reframe the "survives" claim |
| AppD | $0.74/h and $3.49/h; ≈ US$6 | App. D p.12 | Ledger | U | Med | Tabulate hours |

### Inferences
- Of 80 rows: about 35 High, 25 Med, 20 Low or Low-Med (approximate tally). The Low-confidence rows cluster in three places: cross-host mechanism claims, generalizations in Section 8, and registration terminology.

---

## Q7. Severity-ranked issues list (feeds Phase 4 / Phase 9)

### Takeaway
Two issues threaten validity (S1, S2). The rest are about over-generalization, presentation, statistics and terminology.

### Cited Findings / Inferences (ranked; all citations [MS p.N] or [Script] as above)
**Critical (threaten the central claim as worded)**
1. **Unstable unlocked baseline on the same laptop between D1 (Sept 25) and E-LOCK (Oct 19).**
   - Unlocked light medians differ 1.23-2.30x (0.030 vs 0.069; 0.106 vs 0.176; 0.461 vs 0.567).
   - The unlocked spread pattern differs: D1 has 1.00x at E=1k, E-LOCK 2.84x.
   - The heater effect at E=1k is 1.00 in D1 vs 0.57 in E-LOCK.
   - The paper mixes the two (K3 compares E-LOCK Llo to D1 U) and never discusses the drift.
   - **Action:** report drift; check driver, Windows build, NVIDIA power-management-mode and power settings for both dates; re-run D1's unlocked arm alongside E-LOCK.
2. **Cross-host mechanism unsupported quantitatively.**
   - Pod light-path median SM clocks are within 79-89% of boost, yet show a 4.6x spread.
   - H100 at E=10k: a 3.07x clock rise yields only a 1.23x speed-up.
   - Table 3 differences on pods/H100 are 3 and 1 runs.
   - **Action:** per-process clock at kernel time vs per-process kernel time on every host; clock event reasons; ≥ 3 blocks per pod; downgrade to "not explained by sampled SM clock on pods".

**Major**
3. **Busy-fraction dismissal is contradicted by the paper's own numbers.** Exogenous (locked-clock) busy fraction falls from 13.5% to 8.7% as E grows [Script; N16]. Host materialization growth is a simpler explanation of the "clock falls as E grows" pattern. **Action:** manipulate inter-query host gap at fixed E.
4. **Generalization of "unlocked benchmarks understate the efficient implementation".**
   - One laptop, one naive comparator.
   - The headline factor (2.9-5.0x) uses the slowest of three possible unlocked baselines; it is about 1.45-2.0x against the best unlocked.
   - No end-to-end measurement under locking.
   - **Action:** narrow; add end-to-end under lock; add a tuned top-k (e.g., a library selection primitive) as comparator.
5. **The E=10,000 "application would choose it" regime may be an artifact of the CPU baseline switching to FP64 above E=4,096.** CPU per-row cost jumps 3.25-3.37x [Script; N14]. **Action:** report FP32 CPU at E=10k; justify FP64.
6. **Statistics at n = 3-5.**
   - Percentile-bootstrap "95%" intervals are [min, max] with 75-94% coverage.
   - No multiplicity control over about 65 cell verdicts.
   - Bimodality summarized by medians.
   - Host × process confounded on pods.
   - Machine-level pseudoreplication (one laptop, one H100, one host A).
   - **Action:** raw per-block dot plots, exact sign/permutation tests, mode counts, ≥ 10 blocks per machine, hierarchical model, one pre-specified primary outcome per experiment.
7. **Kernel effect does not reach the application.** Laptop E=10k: device time halved by the heater, call time 1.682 → 1.685 ms; P1 failed on every GPU. The paper reports this honestly, but Section 8's practical recommendations (router calibration, co-location) rest on device-level effects. **Action:** report end-to-end under E-LOCK conditions; make end-to-end the primary metric for application claims.
8. **Heater is multi-channel.** Host CPU contention, queue contention, idle-state prevention and keep-alive are all possible. "Speed-up implies power state" is too strong. **Action:** add a CPU-only control (heater thread without GPU work), a GPU-only control pinned to other cores, and a "Prefer maximum performance" driver setting arm on Windows.
9. **"Pre-registered" terminology.** Local git commits with author-settable dates are not registrations by the standard definition ([Nosek 2018](https://www.pnas.org/doi/10.1073/pnas.1708274114); [OSF](https://help.osf.io/article/330-welcome-to-registrations); [Haroz 2022](https://osf.io/preprints/metaarxiv/zry2u); [git-commit --date](https://git-scm.com/docs/git-commit)). **Action:** reword (see Q3); use OSF or OpenTimestamps in future.
10. **Exploratory headline.** The cross-host table (Table 1) and the device-time framing were not planned [MS p.11], yet they supply the abstract's headline numbers. **Action:** label as exploratory in the abstract.

**Moderate**
11. Abstract scoping: 4.2x is from E=3,000; the H100 is not in Table 8; "low clocks" does not hold on pods.
12. Appendix C "survives" argument is self-contradictory (the laptop is the suite host).
13. **Empty Appendix B**; Table 8 orphaned after Appendix D; Tables 6-7 float inside Appendix C; Section 7 cites values absent from Table 4 (heavy Llo, heavy U, U+heater, Lhi+heater).
14. Host A described as 5 blocks (Table 1) vs "ten identical blocks" (§4); the E of the 2.44x is unstated.
15. Spearman p reported from the asymptotic approximation (0.003) where the exact value is 0.0047 (two-sided); endogeneity of the clock (slow kernels → lower utilization) is acknowledged in §5 but not in §4.
16. "Identical results" vs a 0.99 recall gate and "except where noted" with nothing noted.
17. Telemetry: GPU-active median includes warm-up and data loading; 50 ms polling; no clock event reasons; NVIDIA Power management mode not reported.
18. Fig. 1b: the heater raises the laptop light clock to about 1,100 MHz, not to boost (visual); the caption's "raises it" is correct but omits that the heater does not restore boost. n = 1 trace per condition.

**Minor / editorial**
19. "Lower median of three per-pass percentiles" undefined.
20. "Median-block" in Table 8 undefined.
21. Pods E=30,000 P3 tie scored as "yes" (device) and "no" (call).
22. "Uniformly slower on the laptop (1.19×)" contradicted by E=30k (1.00).
23. Heavy "device work proportional to N regardless of E" vs heavy scoring time growing 1.56x with E.
24. "Tail reaching 3.3×" is the p90; the maximum ≈ 4.8x (Fig. 1c).
25. Memory lock 8,001 MHz is described as the heavy path's boost, but telemetry shows heavy at 8,201 MHz.
26. Repository paths broken across lines with stray spaces in the Reproducibility section.
27. Reporting rules (i)-(v) presented as a contribution; mostly restate [K&J 2013](https://dl.acm.org/doi/10.1145/2464157.2464160) and [Hoefler & Belli 2015](https://htor.inf.ethz.ch/publications/img/hoefler-scientific-benchmarking.pdf).

---

## Q8. Hostile peer-review inputs (Phase 4)

### Takeaway
Recommendation input: **major revision (borderline reject at a full systems venue; plausible accept at a measurement/reproducibility workshop after narrowing).**
- Strengths: an unusually transparent single-machine causal experiment (E-LOCK + E-SPLIT); honest reporting of failed predictions; fresh-process replication; released data.
- Weaknesses: n = 1 machine for the causal claim; a quantitatively unsupported cross-host story; a strawman comparator; device-only effects that vanish end to end; small-n statistics presented with CIs that cannot reach their nominal coverage.

### Cited Findings / Inferences
- **Summary:** Measures the process-to-process variability of two Qenlo GPU top-k paths (compact-row light vs full-scan heavy) across a laptop RTX 4050, RTX 4090 hosts/pods and an H100. Attributes the light path's variability to driver-chosen low clocks, demonstrated via clock locking on the laptop and a load-raising heater elsewhere [MS p.1].
- **Main contribution:** A clean locked-clock intervention showing that, on one laptop, driver clock selection drives the light kernel's run-to-run variation and that the SM clock (not memory) sets the selection kernel's level [MS p.5-7].
- **Strengths (evidence-backed):**
  - Fresh-process blocks as replicates [MS p.3].
  - Shuffled within-block order [MS p.4].
  - Factorial SM × memory design with a pre-committed analysis script [MS p.6, p.11].
  - Failed predictions reported (P1, K2) [MS p.5-6].
  - Exclusions and replacements documented [MS p.11].
  - All table arithmetic reproduces [Script].
- **Questions an informed reviewer would ask:**
  1. Why does the unlocked laptop baseline differ by up to 2.3x between D1 and E-LOCK, and what changed?
  2. On the pods, why does a 4.6x spread coexist with median clocks at 79-89% of boost? Show per-process clock vs time.
  3. What is the end-to-end latency under Lhi vs U on the laptop?
  4. Does setting NVIDIA "Prefer maximum performance" (no admin needed) reproduce the lock?
  5. Does the effect persist against a competitive top-k (e.g., radix or bitonic selection, a library primitive)?
  6. What is the CPU baseline at E=10,000 in FP32?
  7. What happens with batch > 1 or concurrent queries (the protocol D2 was written but not run [MS p.11])?
  8. Are timestamps taken per dispatch or per pass, and what is `timestampPeriod`?
  9. Does the slow mode include idle-exit latency?
  10. Can a CPU-only "heater" reproduce the heater's device-time effect?
- **Missing experiments:**
  - End-to-end latency under lock.
  - Inter-query delay sweep (busy-fraction test).
  - Driver power-mode arm.
  - Linux on the same laptop.
  - ≥ 3 blocks per pod.
  - A second laptop or desktop GPU with admin rights for locking.
  - A tuned comparator.
  - Batch-size sweep.
  - CPU-only and GPU-only heater controls.
- **Missing comparisons:** standard GPU top-k/selection primitives; FAISS/cuVS brute-force filtered search (acknowledged as not done [MS p.7]).
- **Likely rejection triggers:** the generality of Section 8 vs single-laptop evidence; the device-only effect with null end-to-end; small n with inferential language ("95% intervals"); the naive comparator; "pre-registered" terminology.

### Gaps
- Venue-specific judgment is not attempted; the standard used is generic high-quality systems review.
