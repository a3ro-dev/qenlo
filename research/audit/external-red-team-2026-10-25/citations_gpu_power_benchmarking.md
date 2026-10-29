# Citation audit: GPU power management, DVFS, server power and benchmarking methodology (Qenlo manuscript)

Scope: the 26 citations assigned from "Power-State Variance in One WGPU Vector-Search Kernel" (manuscript at /home/claude/QENLO-RESEARCH-PAPER.pdf). Citing passages were taken from the extracted text: Abstract and Sec. 1 (clock-locking practice), Sec. 3 and Table 2 caption (nvidia-smi partial sampling), Sec. 5 (undocumented NVIDIA policy), Sec. 9 "Power management versus latency" and "Benchmark validity", and Sec. 10 Telemetry.

Access notes: direct shell access to export.arxiv.org and api.crossref.org was blocked by the egress proxy, so every record below was opened through a web-fetch tool. The URLs listed are the ones actually opened. IEEE Xplore document pages returned HTTP 418 and DBLP returned a bot wall, so I used Crossref, Semantic Scholar, the authors' own pages, Google Research and computer.org for those records.

## Q1. Per-citation verification and classification

### Takeaway
All 26 citations exist, and every arXiv ID resolves to the paper the manuscript names. No ID points to a different paper. Four problems matter most:
- **Sinha et al. SC22.** The "8–22% caused by power management" wording is materially inaccurate.
- **Yang et al. 2023.** The paper is about power readings only, so citing it for clock telemetry stretches it. It has also since been published at SC24.
- **Maliakel et al.** The manuscript cites only the arXiv preprint, but the paper is now published at CCGrid 2026.
- **Chu and Li 2026.** This paper directly undercuts the manuscript's statement in Sec. 9 that no prior report shows power state changing the measured margin between two implementations.

### Cited Findings

Classification key: **DS** = Verified and directly supportive; **PS** = Verified but only partially supportive; **BG** = Relevant background but not evidence for the attributed claim; **MA** = Misattributed or materially inaccurate; **META** = Incorrect or incomplete bibliographic metadata; **UNV** = Unverifiable.

