# Related work: GPU power states / DVFS and latency-bound GPU queries

Date: 2026-09-25. BibTeX: `paper/v2/references_new.bib` (35 entries).

Finding to position: a light, work-efficient GPU query path runs in a low P-state (RTX 4050 Laptop: P5, SM 435 MHz, mem 810 MHz) while a heavier O(N) path runs at P0/P3. A concurrent "heater" load raises the P-state and makes the light path about 1.9x faster at median. RTX 4090 Linux end-of-run clocks show the same split.

Verification: every entry below was checked against Crossref (DOI resolves, metadata matches), the arXiv API (ID, title, authors), or the USENIX presentation page. BibTeX came from doi.org content negotiation (Crossref), the USENIX pages, or the arXiv API. None was typed from memory. DBLP blocked automated access with a bot challenge, so it was not used.

## 1. GPU DVFS / P-states for low-utilization or bursty work

| Paper | Venue | Relevance |
|---|---|---|
| Mei, Wang, Chu. A survey and measurement study of GPU DVFS on energy conservation. https://doi.org/10.1016/j.dcan.2016.10.001 | Digital Commun. Netw. 2017 | Standard GPU DVFS survey and measurement study (Fermi/Maxwell). Background citation. |
| Tang, Wang, Wang, Chu. The Impact of GPU DVFS on the Energy and Performance of Deep Learning. https://doi.org/10.1145/3307772.3328315 | e-Energy 2019 | Shows that the benefit of a higher core clock depends on the kernel's utilization. They fixed the clocks; they did not study the governor's own choice. |
| Adámek et al. Efficiency Near the Edge: Energy Efficiency of FFTs on GPUs. https://doi.org/10.1109/ACCESS.2021.3053409 | IEEE Access 2021 | At low locked clocks the GPU falls into an idle-like P-state and execution time jumps. This is the closest published note of "a low P-state starves a kernel". |
| Pathania et al. Integrated CPU-GPU Power Management for 3D Mobile Games. https://doi.org/10.1145/2593069.2593151 | DAC 2014 | Mobile GPU governors mispredict bursty, frame-driven work. |
| Lin et al. (GearDVFS). A Workload-Aware DVFS Robust to Concurrent Tasks for Mobile Devices. https://doi.org/10.1145/3570361.3592524 | MobiCom 2023 | Utilization-driven governors behave differently when tasks co-run. This is the closest analogue of our co-location effect, but on mobile. |
| Velička et al. Methodology for GPU Frequency Switching Latency Measurement. arXiv:2502.20075 | arXiv 2025 | Measures clock-switch latency on NVIDIA GPUs (GH200, A100, Quadro RTX 6000). Relevant to why short, bursty kernels never reach boost clocks. |
| Zhang, Dash et al. Dissecting the Impact of Mobile DVFS Governors on LLM Inference. arXiv:2507.02135 | arXiv 2025 | The CPU, GPU and memory governors are not coordinated, so a GPU waiting on the host gets downclocked. This is the mobile analogue of our host-loop-bound pattern. |

Novelty verdict: that DVFS governors downclock low-utilization GPU work is known in general (mobile governors, FFT note). We found no paper that reports this for a desktop or datacenter NVIDIA GPU running a synchronous query loop. Nor did we find one that shows the resulting rank reversal between light and heavy implementations.

## 2. DVFS for ML training and inference

| Paper | Venue | Relevance |
|---|---|---|
| You, Chung, Chowdhury. Zeus. https://www.usenix.org/conference/nsdi23/presentation/you | NSDI 2023 | Trades GPU power limit against batch size to cut training energy. |
| Chung et al. (Perseus). Reducing Energy Bloat in Large Model Training. https://doi.org/10.1145/3694715.3695970 | SOSP 2024 | Slows the GPUs that are off the critical path. Clocks as a deliberate knob. |
| Stojkovic et al. DynamoLLM. https://doi.org/10.1109/HPCA61900.2025.00102 | HPCA 2025 | Reconfigures GPU frequency, instances and parallelism for inference SLOs. |
| Qiu et al. μ-Serve. https://www.usenix.org/conference/atc24/presentation/qiu | ATC 2024 | Frequency scaling plus model multiplexing for serving. |
| Kakolyris et al. throttLL'eM. https://doi.org/10.1109/HPCA61900.2025.00103 | HPCA 2025 | Predictive GPU throttling under LLM SLOs. |
| Maliakel, Ilager, Brandic. Characterizing LLM Inference Energy-Performance Tradeoffs under GPU DVFS. arXiv:2501.08219 | arXiv 2025 | Memory-bound decode barely responds to SM clock. This contrasts with our case, where the memory clock also drops (810 vs 7001 MHz). |
| Spaan, Chen, Varbanescu. Kernel-Level DVFS. arXiv:2601.08539 | arXiv 2026 | Per-kernel frequency selection, and notes that NVIDIA's policy is undisclosed. |

Novelty verdict: all of these treat frequency as a knob the system chooses, and they lower it to save energy. None reports the default governor under-clocking latency-bound work, which is our inverse problem. We should position our result as "the default policy is the confound", not as a DVFS controller.

## 3. Benchmarking methodology and clock locking

| Source | Relevance |
|---|---|
| Sinha et al. Not All GPUs Are Created Equal. SC22. https://doi.org/10.1109/SC41404.2022.00070 | 8% average (22% max) performance variation from GPU power management across same-SKU GPUs. Studies large clusters under heavy load, not light loads. |
| Golden et al. PRISM. arXiv:2510.15596 | Attributes 1-14% GEMM variation to dynamic GPU frequency. |
| Chu, Li. Hardware-Attributed Operator Profiling for PyTorch. arXiv:2609.11938 | States that an optimized kernel dissipates less power, so it may hold a higher sustained clock and inflate its apparent speedup. That is the power-limited regime and the opposite sign to ours; cite it as the contrast. |
| Oliaro et al. FastKernels. arXiv:2605.23215 | Locks clocks (1593 MHz) for reproducibility. |
| Yang, Adámek, Armour. Part-time Power Measurements: nvidia-smi's Lack of Attention. arXiv:2312.02741 | nvidia-smi power and telemetry sampling is incomplete. This caveats our end-of-run clock readings. |
| Mytkowicz ASPLOS'09; Kalibera & Jones ISMM'13; Maricq OSDI'18 | General measurement-bias and variability canon (verified). |
| Non-archival: NVIDIA nvbench (https://github.com/NVIDIA/nvbench), which offers clock locking; Jan/Menlo "How we (try to) benchmark GPU kernels accurately" (https://www.jan.ai/post/how-we-benchmark-kernels), which says locking to base or max clocks is unrepresentative and cites a GPU MODE lecture where a small shape got slower at a higher clock | Practice, not peer-reviewed. The Jan post has no date on the page. No BibTeX was generated for these. |

Novelty verdict: that unlocked clocks corrupt benchmarks is well established. The direction we observe is new relative to what we found: the more work-efficient kernel is penalized because it runs under the governor's utilization threshold. The prior bias argument (arXiv:2609.11938) says efficient kernels are favored, because they hit the power cap later.

## 4. "Efficient kernel gets lower clocks" / co-location speeds another workload up

We found no peer-reviewed paper that reports either (a) a work-efficient GPU kernel slowed because the governor picks a low P-state, or (b) a co-running GPU load speeding up a latency-bound GPU workload by raising clocks. All the GPU co-location and interference literature we saw reports slowdowns only, e.g. Elvinger et al., SoCC 2025, "Understanding GPU Resource Interference One Level Deeper"; this one was not added to the BibTeX. Nearest neighbours: GearDVFS (mobile concurrency), arXiv:2507.02135 (host-bound GPU gets downclocked), and on CPUs, Kanev IISWC'14.

Novelty verdict: (b) appears novel for GPUs, and (a) appears novel as an explicit, measured claim. Wording should stay cautious ("we are not aware of..."), because vendor forums and blog posts discuss idle downclocking informally.

## 5. CPU analogue: power management versus tail latency at low load

| Paper | Venue | Relevance |
|---|---|---|
| Meisner et al. Power management of online data-intensive services. https://doi.org/10.1145/2000064.2000103 | ISCA 2011 | Canonical: low-power modes hurt latency-critical OLDI services. |
| Kanev, Hazelwood, Wei, Brooks. Tradeoffs between power management and tail latency in warehouse-scale applications. https://doi.org/10.1109/IISWC.2014.6983037 | IISWC 2014 | Closest analogue: at low load, idle and sleep states raise tail latency. |
| Lo et al. (Pegasus). Towards energy proportionality for large-scale latency-critical workloads. https://doi.org/10.1109/ISCA.2014.6853237 | ISCA 2014 | Latency-feedback power control. |
| Kasture et al. Rubik. https://doi.org/10.1145/2830772.2830797 | MICRO 2015 | Fast DVFS for latency-critical systems. |
| Hsu et al. Adrenaline. https://doi.org/10.1109/HPCA.2015.7056039 | HPCA 2015 | Boosts voltage for tail queries. It is HPCA 2015, not 2017; the 2017 version is in ACM TOCS. |
| Chou, Bhuyan, Wong. μDPM. https://doi.org/10.1109/HPCA.2019.00032 | HPCA 2019 | Microsecond-scale power management. |
| Albers, Antoniadis. Race to idle. https://doi.org/10.1145/2556953 | ACM TALG 2014 (SODA 2012) | Theory citation for "race to idle". |

Novelty verdict: this is established on CPUs. Our contribution is to carry it over to GPUs, where the user cannot control the policy on consumer drivers (Windows/Vulkan), and to show that it confounds CPU-vs-GPU routing crossovers.

## Notes on the already-identified citations

- Mytkowicz et al.: ASPLOS 2009 proceedings DOI 10.1145/1508244.1508275, pp. 265-276.
- Kalibera and Jones: ISMM 2013, 10.1145/2464157.2464160.
- Maricq et al.: OSDI 2018, pp. 409-425, USENIX (no DOI).
- Sinha et al.: SC22, 10.1109/SC41404.2022.00070.
- Ngom et al.: DaMoN 2021, 10.1145/3465998.3466009.
- Breß et al.: SIGMOD 2016, 10.1145/2882903.2882936.
- Mageirakos et al.: arXiv:2605.15957 (preprint).
- VecFlow (Xi et al.): arXiv:2506.00812.
- Gregg and Hazelwood: ISPASS 2011, 10.1109/ISPASS.2011.5762730.
- FAISS GPU (Johnson, Douze, Jégou): IEEE TBD, 10.1109/TBDATA.2019.2921572. Crossref gives vol. 7(3), 2021, pp. 535-547; it was online-first in 2019. The key says 2019, but `year` = 2021.
- BibTeX caveat: the Crossref entries contain Unicode (μ, é, ß). They need biblatex/biber, or hand-escaping for classic BibTeX.