**1. Adamek, Novotny, Thiyagalingam, Armour (2021), IEEE Access 9:18167–18182, doi 10.1109/ACCESS.2021.3053409.** Claim: "at low locked clocks desktop GPUs can drop into idle-like states."
- Metadata: title, journal, pages and year all match Crossref. — [Crossref](https://api.crossref.org/works/10.1109/ACCESS.2021.3053409); [ORCID record for J. Novotny, DOI 10.1109/access.2021.3053409](https://orcid.org/0000-0002-9667-635X)
- Content: I checked this in the arXiv v2 (revised 9 Nov 2021, the version matching the journal article), not in the IEEE copy. Clocks were set with `nvmlDeviceSetGpuLockedClocks`. The authors explain the sharp rise in execution time at low frequencies as "due to the change of the P-state to a state corresponding to the idle status of the GPU with reduced voltage". — [arXiv 2009.06009v2 PDF](https://arxiv.org/pdf/2009.06009v2)
- Caveat: the GPUs tested were V100, Titan V, Titan XP, Tesla P4 and Jetson Nano, mostly datacenter or workstation parts, so "desktop GPUs" is loose wording. The memory clock was not varied in that study. — [arXiv 2009.06009v2](https://arxiv.org/pdf/2009.06009v2)
- **Classification: DS.** Suggested wording: "NVIDIA GPUs (V100, Titan, P4)" in place of "desktop GPUs".

**2. Chou, Bhuyan, Wong. µDPM: Dynamic power management for the microsecond era. HPCA 2019, pp. 120–132, doi 10.1109/HPCA.2019.00032.**
- Metadata matches: pages 120–132, Feb 2019, Washington DC. — [Crossref](https://api.crossref.org/works/10.1109/HPCA.2019.00032)
- The abstract says "killer microsecond" service times "can cause state-of-the-art dynamic power management techniques to break down" because idle periods are short. µDPM coordinates request delaying, per-core sleep states and DVFS to meet tail latency. — [Semantic Scholar](https://api.semanticscholar.org/graph/v1/paper/DOI:10.1109/HPCA.2019.00032?fields=title,abstract,authors,venue,year)
- **Classification: DS** for "motivating latency-aware controllers" (CPU servers, not GPUs).

**3. Chu and Li (2026). Hardware-Attributed Operator Profiling for PyTorch. arXiv 2609.11938.** Claim: kernel benchmarking practice locks clocks.
- Metadata: title and authors (Logan Chu, Dong Li, both Yotta Labs) match. The current version is v2, revised 29 Sep 2026, cs.AR. It is a preprint and I found no peer-reviewed venue. — [arXiv abs](https://arxiv.org/abs/2609.11938)
- **Anomaly to flag.** The abstract page says "Submitted on 4 Jul 2026" for v1. arXiv IDs normally encode the year and month of submission, and 2609 means September 2026, so the ID and date disagree. I could not resolve this. Re-check it on arxiv.org by hand before citing. — [arXiv abs](https://arxiv.org/abs/2609.11938)
- Content: Section 3.2, "GPU Clock Locking for Reproducible Duration Comparisons", pins the SM and memory clocks during capture using "probe-and-lock". It states that boost clocks typically vary "5–15% across captures". It also states that "an optimized workload dissipates less power than the baseline ... so its capture may execute at a higher sustained clock frequency, artificially elevating the apparent speedup". — [arXiv HTML v2](https://arxiv.org/html/2609.11938v2)
- Limitations the authors state themselves: one GPU (RTX PRO 6000 Blackwell), and point estimates from 2 measured iterations. — [arXiv HTML v2](https://arxiv.org/html/2609.11938v2)
- **Classification: DS** for "practice locks clocks", with the caveat that it is a single non-peer-reviewed preprint. See Q3 for its bearing on the manuscript's novelty claim.

**4. Chung, Gu, Jang, Meng, Bansal, Chowdhury. Reducing Energy Bloat in Large Model Training (Perseus). SOSP '24, pp. 144–159, doi 10.1145/3694715.3695970.**
- Metadata matches: six authors in order, pages 144–159, published 4 Nov 2024. — [Crossref](https://api.crossref.org/works/10.1145/3694715.3695970)
- The abstract describes "energy bloat" and schedules computation energy over time, reducing training energy "by up to 30% without any throughput loss". — [Semantic Scholar](https://api.semanticscholar.org/graph/v1/paper/DOI:10.1145/3694715.3695970?fields=title,abstract,authors)
- **Classification: DS** for "GPU DVFS work for ML treats frequency as a knob to lower for energy".

**5. Golden, Kuchnik, Hsia, DeVito, Wei, Brooks, Wu. PRISM. arXiv 2510.15596.** Claim: "1–14% in GEMM timings".
- Metadata: title and seven authors match. v1 was 17 Oct 2025; the latest is v3, 21 Sep 2026. — [arXiv abs](https://arxiv.org/abs/2510.15596)
- Status: still a preprint. The first author's site lists it as "Under Submission / ArXiv". — [Alicia Golden site](https://sites.harvard.edu/alicia-golden/)
- Content (Sec. 2.2 and Fig. 3): **spatial** variation of 1.64–14.04% across GPUs, each GEMM run on 2,944–6,808 GPUs. **Temporal** variation of 0.98–6.46% over N=1000 runs on one representative GPU. The contributions list says "GEMM kernels can experience as much as 14% variation". — [arXiv HTML v3](https://arxiv.org/html/2510.15596v3)
- Attribution: the paper says "a significant source" of the variation is the "dynamically varying frequency setting of GPUs" (Fig. 4: about 500 MHz and 140 W spread on A100s), and that "76% of the time, frequency changes are due to software thermal limits". — [arXiv HTML v3](https://arxiv.org/html/2510.15596v3)
- **Classification: PS.** The "1–14%" figure merges the temporal lower bound (0.98%) with the spatial upper bound (14.04%). The cause is mainly thermal and power capping under heavy load, which is the opposite regime from idle down-clocking. Suggested fix: "1.6–14% across a fleet of GPUs and 1–6.5% across repeated runs on one GPU, partly from thermally driven frequency changes (Golden et al., 2025, preprint)".

**6. Hsu, Zhang, Laurenzano, Meisner, Wenisch, Mars, Tang, Dreslinski. Adrenaline. HPCA 2015, pp. 271–282, doi 10.1109/HPCA.2015.7056039.**
- Metadata matches: eight authors and page 271 per the proceedings table of contents. — [Crossref](https://api.crossref.org/works/10.1109/HPCA.2015.7056039); [HPCA 2015 TOC](https://www.proceedings.com/content/025/025645webtoc.pdf)
- Content not verified: the abstract is elided in Semantic Scholar and IEEE Xplore returned HTTP 418. Only the title, "Pinpointing and reining in tail queries with quick voltage boosting", is confirmed. — [Semantic Scholar](https://api.semanticscholar.org/graph/v1/paper/DOI:10.1109/HPCA.2015.7056039?fields=title,abstract,authors,venue,year)
- **Classification: BG (metadata verified, content not verified).** The title is consistent with "latency-aware controllers".

**7. Kakolyris, Masouros, Vavaroutsos, Xydis, Soudris. throttLL'eM. HPCA 2025, pp. 1363–1378, doi 10.1109/HPCA61900.2025.00103.**
- Metadata matches. — [Crossref](https://api.crossref.org/works/10.1109/HPCA61900.2025.00103)
- The abstract describes cutting energy under SLOs "through the use of instance and GPU frequency scaling", with up to 43.8% lower energy. — [Semantic Scholar](https://api.semanticscholar.org/graph/v1/paper/DOI:10.1109/HPCA61900.2025.00103?fields=title,abstract,authors)
- **Classification: DS** for the energy-knob framing.

**8. Kalibera and Jones. Rigorous benchmarking in reasonable time. ISMM '13, pp. 63–74, doi 10.1145/2464157.2464160.**
- Metadata matches. — [Crossref](https://api.crossref.org/works/10.1145/2464157.2464160)
- The abstract says repetition is needed "between builds, between executions or between iterations" and recommends effect-size confidence intervals. — [Semantic Scholar](https://api.semanticscholar.org/graph/v1/paper/DOI:10.1145/2464157.2464160?fields=title,abstract,authors,venue,year)
- **Classification: DS** for "variation across executions must be sampled and reported". It is also strong support for the manuscript's fresh-process replication unit.

**9. Kanev, Hazelwood, Wei, Brooks. Tradeoffs between power management and tail latency in warehouse-scale applications. IISWC 2014, pp. 31–40, doi 10.1109/IISWC.2014.6983037.**
- Metadata matches. — [Crossref](https://api.crossref.org/works/10.1109/IISWC.2014.6983037)
- The abstract reports 15 Google workloads, 9 of which toggle cores between short bursts and sleep. This "stress[es] sleep selection algorithms and can cause tail latency degradation". — [Kanev's paper page](https://skanev.org/papers/iiswc14ep.html)
- **Classification: DS** (CPU servers).

**10. Kasture, Bartolini, Beckmann, Sanchez. Rubik. MICRO-48, 2015, pp. 598–610, doi 10.1145/2830772.2830797.**
- Metadata matches. Crossref stores the title as "Rubik" with the rest as a subtitle. Dated 5 Dec 2015. — [Crossref](https://api.crossref.org/works/10.1145/2830772.2830797)
- The abstract says servers are "kept lightly loaded", and that prior techniques "adapt slowly and conservatively". Rubik is a fine-grain DVFS scheme that meets tail latency. — [ACM DL](https://dl.acm.org/doi/10.1145/2830772.2830797)
- **Classification: DS** for "latency-aware controllers", and partially for "slow governors".

**11. Lin, Wang, Li, Pu. A Workload-Aware DVFS Robust to Concurrent Tasks for Mobile Devices. MobiCom '23, pp. 1–16, doi 10.1145/3570361.3592524.**
- Metadata matches. Crossref lists the fourth author as "Yu Pu". — [Crossref](https://api.crossref.org/works/10.1145/3570361.3592524)
- The abstract says application-oriented governors' "performance drops significantly when other tasks run concurrently" and proposes GearDVFS. — [Semantic Scholar](https://api.semanticscholar.org/graph/v1/paper/DOI:10.1145/3570361.3592524?fields=title,abstract,authors,venue,year)
- **Classification: DS** for "mispredict ... concurrent work". The paper is about mobile CPU/GPU DVFS in general, not GPUs specifically.

**12. Lo, Cheng, Govindaraju, Barroso, Kozyrakis. Towards energy proportionality for large-scale latency-critical workloads (PEGASUS). ISCA 2014, pp. 301–312, doi 10.1109/ISCA.2014.6853237.**
- Metadata matches. An ACM mirror also exists at doi 10.5555/2665671.2665718. — [Crossref](https://api.crossref.org/works/10.1109/ISCA.2014.6853237); [computer.org](https://www.computer.org/csdl/proceedings-article/isca/2014/06853237/12OmNrNh0CL)
- Content (full paper): utilization-driven `cpufreq` DVFS "leads to frequent latency spikes" and SLO violations for search (Fig. 7). The default policy races to idle. At low load there is latency slack. — [Google Research PDF](https://research.google.com/pubs/archive/42523.pdf)
- **Classification: DS** for both "slow governors inflate latency" and "latency-aware controllers". Nuance: the latency inflation comes from a utilization-based governor; at low load, latency under the default policy is actually below the SLO.

**13. Maliakel, Ilager, Brandic (2025). arXiv 2501.08219.**
- Metadata: the v4 title (24 Feb 2026), "Characterizing LLM Inference Energy-Performance Tradeoffs across Workloads and GPU Scaling", matches the manuscript. — [arXiv abs](https://arxiv.org/abs/2501.08219)
- The v1 title was different: "Investigating Energy Efficiency and Performance Trade-offs in LLM Inference Across Tasks and DVFS Settings". — [axi.lims mirror of v1](https://axi.lims.ac.uk/paper/2501.08219)
- **Now peer-reviewed:** CCGrid 2026, pp. 33–43, doi 10.1109/CCGrid68966.2026.00013. — [computer.org CCGrid 2026](https://www.computer.org/csdl/proceedings-article/ccgrid/2026/706400a033/2izmQ0raMZq)
- **Classification: META.** The content (DVFS for energy) fits as background.

**14. Maricq, Duplyakin, Jimenez, Maltzahn, Stutsman, Ricci. Taming Performance Variability. OSDI 18, pp. 409–425.**
- Metadata matches the USENIX BibTeX, ISBN 978-1-939133-08-3. — [USENIX](https://www.usenix.org/conference/osdi18/presentation/maricq)
- The abstract covers nearly 900,000 data points from 835 servers. Software run repeatedly on the same server, or on "supposedly identical" servers, varies. — [USENIX](https://www.usenix.org/conference/osdi18/presentation/maricq)
- **Classification: DS.**

**15. Meisner, Sadler, Barroso, Weber, Wenisch. Power management of online data-intensive services. ISCA '11, pp. 319–330, doi 10.1145/2000064.2000103.**
- Metadata matches. — [Crossref](https://api.crossref.org/works/10.1145/2000064.2000103); [Semantic Scholar](https://api.semanticscholar.org/graph/v1/paper/DOI:10.1145/2000064.2000103?fields=title,authors)
- The abstract says that for OLDI workloads, energy proportionality "with acceptable query latency can only be achieved using coordinated, full-system active low-power modes", meaning idle low-power modes do not work. — [ACM DL](https://dl.acm.org/doi/10.1145/2000064.2000103)
- **Classification: DS** (model-based CPU-server study; 95th-percentile latency).

**16. Mytkowicz, Diwan, Hauswirth, Sweeney. Producing wrong data without doing anything obviously wrong! ASPLOS '09, pp. 265–276, doi 10.1145/1508244.1508275.**
- Metadata matches. — [Crossref](https://api.crossref.org/works/10.1145/1508244.1508275)
- **Classification: DS (metadata verified; content not verified in this session).** The abstract is elided in Semantic Scholar. The paper's well-known measurement-bias finding (environment size and link order) fits "innocuous setup details bias measured speed-ups", but I did not open it. — [Semantic Scholar](https://api.semanticscholar.org/graph/v1/paper/DOI:10.1145/1508244.1508275?fields=title,abstract,authors)

**17. Oliaro, Fu, Jiang, Lu, Wang, Jia, Zhang, Rajbhandari (2026). FastKernels. arXiv 2605.23215.** Claim: kernel benchmarking locks clocks.
- Metadata: the abstract page lists v1 only (22 May 2026) with these eight authors. — [arXiv abs](https://arxiv.org/abs/2605.23215)
- **Discrepancy:** a search snippet of the arXiv PDF showed a different author line ("Gabriele Oliaro, Jaeseong Lee, Yichao Fu, Zhaoyuan Su ..."). This could be a newer revision or a cache artifact; I could not resolve it. — [arXiv PDF as indexed](https://arxiv.org/pdf/2605.23215)
- Content: Appendix E says "GPU clocks are locked at 1,593 MHz for reproducible timing" on H100 SXM5, and the Limitations section says "Performance is measured on locked-clock H100 SXM5". — [arXiv HTML v1](https://arxiv.org/html/2605.23215v1)
- **Classification: PS.** It is one instance of the practice inside a benchmark for kernel-generation agents. It does not discuss or document why kernel benchmarking locks clocks.

**18. Pathania, Jiao, Prakash, Mitra. Integrated CPU-GPU power management for 3D mobile games. DAC '14, pp. 1–6, doi 10.1145/2593069.2593151.**
- Metadata matches: four authors, all at NUS. — [Crossref](https://api.crossref.org/works/10.1145/2593069.2593151)
- **Classification: BG (metadata verified, content not verified).** The abstract is elided. The title shows a CPU-GPU governor co-design for games, which fits loosely with "mobile GPU governors mispredict bursty work", but I could not confirm that the paper says this. — [Semantic Scholar](https://api.semanticscholar.org/graph/v1/paper/DOI:10.1145/2593069.2593151?fields=title,abstract,authors)

**19. Qiu et al. Power-aware Deep Learning Model Serving with µ-Serve. USENIX ATC 24, pp. 75–93.**
- Metadata matches: ten authors and ISBN 978-1-939133-41-0. The abstract reports 1.2–2.6× power saving "by dynamic GPU frequency scaling". — [USENIX](https://www.usenix.org/conference/atc24/presentation/qiu)
- **Classification: DS** (energy-knob framing).

**20. Sinha, Guliani, Jain, Tran, Sinclair, Venkataraman. Not All GPUs Are Created Equal. SC22, pp. 1–15, doi 10.1109/SC41404.2022.00070.** Claim: "power management causes 8–22% variation across same-model devices under load".
- Metadata matches: six authors (UW-Madison), Dallas, Nov 2022. The preprint is arXiv 2208.11035. — [Crossref](https://api.crossref.org/works/10.1109/SC41404.2022.00070); [arXiv](https://arxiv.org/abs/2208.11035)
- See Q2 for the exact figures. **Classification: MA (quantitative framing).** The numbers exist, but "8–22%" is not a range the paper gives: 8% is the average and 22% the maximum. The 22% case is also not attributed to power management.

**21. Spaan, Chen, Varbanescu (2026). Reducing Compute Waste in LLMs through Kernel-Level DVFS. arXiv 2601.08539.** Claim: "The NVIDIA policy is undocumented".
- Metadata matches: v1, 13 Jan 2026, cs.PF. — [arXiv abs](https://arxiv.org/abs/2601.08539)
- Status: preprint only. Semantic Scholar lists only arXiv. — [Semantic Scholar](https://api.semanticscholar.org/graph/v1/paper/arXiv:2601.08539?fields=title,venue,year,publicationVenue,externalIds,journal)
- Content (Sec. 2.2, "Automatic DVFS"): automatic scalers are performance-oriented, and "For NVIDIA GPUs, its exact workings are not disclosed". It adds that DVFS behaviour "differs between generations and models", citing Velicka et al. 2025. — [arXiv HTML v1](https://arxiv.org/html/2601.08539v1)
- **Classification: DS.** It is a secondary statement in a preprint. It also fits the Sec. 9 "energy knob" list: the paper saves 14.6% energy with a 0.6% slowdown on an RTX 3080 Ti.

**22. Stojkovic, Zhang, Goiri, Torrellas, Choukse. DynamoLLM. HPCA 2025, pp. 1348–1362, doi 10.1109/HPCA61900.2025.00102.**
- Metadata matches. The abstract lists GPU frequency among its reconfiguration knobs and reports 52% energy saved. — [Crossref](https://api.crossref.org/works/10.1109/HPCA61900.2025.00102); [Semantic Scholar](https://api.semanticscholar.org/graph/v1/paper/DOI:10.1109/HPCA61900.2025.00102?fields=title,abstract,authors)
- **Classification: DS** (energy-knob framing).

**23. Tang, Wang, Wang, Chu. The impact of GPU DVFS on the energy and performance of deep learning: an empirical study. e-Energy '19, pp. 315–325, doi 10.1145/3307772.3328315.**
- Metadata matches. — [Crossref](https://api.crossref.org/works/10.1145/3307772.3328315)
- The paper varies core and memory frequency on DNN training and inference to measure both energy and performance. — [arXiv 1905.11012](https://arxiv.org/pdf/1905.11012)
- **Classification: DS** (background).

**24. Yang, Adamek, Armour (2023). Part-time Power Measurements: nvidia-smi's Lack of Attention. arXiv 2312.02741.**
- Metadata: title and authors match. The latest is v3, 12 Dec 2024. — [arXiv abs](https://arxiv.org/abs/2312.02741)
- **Now peer-reviewed:** the same study appears as "Accurate and Convenient Energy Measurements for GPUs: A Detailed Study of NVIDIA GPU's Built-In Power Sensor", SC24, pp. 1–17, doi 10.1109/SC41406.2024.00028. The IEEE abstract repeats the "only 25% of the runtime is sampled" and 35%-error-reduction findings. — [IEEE Xplore](https://ieeexplore.ieee.org/abstract/document/10793163); [computer.org](https://www.computer.org/csdl/proceedings-article/sc/2024/529100a307/21HUVkzjyWk)
- See Q5 for content. **Classification: META, and MA for the clock-telemetry uses.**

**25. You, Chung, Chowdhury. Zeus. NSDI 23, pp. 119–139.**
- Metadata matches, ISBN 978-1-939133-33-5. The abstract covers GPU-level configuration for energy, improving efficiency by 15.3–75.8%. — [USENIX](https://www.usenix.org/conference/nsdi23/presentation/you)
- **Classification: DS** (energy-knob framing).

**26. Zhang, Dash, Hu, Xu, Li, Guan (2025). Dissecting the Impact of Mobile DVFS Governors on LLM Inference Performance and Energy Efficiency. arXiv 2507.02135.**
- Metadata matches: v1 only, 2 Jul 2025. Zhang and Dash are equal-contribution authors. — [arXiv abs](https://arxiv.org/abs/2507.02135)
- Content: mobile CPU, GPU and memory governors "operate independently and are oblivious of each other", giving up to 40.4% longer latency. — [arXiv abs](https://arxiv.org/abs/2507.02135)
- Possible published successor: the same author group's MLSys 2026 oral, "Rethinking DVFS for Mobile LLMs: Unified Energy-Aware Scheduling with CORE", reports the same 23.0–40.4% latency finding. It is not confirmed to be the same paper. — [MLSys 2026](https://mlsys.org/virtual/2026/oral/3814)
- **Classification: PS.** The paper is about uncoordinated governors, not "bursty" misprediction. Possible META issue if the MLSys paper is its published version.

### Inferences
- No citation in this cluster is fabricated or resolves to the wrong paper. The problems are claim-to-source fidelity (Sinha, Golden, Yang) and stale metadata (Yang now SC24, Maliakel now CCGrid 2026).
- Nearly all of the server-power and ML-DVFS citations describe CPU servers or high-load GPU energy optimization. They are fair background for the Sec. 9 framing but not evidence about idle-GPU down-clocking. The manuscript's "we study the opposite case" sentence is consistent with this.

### Gaps
- Adrenaline (HPCA 2015) and Pathania (DAC 2014): content not verified, because the abstracts are elided and IEEE returned 418.
- Mytkowicz (ASPLOS 2009): content not opened.
- The 2609.11938 ID/date mismatch and the FastKernels author-list discrepancy need someone to look at arxiv.org directly.

## Q2. Does Sinha et al. SC22 report 8–22% variation from power management? Under what workloads and GPUs?

### Takeaway
No, not in the form the manuscript states. The paper reports **8% average (maximum 22%)** performance variation across same-SKU GPUs. The 22% maximum is ResNet-50 multi-GPU training, which the paper does **not** tie to frequency or power-management throttling. Power-management-driven variation is clearest for SGEMM, at about 7–9%.

### Cited Findings
- Headline: "8% (max 22%) average performance variation" across same-SKU GPUs, "with outliers up to 1.5X slower than the median GPU". Data come from five clusters, over 18,800 GPU-hours and more than 90% of each cluster's GPUs. — [arXiv 2208.11035](https://arxiv.org/abs/2208.11035)
- Clusters and GPUs: Summit, Longhorn and Vortex use V100s; Frontera uses Quadro RTX 5000; Corona uses AMD MI60. The paper says variation occurs "due to manufacturing variability and the chip's PM". — [arXiv PDF](https://arxiv.org/pdf/2208.11035)
- SGEMM varies 7–9% across GPUs in the same cluster. On Longhorn, performance and frequency are strongly correlated (ρ = −0.97). GPUs configured for 1530 MHz actually ran at 1300–1440 MHz. On Vortex, frequency drops are "driven by the PM infrastructure trying to keep power consumption under the GPU's TDP". — [arXiv PDF](https://arxiv.org/pdf/2208.11035)
- ResNet-50 shows 22% (multi-GPU) and 14% (single-GPU) variation. However, "most nodes run at the max 1530MHz", iteration time and frequency are "almost uncorrelated" (ρ = −0.01), and single-GPU runs "observe little PM interference". — [arXiv PDF](https://arxiv.org/pdf/2208.11035)
- Other workloads: BERT about 8%, LAMMPS under 1%, PageRank 1%. Memory-bound work varies little. — [arXiv PDF](https://arxiv.org/pdf/2208.11035)
- Variation on a single GPU across independent runs is small: median 0.44% on Longhorn, 0.12% on Summit, 6.06% on Corona. — [arXiv PDF](https://arxiv.org/pdf/2208.11035)
- On CloudLab, pinning versus not pinning frequency gave similar variation (9% at a 300 W limit). Variation grows at low power caps (18% at 150 W). — [arXiv PDF](https://arxiv.org/pdf/2208.11035)

### Inferences
- Suggested replacement: "Same-SKU datacenter GPUs vary by 8% on average (up to 22%) under sustained compute-heavy load, from power management and manufacturing variability (Sinha et al., 2022)."
- Sinha's regime is TDP and power-cap throttling under heavy load, which is the opposite of the manuscript's light-load down-clocking. That makes it a contrast case rather than a corroboration.
- Sinha's single-GPU, cross-run variation is small (0.12–6%), whereas the manuscript reports up to 4.2× across processes on one machine. This strengthens the manuscript's argument that its effect is a different mechanism.

### Gaps
- None material.

## Q3. Golden PRISM's "1–14%"; whether Oliaro and Chu & Li discuss clock locking; better primary sources

### Takeaway
- **PRISM:** 14% is the top of the cross-GPU (spatial) range and about 1% is the bottom of the repeated-run (temporal) range, in Sec. 2.2 and Fig. 3. Merging them into "1–14%" mixes two different quantities.
- **Clock locking:** both 2026 preprints do lock clocks. FastKernels only states the setting; Chu & Li give the reasoning.
- **Better sources:** for the general practice, primary tool documentation is a stronger citation: Nsight Compute's clock control, nvbench's clock-locking option, Triton's `set_gpu_clock`, and Nsight Graphics' "Lock Clocks to Base".
- **Novelty:** Chu & Li's Sec. 3.2 describes implementation-dependent clock states biasing speed-up ratios. This contradicts the manuscript's statement in Sec. 9: "We have not found a report that this changes the measured margin between two implementations".

### Cited Findings
- PRISM Sec. 2.2: "1.64-14.04% spatial variation across GPUs for the GEMM kernel" and "0.98-6.46% temporal variation". Each GEMM was run on 2,944–6,808 GPUs across a fleet of more than 20,000. — [arXiv HTML v3](https://arxiv.org/html/2510.15596v3)
- FastKernels Appendix E: "GPU clocks are locked at 1,593 MHz for reproducible timing". Measurements average 3 runs with 10 warm-up iterations. — [arXiv HTML v1](https://arxiv.org/html/2605.23215v1)
- Chu & Li Sec. 3.2: boost clocks float "5–15% across captures". A lower-power optimized workload "may execute at a higher sustained clock frequency, artificially elevating the apparent speedup". Their tool therefore locks the SM and memory clocks to a probed sustained frequency. — [arXiv HTML v2](https://arxiv.org/html/2609.11938v2)
- Nsight Compute Profiling Guide, Sec. 2.6.2: a kernel launched after earlier kernels may find the GPU "in a higher clocked state", while the first kernel sees lower clocks. To reduce this, Nsight Compute "attempts to limit GPU clock frequencies to their boost value". It "locks SM clocks by default", while "Nsight Systems does not lock clocks". When comparing, NVIDIA recommends locking with `nvidia-smi` and using `--clock-control none`. — [NVIDIA Nsight Compute Profiling Guide (2026)](https://docs.nvidia.com/nsight-compute/ProfilingGuide/)
- Older versions of the guide said Nsight Compute limited clocks to their **base** value. The default target has changed across versions, so cite a specific version. — [Nsight Compute 2020.2 guide](https://docs.nvidia.com/nsight-compute/2020.2/ProfilingGuide/index.html); [2021.3 PDF](https://docs.nvidia.com/nsight-compute/2021.3/pdf/ProfilingGuide.pdf)
- CUPTI recommends locking GPU clocks for consistent multi-pass metrics (`nvidia-smi --lock-gpu-clocks=tdp,tdp`). — [CUPTI Special Configurations](https://docs.nvidia.com/cupti/special-configurations/special-configurations.html)
- NVIDIA nvbench lists "locking GPU clocks (Volta+)" as a command-line option. — [NVIDIA/nvbench README](https://github.com/NVIDIA/nvbench)
- Triton `testing.py`: `do_bench` uses warm-up (default 25 ms) and clears L2 before each run, but does not lock clocks. A separate `set_gpu_clock` context manager calls `nvidia-smi --lock-gpu-clocks` and `--lock-memory-clocks` and asserts the clocks hold. — [triton-lang/triton testing.py](https://github.com/triton-lang/triton/blob/main/python/triton/testing.py)
- Nsight Graphics GPU Trace locks clocks to base by default "for better consistency between different captures". — [Nsight Graphics GPU Trace UI](https://docs.nvidia.com/nsight-graphics/UserGuide/gpu-trace-ui.html)

### Inferences
- Suggested citation set for Abstract, Sec. 1 and Sec. 9: the Nsight Compute Profiling Guide (Clock Control), nvbench and Triton `set_gpu_clock` as the primary sources for the practice. Chu & Li can stay as an example with stated rationale; FastKernels as an example only.
- The Abstract's "careful kernel benchmarks lock clocks" is accurate for Nsight Compute and nvbench. It would be overstated to say it of common harnesses such as Triton `do_bench`, which use warm-up but leave clocks unlocked by default.
- **Novelty risk:** Chu & Li (2026) state the same class of confound: the implementations being compared run at different clocks, which biases the ratio between them. Their mechanism is power and thermal headroom under load, the opposite of the manuscript's idle down-clocking. Nsight Compute's documentation also notes that a kernel's clock state depends on what ran before it. The Sec. 9 sentence "We have not found a report that this changes the measured margin between two implementations" should be narrowed. For example: "...that light-load down-clocking makes the more work-efficient implementation look slower", with Chu & Li cited as the closest prior statement.

### Gaps
- I did not find a dedicated NVIDIA developer blog post titled "How to benchmark CUDA kernels" that prescribes clock locking. The Nsight Compute guide is the most authoritative primary source I found.

## Q4. Does Adamek et al. 2021 show idle-like states at low locked clocks?

### Takeaway
Yes. With core clocks locked low through NVML, the authors attribute the sharp rise in FFT time to the GPU switching to the P-state that corresponds to idle. This matters for the manuscript beyond Sec. 9: the manuscript's low lock (L_lo at 585/810 MHz) may itself force an idle-like P-state, which affects how Sec. 7's "the low state alone reproduces the slow mode" should be read.

### Cited Findings
- Clock frequency was set with `nvmlDeviceSetGpuLockedClocks` and reset with `nvmlDeviceResetGpuLockedClocks`. — [arXiv 2009.06009v2](https://arxiv.org/pdf/2009.06009v2)
- "The sharp increase in the execution time t_fix for low frequencies ... are due to the change of the P-state to a state corresponding to the idle status of the GPU with reduced voltage". — [arXiv 2009.06009v2](https://arxiv.org/pdf/2009.06009v2)
- The GPUs tested were Titan XP, Tesla P4, Jetson Nano (GDDR, memory clock adjustable), and Titan V and V100 (HBM2, fixed memory clock). The memory clock was not changed in the study. — [arXiv 2009.06009v2](https://arxiv.org/pdf/2009.06009v2)

### Inferences
- The finding supports the manuscript's sentence. Two refinements: replace "desktop GPUs" with the actual GPU list; and add in Sec. 7 that a low lock can put the GPU into an idle-like P-state. That gives the L_lo condition a mechanism, but it also means L_lo may not be identical to the driver-chosen slow state.

### Gaps
- I verified content in arXiv v2 only. Page numbers inside the IEEE Access version were not checked.

## Q5. Is Yang et al. 2023 about power only, or also clock sampling? Is applying it to clock telemetry justified?

### Takeaway
The paper is about **power readings only**: power update period, boxcar averaging window, and energy error. Nothing I read covers clock-frequency sampling. Citing it for "clocks ... sampled by a partial sampler" (Table 2 caption, Sec. 10) is not supported. The partial-coverage finding is also GPU-dependent: on the RTX 3090 tested, the power window covered the full update period.

### Cited Findings
- Stated aim: "elucidate the internal mechanisms of the power readings provided by nvidia-smi and assess the accuracy of the power and energy consumption data". — [arXiv abs](https://arxiv.org/abs/2312.02741)
- Power update period: about 20 ms on V100 and 101 ms on A100. Averaging windows: GTX 1080 Ti 10/20 ms, A100 25/100 ms, RTX 3090 100/100 ms. "On H100 and A100, nvidia-smi only reports the average of the past 25ms every 100ms". — [arXiv HTML v3](https://arxiv.org/html/2312.02741v3)
- Published version: SC24, doi 10.1109/SC41406.2024.00028, with the same 25%-sampled finding. — [IEEE Xplore](https://ieeexplore.ieee.org/abstract/document/10793163)

### Inferences
- **Sec. 3:** "nvidia-smi itself observes the GPU only part of the time ... so short kernels can fall between samples". The 50 ms polling interval alone makes short kernels fall between samples; Yang et al. is not needed for that. If Yang is kept, restrict it to power: "nvidia-smi power readings cover only part of each update period on some GPUs".
- **Table 2 caption:** "Clocks are sampled ... which is itself a partial sampler (Yang et al., 2023)". This is misattributed and should be removed or rephrased. The manuscript's tested GPUs (RTX 4050 Laptop, RTX 4090, H100) were not all characterized in what I read.
- **Sec. 9:** "nvidia-smi telemetry itself is sampled only part-time". Accurate only if it says "power telemetry" and names the affected GPUs (A100, H100, Volta, Pascal).
- Cite the SC24 version as the primary record and the arXiv version as the preprint.

### Gaps
- I did not check whether the full paper lists RTX 40-series or H100 power windows in an appendix (it tested more than 70 GPUs). I found no statement in it about clock readings.

## Q6. Does Spaan et al. 2026 state that NVIDIA's DVFS policy is undocumented?

### Takeaway
Yes. The paper says of NVIDIA's automatic DVFS that "its exact workings are not disclosed". This supports the Sec. 5 sentence, but the source is a single preprint.

### Cited Findings
- "For NVIDIA GPUs, its exact workings are not disclosed. Furthermore, DVFS scaling behavior differs between generations and models (Velicka et al., 2025)." — [arXiv HTML v1](https://arxiv.org/html/2601.08539v1)
- No peer-reviewed venue found as of this audit; listed as arXiv only. — [Semantic Scholar](https://api.semanticscholar.org/graph/v1/paper/arXiv:2601.08539?fields=title,venue,year,publicationVenue,externalIds,journal)

### Inferences
- The claim is acceptable as stated. It could be strengthened by also citing Velicka et al. 2025 (cited inside Spaan) for how DVFS behaviour differs across generations, after verifying that paper separately.

### Gaps
- Velicka et al. 2025 was not verified (it is outside this assignment).

## Q7. Do the server-power works support "low-power states and slow governors inflate latency at low load"?

### Takeaway
Mostly yes, but all of them concern CPU servers. Kanev (sleep selection degrades tail latency), Lo (utilization-based `cpufreq` causes latency spikes) and Meisner (idle low-power modes are incompatible with OLDI latency) directly support the sentence. Kasture, Chou and Lo support "motivating latency-aware controllers". Hsu (Adrenaline) is supported by title only.

### Cited Findings
- Kanev: bursty sleep/wake patterns "can cause tail latency degradation or missed potential for power savings of up to 10%". — [Kanev paper page](https://skanev.org/papers/iiswc14ep.html)
- Lo: `cpufreq` "leads to frequent latency spikes for OLDI workloads"; CPU utilization is "a poor choice of control input". At low and medium load there is latency headroom. — [Google Research PDF](https://research.google.com/pubs/archive/42523.pdf)
- Meisner: only "coordinated, full-system active low-power modes" achieve proportionality "with acceptable query latency". — [ACM DL](https://dl.acm.org/doi/10.1145/2000064.2000103)
- Chou: microsecond services make dynamic power management "break down" because idle periods are short and state-transition overheads dominate. — [Semantic Scholar](https://api.semanticscholar.org/graph/v1/paper/DOI:10.1109/HPCA.2019.00032?fields=title,abstract,authors,venue,year)
- Kasture: prior techniques "adapt slowly and conservatively". — [ACM DL](https://dl.acm.org/doi/10.1145/2830772.2830797)

### Inferences
- Suggested edit: "On CPU servers, ..." to make the scope explicit. The manuscript already says "On servers", which is acceptable.
- The cluster is adequate background. None of it shows a GPU effect.

### Gaps
- Adrenaline and Pathania content not verified (see Q1).

## Q8. Corrected BibTeX-ready metadata

### Takeaway
Most entries need only house-style cleanup: lowercase DOIs, and replacing dx.doi.org with doi.org. Material corrections are needed for Yang (SC24), Maliakel (CCGrid 2026), Sinha (add the arXiv ID), and the 2026 preprints (add version and "preprint" status).

### Cited Findings
All fields below were confirmed against the Crossref, USENIX or arXiv records cited in Q1.

```bibtex
@article{adamek2021fft, author={Adamek, Karel and Novotny, Jan and Thiyagalingam, Jeyarajan and Armour, Wesley}, title={Efficiency Near the Edge: Increasing the Energy Efficiency of {FFTs} on {GPUs} for Real-Time Edge Computing}, journal={IEEE Access}, volume={9}, pages={18167--18182}, year={2021}, doi={10.1109/ACCESS.2021.3053409}, note={arXiv:2009.06009}}
@inproceedings{chou2019udpm, author={Chou, Chih-Hsun and Bhuyan, Laxmi N. and Wong, Daniel}, title={{$\mu$DPM}: Dynamic Power Management for the Microsecond Era}, booktitle={2019 IEEE International Symposium on High Performance Computer Architecture (HPCA)}, pages={120--132}, year={2019}, doi={10.1109/HPCA.2019.00032}}
@misc{chu2026operator, author={Chu, Logan and Li, Dong}, title={Hardware-Attributed Operator Profiling for {PyTorch}}, year={2026}, eprint={2609.11938}, archivePrefix={arXiv}, primaryClass={cs.AR}, note={Preprint, v2 (29 Sep 2026); not peer reviewed}, url={https://arxiv.org/abs/2609.11938}}
@inproceedings{chung2024perseus, author={Chung, Jae-Won and Gu, Yile and Jang, Insu and Meng, Luoxi and Bansal, Nikhil and Chowdhury, Mosharaf}, title={Reducing Energy Bloat in Large Model Training}, booktitle={Proceedings of the ACM SIGOPS 30th Symposium on Operating Systems Principles (SOSP '24)}, pages={144--159}, year={2024}, doi={10.1145/3694715.3695970}}
@misc{golden2025prism, author={Golden, Alicia and Kuchnik, Michael and Hsia, Samuel and DeVito, Zachary and Wei, Gu-Yeon and Brooks, David and Wu, Carole-Jean}, title={{PRISM}: Probabilistic Runtime Insights and Scalable Performance Modeling for Large-Scale Distributed Training}, year={2025}, eprint={2510.15596}, archivePrefix={arXiv}, primaryClass={cs.DC}, note={Preprint, v3 (21 Sep 2026)}, url={https://arxiv.org/abs/2510.15596}}
@inproceedings{hsu2015adrenaline, author={Hsu, Chang-Hong and Zhang, Yunqi and Laurenzano, Michael A. and Meisner, David and Wenisch, Thomas F. and Mars, Jason and Tang, Lingjia and Dreslinski, Ronald G.}, title={Adrenaline: Pinpointing and Reining in Tail Queries with Quick Voltage Boosting}, booktitle={2015 IEEE 21st International Symposium on High Performance Computer Architecture (HPCA)}, pages={271--282}, year={2015}, doi={10.1109/HPCA.2015.7056039}}
@inproceedings{kakolyris2025throttllem, author={Kakolyris, Andreas Kosmas and Masouros, Dimosthenis and Vavaroutsos, Petros and Xydis, Sotirios and Soudris, Dimitrios}, title={{throttLL'eM}: Predictive {GPU} Throttling for Energy Efficient {LLM} Inference Serving}, booktitle={2025 IEEE International Symposium on High Performance Computer Architecture (HPCA)}, pages={1363--1378}, year={2025}, doi={10.1109/HPCA61900.2025.00103}}
@inproceedings{kalibera2013rigorous, author={Kalibera, Tomas and Jones, Richard}, title={Rigorous Benchmarking in Reasonable Time}, booktitle={Proceedings of the 2013 International Symposium on Memory Management (ISMM '13)}, pages={63--74}, year={2013}, doi={10.1145/2464157.2464160}}
@inproceedings{kanev2014tradeoffs, author={Kanev, Svilen and Hazelwood, Kim and Wei, Gu-Yeon and Brooks, David}, title={Tradeoffs between Power Management and Tail Latency in Warehouse-Scale Applications}, booktitle={2014 IEEE International Symposium on Workload Characterization (IISWC)}, pages={31--40}, year={2014}, doi={10.1109/IISWC.2014.6983037}}
@inproceedings{kasture2015rubik, author={Kasture, Harshad and Bartolini, Davide B. and Beckmann, Nathan and Sanchez, Daniel}, title={Rubik: Fast Analytical Power Management for Latency-Critical Systems}, booktitle={Proceedings of the 48th International Symposium on Microarchitecture (MICRO-48)}, pages={598--610}, year={2015}, doi={10.1145/2830772.2830797}}
@inproceedings{lin2023gearDVFS, author={Lin, Chengdong and Wang, Kun and Li, Zhenjiang and Pu, Yu}, title={A Workload-Aware {DVFS} Robust to Concurrent Tasks for Mobile Devices}, booktitle={Proceedings of the 29th Annual International Conference on Mobile Computing and Networking (MobiCom '23)}, pages={1--16}, year={2023}, doi={10.1145/3570361.3592524}}
@inproceedings{lo2014pegasus, author={Lo, David and Cheng, Liqun and Govindaraju, Rama and Barroso, Luiz Andr{\'e} and Kozyrakis, Christos}, title={Towards Energy Proportionality for Large-Scale Latency-Critical Workloads}, booktitle={2014 ACM/IEEE 41st International Symposium on Computer Architecture (ISCA)}, pages={301--312}, year={2014}, doi={10.1109/ISCA.2014.6853237}}
@inproceedings{maliakel2026characterizing, author={Maliakel, Paul Joe and Ilager, Shashikant and Brandic, Ivona}, title={Characterizing {LLM} Inference Energy-Performance Tradeoffs Across Workloads and {GPU} Scaling}, booktitle={2026 IEEE/ACM International Symposium on Cluster, Cloud and Internet Computing (CCGrid)}, pages={33--43}, year={2026}, doi={10.1109/CCGrid68966.2026.00013}, note={Preprint: arXiv:2501.08219}}
@inproceedings{maricq2018taming, author={Maricq, Aleksander and Duplyakin, Dmitry and Jimenez, Ivo and Maltzahn, Carlos and Stutsman, Ryan and Ricci, Robert}, title={Taming Performance Variability}, booktitle={13th USENIX Symposium on Operating Systems Design and Implementation (OSDI 18)}, pages={409--425}, year={2018}, isbn={978-1-939133-08-3}, url={https://www.usenix.org/conference/osdi18/presentation/maricq}}
@inproceedings{meisner2011oldi, author={Meisner, David and Sadler, Christopher M. and Barroso, Luiz Andr{\'e} and Weber, Wolf-Dietrich and Wenisch, Thomas F.}, title={Power Management of Online Data-Intensive Services}, booktitle={Proceedings of the 38th Annual International Symposium on Computer Architecture (ISCA '11)}, pages={319--330}, year={2011}, doi={10.1145/2000064.2000103}}
@inproceedings{mytkowicz2009wrong, author={Mytkowicz, Todd and Diwan, Amer and Hauswirth, Matthias and Sweeney, Peter F.}, title={Producing Wrong Data Without Doing Anything Obviously Wrong!}, booktitle={Proceedings of the 14th International Conference on Architectural Support for Programming Languages and Operating Systems (ASPLOS '09)}, pages={265--276}, year={2009}, doi={10.1145/1508244.1508275}}
@misc{oliaro2026fastkernels, author={Oliaro, Gabriele and Fu, Yichao and Jiang, May and Lu, Owen and Wang, Junli and Jia, Zhihao and Zhang, Hao and Rajbhandari, Samyam}, title={{FastKernels}: Benchmarking {GPU} Kernel Generation in Production}, year={2026}, eprint={2605.23215}, archivePrefix={arXiv}, primaryClass={cs.LG}, note={Preprint, v1; author list per abstract page, re-check before submission}, url={https://arxiv.org/abs/2605.23215}}
@inproceedings{pathania2014integrated, author={Pathania, Anuj and Jiao, Qing and Prakash, Alok and Mitra, Tulika}, title={Integrated {CPU-GPU} Power Management for {3D} Mobile Games}, booktitle={Proceedings of the 51st Annual Design Automation Conference (DAC '14)}, pages={1--6}, year={2014}, doi={10.1145/2593069.2593151}}
@inproceedings{qiu2024muserve, author={Qiu, Haoran and Mao, Weichao and Patke, Archit and Cui, Shengkun and Jha, Saurabh and Wang, Chen and Franke, Hubertus and Kalbarczyk, Zbigniew and Ba{\c{s}}ar, Tamer and Iyer, Ravishankar K.}, title={Power-aware Deep Learning Model Serving with {$\mu$-Serve}}, booktitle={2024 USENIX Annual Technical Conference (USENIX ATC 24)}, pages={75--93}, year={2024}, isbn={978-1-939133-41-0}, url={https://www.usenix.org/conference/atc24/presentation/qiu}}
@inproceedings{sinha2022notall, author={Sinha, Prasoon and Guliani, Akhil and Jain, Rutwik and Tran, Brandon and Sinclair, Matthew D. and Venkataraman, Shivaram}, title={Not All {GPUs} Are Created Equal: Characterizing Variability in Large-Scale, Accelerator-Rich Systems}, booktitle={SC22: International Conference for High Performance Computing, Networking, Storage and Analysis}, pages={1--15}, year={2022}, doi={10.1109/SC41404.2022.00070}, note={arXiv:2208.11035}}
@misc{spaan2026kerneldvfs, author={Spaan, Jeffrey and Chen, Kuan-Hsun and Varbanescu, Ana-Lucia}, title={Reducing Compute Waste in {LLMs} through Kernel-Level {DVFS}}, year={2026}, eprint={2601.08539}, archivePrefix={arXiv}, primaryClass={cs.PF}, note={Preprint}, url={https://arxiv.org/abs/2601.08539}}
@inproceedings{stojkovic2025dynamollm, author={Stojkovic, Jovan and Zhang, Chaojie and Goiri, {\'I}{\~n}igo and Torrellas, Josep and Choukse, Esha}, title={{DynamoLLM}: Designing {LLM} Inference Clusters for Performance and Energy Efficiency}, booktitle={2025 IEEE International Symposium on High Performance Computer Architecture (HPCA)}, pages={1348--1362}, year={2025}, doi={10.1109/HPCA61900.2025.00102}}
@inproceedings{tang2019dvfs, author={Tang, Zhenheng and Wang, Yuxin and Wang, Qiang and Chu, Xiaowen}, title={The Impact of {GPU} {DVFS} on the Energy and Performance of Deep Learning: An Empirical Study}, booktitle={Proceedings of the Tenth ACM International Conference on Future Energy Systems (e-Energy '19)}, pages={315--325}, year={2019}, doi={10.1145/3307772.3328315}}
@inproceedings{yang2024accurate, author={Yang, Zeyu and Adamek, Karel and Armour, Wesley}, title={Accurate and Convenient Energy Measurements for {GPUs}: A Detailed Study of {NVIDIA} {GPU}'s Built-In Power Sensor}, booktitle={SC24: International Conference for High Performance Computing, Networking, Storage and Analysis}, pages={1--17}, year={2024}, doi={10.1109/SC41406.2024.00028}, note={Preprint: arXiv:2312.02741, ``Part-time Power Measurements: nvidia-smi's Lack of Attention''}}
@inproceedings{you2023zeus, author={You, Jie and Chung, Jae-Won and Chowdhury, Mosharaf}, title={Zeus: Understanding and Optimizing {GPU} Energy Consumption of {DNN} Training}, booktitle={20th USENIX Symposium on Networked Systems Design and Implementation (NSDI 23)}, pages={119--139}, year={2023}, isbn={978-1-939133-33-5}, url={https://www.usenix.org/conference/nsdi23/presentation/you}}
@misc{zhang2025mobiledvfs, author={Zhang, Zongpu and Dash, Pranab and Hu, Y. Charlie and Xu, Qiang and Li, Jian and Guan, Haibing}, title={Dissecting the Impact of Mobile {DVFS} Governors on {LLM} Inference Performance and Energy Efficiency}, year={2025}, eprint={2507.02135}, archivePrefix={arXiv}, primaryClass={cs.OS}, note={Preprint; see possibly related MLSys 2026 paper ``Rethinking DVFS for Mobile LLMs ... CORE''}, url={https://arxiv.org/abs/2507.02135}}
@misc{nvidia_ncu_profiling_guide, author={{NVIDIA}}, title={Nsight Compute Profiling Guide: Clock Control (Sec. 2.6.2)}, year={2026}, url={https://docs.nvidia.com/nsight-compute/ProfilingGuide/}, note={Accessed 2026-10-25}}
@misc{nvidia_nvbench, author={{NVIDIA}}, title={{NVBench}: {CUDA} Kernel Benchmarking Library}, url={https://github.com/NVIDIA/nvbench}, note={Accessed 2026-10-25}}
```

### Inferences
- `nvidia_ncu_profiling_guide` and `nvidia_nvbench` are proposed **additions** that would serve as the primary sources for clock-locking practice.

### Gaps
- Exact BibTeX venue strings for SC24 and CCGrid 2026 were taken from computer.org/IEEE listings, not from a Crossref record fetch.
