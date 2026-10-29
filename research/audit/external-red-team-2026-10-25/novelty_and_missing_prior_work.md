# Novelty claims and missing prior work: "Power-State Variance in One WGPU Vector-Search Kernel" (Qenlo)

Scope: prior work bearing on the manuscript's novelty/priority statements (Sec. 1, 8, 9): GPU DVFS/power-state effects on latency-bound kernels, benchmarking variability, clock locking, co-location, reporting rules. The other researcher is verifying the manuscript's existing citations; this note lists what is MISSING and assesses each priority claim. Date of audit: 2026-10-25.

Verification legend used below:
- [opened] = page/PDF fetched and the quoted content read directly.
- [excerpt] = content read from the official page's own text as returned by a search tool (title/venue/abstract text came from the publisher/official page), but the full document was not fetched.
- Type tags: PR = peer-reviewed; PP = preprint (not peer-reviewed); DOC = vendor documentation; BLOG = blog/industry post; FORUM = forum/GitHub/Q&A.

Search breadth (for the report's "first" assessment): ~45 search/fetch calls across general web search (two engines), NVIDIA developer forums, NVIDIA docs (TensorRT, Nsight Compute, nvidia-smi, CUPTI), NVIDIA technical blog and GeForce pages, GitHub (nvbench, triton, llama.cpp, frigate, onnxruntime, pytorch), PyTorch forums, Stack Overflow, arXiv, ACM DL, IEEE CSDL/Xplore, USENIX, ScienceDirect, MLSys proceedings. Query families: "GPU clocks drop between kernels", "kernel slower after idle", "keep GPU busy / dummy kernel / keep-alive", "persistence mode", "lock GPU clocks benchmarking", "co-located/concurrent workload faster/raises clock", "background load lowers latency", "default governor low utilization latency", "small batch / batch-1 DVFS latency", "frequency switching latency", "cloud GPU variability", "GPU co-location interference", "WebGPU/Vulkan timestamp variance", "registered reports / artifact badging". Not searched: Google Scholar citation graphs (no access), paywalled full texts of ASPLOS/OSDI co-location papers beyond abstracts, non-English literature, AMD/Intel developer forums in depth, patents beyond one hit. A "first" claim can therefore be qualified as "to our search" but not asserted absolutely.

---

## Q1. Prior reports that GPU idle/low-utilization power states inflate latency of short/bursty/latency-bound GPU work, and that extra (background/dummy) GPU load speeds it up

### Takeaway
The core phenomenon (default GPU power management under-clocks light/bursty GPU work; latency can rise 2-3x; keeping the GPU busy with dummy work or locking clocks removes it) is well documented in vendor documentation, NVIDIA forum answers by NVIDIA staff/experts since 2017, an NVIDIA product feature (Reflex "Low Latency Boost"), and peer-reviewed mobile-GPU work (MLSys 2026). A deliberate keep-alive/dummy kernel as a mitigation is folklore-level known. What I did not find is a peer-reviewed controlled experiment in which a *separate, unrelated process's* light GPU load is used as an intervention and shown to speed up a foreground latency-bound kernel's device time; the closest documented analogue is shared clock domains under MIG co-location (Patel et al., IEEE CAL 2023), where a co-runner forces a light workload's partition from 0.8 GHz to 1.4 GHz.

### Cited Findings
- FORUM [opened] NVIDIA Developer Forums, "Avoiding performance mode slowdown" (Oct 13-18, 2017): a short CUDA program ran ~100 µs when re-run soon versus ~300 µs after the GPU dropped from P2 to P5/P8 after ~20 s inactivity; user reports fixing it by "regularly having it do some non-trivial work, e.g. every 10 seconds"; njuffa: "GPUs are designed as throughput devices, not low-latency devices... It seems your use case requires the GPU to act as a low-latency device" and notes nvidia-smi's limited temporal granularity may miss brief P0 excursions. — [Forum 54181](https://forums.developer.nvidia.com/t/avoiding-performance-mode-slowdown/54181); same question on [Stack Overflow 46731101](https://stackoverflow.com/questions/46731101/avoiding-performance-state-slowdown) [excerpt]
- FORUM [opened] "Why kernel calculate speed got slower after waiting for a while?" (Jul 18-19, 2022): kernel 200 ms when run continuously vs 400 ms for the first run after a 10 s wait; njuffa describes ~300 MHz GUI-only, ~1000 MHz "standby for compute", boost only once a kernel runs, and "if an app sends relatively short running kernels in fairly large time intervals, the GPU clock never rises above 1000 MHz"; reports up to 20% variation of the same kernel across instances of the same GPU model; Robert_Crovella (NVIDIA) points to performance states; the user's fix: launching a "keepBusyKernel" every 500 ms "the speed of GPU became steady." — [Forum 221059](https://forums.developer.nvidia.com/t/why-kernel-calculate-speed-got-slower-after-waiting-for-a-while/221059)
- FORUM [opened] "Why warm-up?" (thread from 2017, reply Feb 2025): a 10 s activity gap sends the GPU "back into deep idle mode, where compute cores and memory are clocked down to a very low clock (e.g. 300 MHz)"; suggested workarounds: lock clocks with nvidia-smi "or by feeding the GPU a bunch of dummy kernels during a lull in activity to prevent idle mode from kicking in", or "prime the pump" with a dummy kernel 0.1-1 s before real work; idle draw rises from single-digit W to 50-60 W. — [Forum 48565](https://forums.developer.nvidia.com/t/why-warm-up/48565)
- FORUM [opened] "CUDA performance get slower after sleep in host side" (Nov 2022): sensor pipeline at 20 fps runs 5 ms -> 25 ms per frame after ~10 s, not at 120 fps; njuffa: idling between frames may make power management reduce frequency; "your best bet... is to keep it busy, for example by sending dummy data frames"; nvidia-smi -lgc keeps a Quadro RTX 4000 at 1395 MHz while idle at 37 W. — [Forum 232943](https://forums.developer.nvidia.com/t/cuda-performance-get-slower-after-sleep-in-host-side/232943)
- DOC/PRODUCT [excerpt, official NVIDIA page] NVIDIA Reflex (Sep 1, 2020): "Low Latency Boost... overrides the power saving features in the GPU to allow the GPU clocks to stay high when heavily CPU-bound... can reduce latency slightly when the GPU is significantly underutilized." — [NVIDIA GeForce News](https://www.nvidia.com/en-us/geforce/news/reflex-low-latency-platform/)
- DOC [opened] NVIDIA TensorRT, "Hardware/Software Environment for Performance Measurements": "By default, the GPU clock frequency is floating, meaning it sits idle when there is no active workload and boosts... when the workload starts"; floating clocks/throttling "can lead to more non-determinism in tactic selections and unstable performance measurements"; gaps between inferences change average power and hence clocks, which skews benchmark numbers (power-throttled regime); recommends logging clocks with `nvidia-smi dmon -s pcu` while the workload runs. — [TensorRT docs](https://docs.nvidia.com/deeplearning/tensorrt/10.x.x/performance/hw-sw-environment.html)
- PR [excerpt, official proceedings page + PDF snippets] Zhang, Dash, Xu, Hu, Li, Guan, "Rethinking DVFS for Mobile LLMs: Unified Energy-Aware Scheduling with CORE", MLSys 2026: default mobile governors yield 23.0-40.4% longer latency; the GPU governor chooses low frequency because decode has low GPU utilization; pinning GPU higher cut decode TPOT by 34.6-41.0%. (Manuscript cites only the 2025 arXiv precursor, arXiv:2507.02135; should cite the MLSys version.) — [MLSys 2026 proceedings](https://proceedings.mlsys.org/paper_files/paper/2026/hash/136b9a13861308c8948cd308ccd02658-Abstract-Conference.html); [arXiv 2507.02135](https://arxiv.org/abs/2507.02135)
- PR [opened] Patel, Gong, Rizvi, Choukse, Misra, Anderson, Sriraman, "Towards Improved Power Management in Cloud GPUs", IEEE Computer Architecture Letters 2023, DOI 10.1109/LCA.2023.3278652: "All MIG partitions share the same GPU clocks"; spmv alone runs at 0.8 GHz, but co-located with vgg16-training "the spmv partition is forced to operate at 1.4GHz"; also: on A100, DVFS "continuously runs at maximum frequency after Triton is loaded", RTX 6000 stays at max ~20 s after use then drops; "DVFS closely matches the highest frequency configuration" for their (sustained) workloads. — [CAL 2023 PDF](https://foci.uw.edu/papers/cal23-gpupower.pdf)
- FORUM [excerpt] Frigate discussion (Jetson, 2026): YOLOv9-t 320 ~6 ms with jetson_clocks vs ~15 ms with default DVFS; "DVFS makes inference ~2x slower, probably because it doesn't ramp quickly enough for bursty detection." — [GitHub frigate #23388](https://github.com/blakeblackshear/frigate/discussions/23388)
- PP/PR [opened] Velicka, Vysocky, Yasal, Riha, "An in-depth study of GPU frequency-scaling latency and its optimization on modern architectures", Future Generation Computer Systems 179 (June 2026) 108331, DOI 10.1016/j.future.2025.108331 (extends HCW@IPDPS 2025 paper; arXiv:2502.20075): GPU frequency transition latency from hundreds of µs to hundreds of ms; GH200 switching 5.6-477 ms. Relevant to why sub-ms kernels can complete before a clock ramp. — [FGCS](https://www.sciencedirect.com/science/article/pii/S0167739X25006259); [arXiv](https://arxiv.org/abs/2502.20075)
- TECH REPORT [opened] Volkov, "Understanding Latency Hiding on GPUs", UC Berkeley EECS-2016-143 (PhD thesis): to keep DVFS out of scope, a monitor thread polls clocks every 10 µs (NVAPI); "To increase GPU processor clock rate to maximum, we execute an arithmetically intensive kernel" and measurements are redone if clocks were not at maximum. An early documented use of a load-raising kernel as a measurement control. — [EECS-2016-143](https://www2.eecs.berkeley.edu/Pubs/TechRpts/2016/Archive/EECS-2016-143.pdf)
- FORUM [opened] PyTorch forums (2024, 2025): warmups are needed "as you could otherwise profile the startup time after the device was set to IDLE and reduced its power usage"; NVIDIA engineer reply: idling between bursts makes timing vary; Nsight Compute locks clocks to base by default to avoid variance. — [PyTorch 204519](https://discuss.pytorch.org/t/are-there-any-reasons-why-running-gpu-inference-in-a-thread-would-be-slower/204519); [PyTorch 223766](https://discuss.pytorch.org/t/how-to-get-stable-torch-cuda-event-timings-for-reliable-benchmarking/223766)
- FORUM [excerpt] ONNX Runtime issue #23101 (Dec 2024): a single inference raises GPU/memory clocks to near max; clocks stay up ~18 s then drop; observed with one inference every 20 s. — [onnxruntime #23101](https://github.com/microsoft/onnxruntime/issues/23101)
- FORUM [excerpt] TensorRT forum (latency rises and jitter "explodes" if last inference >= 15 s ago, even with clocks fixed at max) — shows idle-gap effects not fully removed by clock locking on that system. — [NVIDIA forum 237501](https://forums.developer.nvidia.com/t/strange-cnn-inference-latency-behavior-with-cuda-and-tensorrt/237501)
- CPU analogues (PR, manuscript already cites Kanev but not these specifics): Kanev et al., IISWC 2014 [opened]: at low load (100 QPS) deep C-states cause "an overall 2.4x increase in request latency compared to the high-load (10,000 QPS) case. Such latency degradation is unintuitive"; also found busy-spinning in C0 had worse latency than shallow C-states (a caution for keep-alive designs). Li, Sharma, Ports, Gribble, "Tales of the Tail", SoCC 2014 [opened tech-report version]: power saving "cause a counter-intuitive effect... low-utilization systems... have higher tail latency" at 10% utilization; disabling C-states removes the highest-latency tail. — [Kanev IISWC14](https://skanev.org/papers/iiswc14ep.pdf); [Li et al. TR](https://homes.cs.washington.edu/~drkp/papers/latency-tr14.pdf)

### Inferences
- "GPUs under-clock light/bursty latency-bound work, and keeping the GPU busy (dummy kernels) or locking clocks fixes it" is established practitioner knowledge with vendor endorsement (NVIDIA staff/expert forum answers, TensorRT docs, Reflex Boost). The manuscript's intro concedes kernel-benchmarking practice knows this, but Section 9 omits all of these primary sources and the CPU-side "more load -> lower latency" literature that is the direct conceptual precedent for the heater.
- The heater experiment's specific design (separate process, few-percent utilization, used as an intervention with a heavy-path negative control, device timestamps) is not reported in what I found; but its mechanism and predicted sign are anticipated by the forum keep-busy reports (same-process), Volkov's methodology (same-process), and Patel et al.'s MIG shared-clock observation (separate workloads, but framed as wasted power, not as speed-up, and frequency not latency was reported).
- Patel et al. report A100 holding max frequency once a CUDA context/Triton is loaded; Qenlo observed slow modes on H100 under Vulkan. The low-clock behavior is driver/API/GPU dependent; the manuscript should not generalize to CUDA datacenter deployments (its Limitations note "CUDA runtimes may be managed differently" should cite this).

### Gaps
- No peer-reviewed paper found that measures an unrelated co-running process speeding up a latency-bound GPU kernel's device time. Absence is "to our search" only.
- Could not find MLPerf Inference rules text that specifically governs GPU clock locking; MLPerf Power rules require documenting "Power Management Settings" ([MLCommons power_measurement.adoc](https://github.com/mlcommons/inference_policies/blob/master/power_measurement.adoc) [excerpt]), but I did not verify whether clock locking is allowed/forbidden in performance submissions.
- Did not find HFT/audio/robotics primary sources describing spin kernels to hold clocks (only generic persistent-kernel material and Jetson "jetson_clocks" practice).

---

## Q2. GPU benchmarking methodology: clock locking, throttle detection, warm-up, telemetry, variability baselines

### Takeaway
Clock locking, throttle detection, continuous clock logging, warm-up and repetition across processes/machines are standard and documented in NVIDIA tooling (nvbench, Nsight Compute, TensorRT, CUPTI), Triton's testing utilities, and methodology papers (Hoefler & Belli SC'15). Published same-model variability under sustained load is typically single-digit to tens of percent (up to 38% across cloud devices), supporting the manuscript's contrast, but those studies measure a different regime (saturating kernels, device-to-device), not idle-state latency inflation.

### Cited Findings
- DOC [opened] NVIDIA nvbench CLI: `--lock-gpu-clocks base|max` ("base... best for stable results"; "max... best for fastest results"), persistence mode, and throttle detection with `--throttle-threshold` (default 75% of default clock) and `--throttle-recovery-delay` (0.05 s). — [nvbench cli_help.md](https://github.com/NVIDIA/nvbench/blob/main/docs/cli_help.md)
- DOC [opened] Nsight Compute Profiling Guide, "Clock Control": "if a kernel instance is profiled that has prior kernel executions in the application, the GPU might already be in a higher clocked state and the measured kernel duration... will be affected. Likewise, if a kernel instance is the first kernel... GPU clocks will regularly be lower"; Nsight Compute locks clocks by default; Nsight Systems does not; recommends external nvidia-smi locking and `--clock-control none` when comparing. — [Nsight Compute guide](https://docs.nvidia.com/nsight-compute/ProfilingGuide/index.html)
- BLOG (NVIDIA) [opened] Bavoil & Cantlay, "Advanced API Performance: SetStablePowerState" (Jun 28, 2022): clock changes "introduc[e] errors in measurements and render[] comparisons between runs difficult"; use `nvidia-smi --lock-gpu-clocks/--lock-memory-clocks`; D3D12 SetStablePowerState does not lock memory clock. — [NVIDIA blog](https://developer.nvidia.com/blog/advanced-api-performance-setstablepowerstate)
- CODE [opened] Triton `triton.testing.set_gpu_clock` locks SM and memory clocks via nvidia-smi and asserts they hold; do_bench has warm-up. Triton issue #2306 (2023): default 25 ms warm-up gave 11.23 ms vs 8.79 ms with 500 ms warm-up (~30%) on RTX 2060 SUPER, reproduced on A100 (cause not formally attributed in thread excerpt, consistent with ramp-up). — [triton testing.py](https://github.com/triton-lang/triton/blob/main/python/triton/testing.py); [triton #2306](https://github.com/triton-lang/triton/issues/2306)
- BLOG [opened] Standard Kernel, "This Kernel Was Faster Yesterday" (Feb 24, 2026): GPU contention +21% mean, 30x std; clock locking not guaranteed under power cap; inserting 500 ms sleep between trials changed H100 matmul mean from 12.36 ms to 11.04 ms (power-limited regime, opposite sign to Qenlo's); cloud providers may refuse `nvidia-smi -pl`/`-rac` ("not supported in current scope"), matching Qenlo's inability to lock clocks on pods. — [standardkernel.com](https://standardkernel.com/blog/in-pursuit-of-high-fidelity-gpu-kernel-benchmarking/)
- PR [excerpt, ACM DL] Hoefler & Belli, "Scientific Benchmarking of Parallel Computing Systems: Twelve ways to tell the masses when reporting performance results", SC '15, Article 73, DOI 10.1145/2807591.2807644: survey found only 15 of 95 papers mention variance and 2 report CIs; rules on reporting nondeterministic results with CIs. Not cited by the manuscript. — [ACM DL](https://dl.acm.org/doi/10.1145/2807591.2807644)
- PR [opened] Slynko et al. (Silicon Data), "Did You Win the GPU Cloud Lottery? Benchmarking from TFLOPS to Tokens/$", GPGPU '26, pp. 15-22, DOI 10.1145/3818671.3818674: 3,500+ GPUs, 11 providers; execution-level variation <9%, up to 38% across devices/providers for the same model; driver/OS <1% of variance; LLM TPS range up to 53% under one workload. — [ACM DL](https://dl.acm.org/doi/full/10.1145/3818671.3818674)
- PR [opened abstract] Mogilevsky, Zaky, Sun, Zheng, Yang, Zhang, "Quantifying Performance Variability in GPU Clusters", IEEE TPDS 37(6):1400-1411, June 2026, DOI 10.1109/TPDS.2026.3684387: GEMM variability 0.1-8.8% (outliers higher), applications 1.0-12.2%. — [IEEE CSDL](https://www.computer.org/csdl/journal/td/2026/06/11481967/2fHHzPybVuM)
- PR [opened abstract] Coplin & Burtscher, "Effects of source-code optimizations on GPU performance and energy consumption", GPGPU-8 (2015), pp. 48-58, DOI 10.1145/2716282.2716292: 128 versions of two n-body codes across clock frequencies; optimizations alter performance and energy by up to 5x; some reduce power draw. — [ACM DL](https://dl.acm.org/doi/10.1145/2716282.2716292)
- PR [excerpt] GPU DVFS characterization background not cited by the manuscript: Mei, Wang, Chu, "A survey and measurement study of GPU DVFS on energy conservation", Digital Communications and Networks 3(2):89-100, 2017, DOI 10.1016/j.dcan.2016.10.001 ([HIT record](https://scholar.hit.edu.cn/en/publications/a-survey-and-measurement-study-of-gpu-dvfs-on-energy-conservation)); Ge et al., "Effects of DVFS on a K20 GPU", ICPP 2013 ([CSDL](https://www.computer.org/csdl/proceedings-article/icpp/2013/5117a826/12OmNyFU74p)); Abe et al., "Power and Performance Characterization and Modeling of GPU-Accelerated Systems", IPDPS 2014 pp. 113-122 ([CSDL TOC](https://www.computer.org/csdl/proceedings/2014/ipdps/12OmNxjjEcj)); Fan, Cosenza, Juurlink, "Predictable GPUs Frequency Scaling for Energy and Performance", ICPP 2019, DOI 10.1145/3337821.3337833 ([ACM DL](https://dl.acm.org/doi/10.1145/3337821.3337833)); Guerreiro, Ilic, Roma, Tomás, "DVFS-aware application classification to improve GPGPUs energy efficiency", Parallel Computing (2019) ([ScienceDirect](https://www.sciencedirect.com/science/article/pii/S0167819118300243)). All study energy/performance under fixed frequencies for sustained kernels; none addresses idle-state latency of light kernels.
- PR [excerpt] Burtscher, Zecena, Zong, "Measuring GPU Power with the K20 Built-in Sensor", GPGPU-7 (2014): naive averaging of sensor samples can give large errors (sampling/lag issues) — a precedent for the manuscript's telemetry caveat alongside Yang et al. 2023. — [PDF](https://userweb.cs.txstate.edu/~mb92/papers/gpgpu14.pdf)
- PR/PP: WebGPU timestamp caveats — WebGPU spec permits timestamp counters to be invalid across "a GPU clock or power-state change" (secondary source [excerpt](https://www.spatialvisualization.org/performance-tuning-profiling-webgpu-spatial/frame-profiling-with-timestamp-queries)); a 2024 Innsbruck thesis found timestamp-query results in wgpu/Dawn "consistency got even worse" with large run-to-run variance ([thesis PDF](https://ulb-dok.uibk.ac.at/ulbtirolhs/download/pdf/11304364) [excerpt]). Not peer-reviewed; relevant to the manuscript's reliance on WGPU timestamp queries.

### Inferences
- The manuscript's Section 9 "kernel benchmarking practice therefore often locks clocks (Oliaro et al., 2026; Chu and Li, 2026)" cites two 2026 preprints for a practice that is documented by NVIDIA tools for years (nvbench, Nsight Compute, TensorRT, SetStablePowerState blog) and Triton's utilities. These primary sources should replace or supplement the preprints.
- The "tens of percent" comparison set (Sinha 2022; Golden 2025; plus missing Slynko GPGPU'26 and Mogilevsky TPDS'26) concerns same-model device variability under saturating load, a different phenomenon from power-state-induced latency inflation of light kernels. Comparing 4.6x to those numbers overstates novelty; the apt comparators are idle-state latency reports (2-3x in forum reports; 2.4x on CPU in Kanev 2014; ~2x Jetson; 34-41% mobile LLM decode).

### Gaps
- Did not fetch Hoefler & Belli full text to check whether frequency/power-state reporting is an explicit rule (I recall it requires documenting the environment, but did not verify wording).
- No peer-reviewed GPU benchmarking-pitfalls paper found that specifically quantifies idle-state effects on sub-ms kernels.

---

## Q3. Does DVFS confound work-efficiency comparisons (less efficient implementation looks relatively better)?

### Takeaway
The general point that clock state changes measured speedups between implementations is documented: TensorRT docs say floating clocks cause non-determinism in tactic selection (i.e., choosing among implementations of the same operation by timing), Nsight Compute says a kernel's measured duration depends on whether preceding kernels raised the clock, and a Sept 2026 preprint reports that idling between timing trials let clocks drop and caused a spurious retraction of a real kernel speedup. I found no prior work making the specific directional argument that the work-efficient path is systematically penalized because it induces a lower power state while the wasteful one holds boost; that framing appears new, but the manuscript's "we have not found a report" sentence should acknowledge the adjacent sources.

### Cited Findings
- DOC [opened] TensorRT: floating clocks or throttling "can lead to more non-determinism in tactic selections and unstable performance measurements across inferences because every CUDA kernel can run at slightly different clock frequencies." — [TensorRT docs](https://docs.nvidia.com/deeplearning/tensorrt/10.x.x/performance/hw-sw-environment.html)
- DOC [opened] Nsight Compute: prior kernel executions can leave the GPU "in a higher clocked state and the measured kernel duration... will be affected"; replay passes later in sequence see higher clocks. — [Nsight Compute guide](https://docs.nvidia.com/nsight-compute/ProfilingGuide/index.html)
- PP [opened] Agarwal, Garg, Singhal, "How Much of a Real Workload Can LLM-Generated GPU Kernels Actually Reach?", arXiv:2609.21058 (Sep 17, 2026): "timing loops must not be hand-rolled: a custom loop that synchronised and idled between trials allowed GPU clocks to drop and caused a spurious retraction of a real result"; three claimed speedups demoted on clean re-timing. — [arXiv](https://arxiv.org/abs/2609.21058); [HTML](https://arxiv.org/html/2609.21058v1)
- BLOG [opened] Standard Kernel (2026): "measurement noise can look like real gains and mislead RL into reinforcing changes"; power-limited heavy kernels run faster with idle gaps (11%), i.e., duty cycle changes the clock and thus the measured time. — [standardkernel.com](https://standardkernel.com/blog/in-pursuit-of-high-fidelity-gpu-kernel-benchmarking/)
- PR [excerpt] Mobile analogue: CORE (MLSys 2026) shows the GPU governor runs low-utilization decode at low clocks, penalizing the low-utilization phase's latency. — [MLSys 2026](https://proceedings.mlsys.org/paper_files/paper/2026/hash/136b9a13861308c8948cd308ccd02658-Abstract-Conference.html)
- PR [excerpt] CPU analogue on frequency and comparisons: Kanev et al. disabled Turbo Boost because "the additional frequency headroom is heavily dependent on idle power management... Enabling TurboBoost can bring significant additional variance." — [Kanev IISWC14](https://skanev.org/papers/iiswc14ep.pdf) [opened]; LeSueur & Heiser, HotPower 2010 on DVFS diminishing returns ([PDF](https://www.usenix.org/legacy/events/hotpower10/tech/full_papers/LeSueur.pdf) [excerpt]).

### Inferences
- Priority claim 1 ("We have not found a report that this changes the measured margin between two implementations of the same GPU operation") is too strong as worded: vendor docs (TensorRT tactic selection) and a recent preprint state or show exactly that clock state changes measured margins between implementations. The defensible novelty is the controlled, directional demonstration that the *work-efficient* variant is the one penalized because its own light load induces the low state.

### Gaps
- No peer-reviewed paper found showing a GPU case where a less efficient implementation's relative standing improves due to induced power state; also no CPU paper found explicitly showing turbo/C-states inverting a work-efficiency comparison (searched; not found).

---

## Q4. Co-location / interference on GPUs

### Takeaway
The GPU co-location literature (Baymax, Prophet, REEF, Orion, Strati et al., Holoscan, MPS/MIG studies) uniformly reports interference slowdowns and tail inflation; "isolated benchmarks misstate co-located latency" is therefore well established in the slowdown direction. The opposite-sign effect (co-runner speeds up a light workload via shared clocks) appears only indirectly (Patel et al. 2023 MIG shared clock domain), framed as power waste.

### Cited Findings
- PR [excerpt, ACM DL] Chen et al., "Baymax: QoS Awareness and Increased Utilization for Non-Preemptive Accelerators in Warehouse Scale Computers", ASPLOS '16, DOI 10.1145/2872362.2872368 (K40 GPU; interference on non-preemptive accelerators differs from multicore contention). — [ACM DL](https://dl.acm.org/doi/10.1145/2872362.2872368)
- PR [excerpt, author PDF] Chen et al., "Prophet: Precise QoS Prediction on Non-Preemptive Accelerators to Improve Utilization in Warehouse-Scale Computers", ASPLOS '17, DOI 10.1145/3037697.3037700: "We measured up to 60x tail latency degradation due to interference"; PCIe bandwidth contention a key factor. — [PDF](https://ramsrivatsa.github.io/papers/p17-chen.pdf)
- PR [excerpt, USENIX] Han, Zhang, Chen, Chen, "Microsecond-scale Preemption for Concurrent GPU-accelerated DNN Inferences" (REEF), OSDI '22: real-time task latency ~4 ms alone, tail latency increases "by over an order" under concurrent best-effort load; REEF <2% overhead, up to 7.7x throughput. Extended version in ACM TOCS 2025 (DOI 10.1145/3768622). — [USENIX](https://www.usenix.org/conference/osdi22/presentation/han); [TOCS](https://dl.acm.org/doi/10.1145/3768622)
- PR [excerpt, ACM DL] Strati, Ma, Klimovic, "Orion: Interference-aware, Fine-grained GPU Sharing for ML Applications", EuroSys '24, pp. 1075-1092, DOI 10.1145/3627703.3629578. — [ACM DL](https://dl.acm.org/doi/10.1145/3627703.3629578)
- PP [excerpt] "Understanding/Measuring GPU Resource Interference One Level Deeper", arXiv:2501.16909: schedulers consider only a subset of utilization metrics; colocation can give >1.87x speedup when pipeline utilization is below 100%, or large slowdowns otherwise. — [arXiv](https://arxiv.org/html/2501.16909)
- PP [opened abstract] Sinha, Dwivedi, Azizian, "Towards Deterministic End-to-end Latency for Medical AI Systems in NVIDIA Holoscan", arXiv:2402.04466 (2024): concurrent AI apps cause unpredictable latency "primarily due to GPU resource contentions"; MPS + GPU isolation cut max latency 21-35%. — [arXiv](https://arxiv.org/abs/2402.04466)
- PR [opened] Patel et al., IEEE CAL 2023: MIG partitions share clocks; a light partition is forced to the co-runner's frequency (0.8 -> 1.4 GHz). — [PDF](https://foci.uw.edu/papers/cal23-gpupower.pdf)
- PP [opened abstract] "Edge-Inference Governors Need Memory-Clock State", arXiv:2606.16106 (Jun 2026): on Jetson Orin, EMC (memory clock)-blind latency models miss 25-28% of deadlines; GPU co-tenancy opens queueing tails; tenancy-blind admission misses 32% of tokens. Relevant to the manuscript's E-SPLIT memory-vs-SM finding and to co-tenancy. — [arXiv](https://arxiv.org/abs/2606.16106)

### Inferences
- The manuscript's "Isolated benchmarks may misstate co-located latency" paragraph has no co-location citations; it should cite Baymax/Prophet/REEF/Orion for the slowdown direction and Patel et al. (shared clock domain) as the nearest precedent for a speed-up direction, and state that its sign-reversal claim is untested with real co-runners (which it does).

### Gaps
- Did not access full texts of Baymax/Orion to check whether any reported anomalous speed-ups under co-location attributed to clocks. GSLICE and Laius were not verified in this pass.

---

## Q5. Vector-search-specific prior work (batch-1 GPU retrieval latency, power states)

### Takeaway
GPU vector search at batch 1 / small batch is studied (CAGRA, cuVS single-query latency, ALGAS, Milvus batching thresholds), and vendors offer persistent kernels for one-at-a-time latency, but I found no report attributing GPU vector-search latency variance to power states. The manuscript's vector-search-specific observation appears unreported, with the caveat that persistent-kernel designs (which keep the GPU busy) are an existing mitigation that also sidesteps the effect.

### Cited Findings
- DOC [excerpt] cuVS CAGRA C API: optional `persistent` kernel with `persistent_lifetime` and `persistent_device_usage`; "running any other work on GPU alongside with the persistent kernel makes the setup fragile." — [cuVS docs](https://docs.rapids.ai/api/cuvs/legacy/c_api/neighbors_cagra_c)
- BLOG (NVIDIA) [excerpt] cuVS (Jul 24, 2025): dynamic batching improves online search latency up to 10x; "CAGRA persistent search" for "high-volume online search... while maintaining low one-at-a-time latencies." — [NVIDIA blog](https://developer.nvidia.com/blog/optimizing-vector-search-for-indexing-and-real-time-retrieval-with-nvidia-cuvs)
- PR [excerpt] ALGAS: "A Low-Latency GPU-Based Approximate Nearest Neighbor Search System", IPDPS 2025, pp. 199-209: small-batch GPU ANN with dynamic batching on a persistent GPU kernel; 21.9-35.4% lower latency. — [IEEE CSDL](https://www.computer.org/csdl/proceedings-article/ipdps/2025/323700a199/28Ai78Sr96w)
- PR [excerpt] Wang et al., "Milvus", SIGMOD 2021: "GPU outperforms CPU only if the query batch size is large enough"; GPU used if batch > threshold (e.g., 1000). — [PDF](https://www.cs.purdue.edu/homes/csjgwang/pubs/SIGMOD21_Milvus.pdf)
- PP [excerpt] Mageirakos et al., "To GPU or Not to GPU: Vector Search in Relational Engines", arXiv:2605.15957 (May 15, 2026) — already cited by manuscript; not peer-reviewed as of this search.

### Inferences
- Novelty for "power-state variance in a GPU vector-search kernel" is plausible to our search; but it is a domain instance of the general phenomenon, and the paper already frames it as one engine. Persistent-kernel and dynamic-batching literature should be cited as deployment mitigations that change the GPU's load state.

### Gaps
- No search of FAISS GitHub issues for clock-related variance at small batch.

---

## Q6. Reporting rules and reproducibility standards

### Takeaway
Rules (i)-(iv-a) restate established guidance (Kalibera & Jones on hierarchical repetition including process level; Hoefler & Belli on variability reporting; TensorRT on continuous clock logging and lock/float; nvbench/Nsight on locking and throttle detection). Rule (iv-b) "load-raising control plus heavy negative control" and rule (v) "state the power state of compared implementations" are not found as stated rules in the sources searched, though load-raising kernels as measurement aids exist (Volkov 2016). Preregistration in software research has an established track (registered reports since MSR 2020).

### Cited Findings
- DOC [opened] TensorRT: run `nvidia-smi dmon -s pcu` in parallel to log clocks/power/temperature/utilization; choose locked vs floating clocks depending on goal; report both caveats. — [TensorRT docs](https://docs.nvidia.com/deeplearning/tensorrt/10.x.x/performance/hw-sw-environment.html)
- DOC [opened] nvbench throttle detection; Nsight Compute clock control (see Q2).
- PR [excerpt] Hoefler & Belli SC'15 (see Q2).
- PR [excerpt, PubMed/EMSE] Ernst & Baldassarre, "Registered reports in software engineering", Empirical Software Engineering 28(2):55, 2023, DOI 10.1007/s10664-022-10277-5: registered reports introduced at MSR in 2020, adopted at three conferences and two journals. — [PubMed](https://pubmed.ncbi.nlm.nih.gov/36937703); [arXiv 2302.03649](https://arxiv.org/abs/2302.03649)
- DOC [excerpt] ACM Artifact Review and Badging v1.1 (Aug 24, 2020): Artifacts Available; Artifacts Evaluated (Functional/Reusable); Results Validated (Reproduced/Replicated). — [ACM](https://www.acm.org/publications/policies/artifact-review-and-badging-current)
- DOC [excerpt] ACM SIGMOD Availability & Reproducibility Initiative (since SIGMOD 2015 reproducibility track). — [SIGMOD ARI](http://reproducibility.sigmod.org/index.html); [history](https://reproducibility.sigmod.org/history.html)
- DOC [excerpt] AMD SMI "performance determinism" and STABLE_STD/STABLE_PEAK performance levels: vendor mechanism for stable clocks on AMD GPUs, framed around "benchmarking inaccuracy – results vary between runs." — [ROCm docs](https://rocm.docs.amd.com/projects/amdsmi/en/develop/conceptual/perf-determinism.html)

### Inferences
- The manuscript's "pre-registered" language for E-LOCK/E-SPLIT (commit before data) is a self-timestamped protocol, not a registered report reviewed in advance; citing Ernst & Baldassarre and using "preregistered in a public repository (not peer-reviewed in advance)" would be accurate.
- Section 8's rules should cite TensorRT/nvbench/Nsight/Hoefler & Belli/Kalibera & Jones for (i)-(iv-a) and claim novelty only for (iv-b) and (v).

### Gaps
- Did not verify whether any SIGMOD/VLDB reproducibility guideline mentions GPU clock settings.

---

## Related-work comparison matrix (verified entries; see verification tags above)

| Work | Year & venue (type) | Problem | Method | Relevant finding | Relationship to Qenlo | Remaining gap |
|---|---|---|---|---|---|---|
| NVIDIA forum "Avoiding performance mode slowdown" | 2017, NVIDIA Dev Forums (FORUM) | Short CUDA call slower after idle | User telemetry + expert replies | ~100 µs vs ~300 µs by P-state; periodic dummy work fixes it | Same mechanism, single machine; keep-alive anticipates heater | Anecdotal; no controls, no cross-host |
| NVIDIA forum "kernel slower after waiting" | 2022 (FORUM) | Kernel 2x slower after 10 s idle | Experiments incl. keepBusyKernel | Keep-busy every 500 ms makes speed steady; njuffa: short sparse kernels never exceed ~1000 MHz | Direct precedent for heater sign | Same-process; no device timestamps reported |
| NVIDIA forum "Why warm-up?" | 2017/2025 (FORUM) | Slow after activity gap | Expert explanation | Deep idle ~300 MHz; lock clocks or dummy kernels; "prime the pump" | Precedent for load-raising mitigation | No measurement study |
| NVIDIA Reflex Low Latency Boost | 2020, NVIDIA (DOC/PRODUCT) | Latency when GPU underutilized | Driver keeps clocks high | Higher clocks reduce latency when GPU underutilized | Vendor acknowledges under-utilization → under-clocking → latency | Graphics; no data on kernel margins |
| TensorRT perf-environment docs | current (2026 snapshot), NVIDIA (DOC) | Stable inference benchmarking | Guidance | Floating clocks → tactic-selection nondeterminism; log clocks continuously; gaps change clocks | Overlaps rules (ii),(iv); contradicts claim-1 wording | Power-throttled regime emphasis, not idle-state |
| Nsight Compute Profiling Guide | current, NVIDIA (DOC) | Profiling determinism | Default clock locking | Kernel duration depends on prior kernels' clock state | Same confound, vendor-documented | No magnitudes |
| nvbench | current, NVIDIA (DOC/CODE) | Kernel benchmarking | Clock lock, throttle detection | Standard tooling for rule (iv) | Rule (iv) precedent | CUDA only; not app-level |
| SetStablePowerState blog (Bavoil & Cantlay) | 2022, NVIDIA (BLOG) | Profiling consistency | nvidia-smi lock guidance | Clock changes make comparisons between runs difficult | Precedent | Graphics focus |
| Triton set_gpu_clock / issue #2306 | 2023-, GitHub (CODE/FORUM) | Kernel timing | Clock locking; warm-up | 30% error with short warm-up | Practice precedent | Cause not formally established |
| Volkov, EECS-2016-143 | 2016, UC Berkeley (Thesis/TR) | GPU latency hiding | Clock polling + compute kernel to raise clock | Load-raising kernel used to control DVFS in measurements | Methodological precedent for heater-as-control | Same process; not studied as variable |
| Standard Kernel blog | 2026 (BLOG) | Kernel benchmarking fidelity | H100 experiments | Contention +21%/30x std; lock not guaranteed; sleep gaps change timing 11%; clouds refuse clock control | Matches Qenlo cloud limitation; opposite-sign regime | Heavy kernels only |
| Agarwal et al., arXiv:2609.21058 | Sep 2026 (PP) | LLM-generated kernel speedups | KernelBench re-timing | Idle between trials let clocks drop → spurious retraction of a real speedup | Clock state changed an implementation margin; weakens claim 1 | Anecdote within paper; not quantified |
| Patel et al., IEEE CAL | 2023 (PR) | Cloud GPU power management | A100/RTX characterization | MIG shared clock forces light partition 0.8→1.4 GHz; A100 holds max clock once CUDA context/Triton loaded | Nearest precedent for co-runner raising clocks; shows CUDA datacenter behavior may differ | Frames as power waste; no latency of light partition reported |
| Zhang et al., CORE | 2026, MLSys (PR) | Mobile LLM DVFS | Governor vs pinned frequencies | Low-utilization decode under-clocked; pinning higher cuts TPOT 34.6-41% | Same under-clocking of latency-bound work (mobile) | Mobile SoC; not implementation comparison |
| Velicka et al. | 2026, FGCS (PR; HCW 2025; arXiv 2025) | GPU frequency switching latency | LATEST tool | 0.2-471 ms transitions on GH200 | Explains why sub-ms kernels miss boost | Explicit frequency requests, not driver governor |
| Kanev et al. | 2014, IISWC (PR; cited) | CPU power mgmt vs tail latency | Google workloads | Low load → 2.4x latency vs high load; busy-spin in C0 worse than shallow C-state | CPU analog of "more load → faster" | CPU |
| Li et al., Tales of the Tail | 2014, SoCC (PR) | Tail latency sources | Systematic elimination | At 10% utilization, power saving gives higher tail than at higher load | CPU analog | CPU |
| Slynko et al. | 2026, GPGPU '26 (PR) | Cloud GPU variability | 3,500+ GPUs, 11 providers | <9% execution-level; up to 38% across devices | Baseline for "tens of percent" | Saturating benchmarks only |
| Mogilevsky et al. | 2026, IEEE TPDS (PR) | Cluster GPU variability | GEMM/STREAM/apps | 0.1-8.8% GEMM; up to 12.2% apps | Baseline | Saturating |
| Coplin & Burtscher | 2015, GPGPU-8 (PR) | Optimizations vs energy/perf | 128 code versions × clocks | Up to 5x changes; optimizations change power draw | Implementations change power, related to induced state | Fixed clocks; no idle states |
| Hoefler & Belli | 2015, SC (PR) | Benchmark reporting | Survey + rules | Only 2/95 papers report CIs | Precedent for rules (i),(iii) | Not GPU power-state specific |
| Baymax / Prophet | 2016/2017, ASPLOS (PR) | GPU co-location QoS | Prediction/scheduling | Up to 60x tail degradation (Prophet) | Co-location literature missing from paper | Slowdown direction only |
| REEF | 2022, OSDI (PR); TOCS 2025 | Real-time + best-effort GPU inference | Preemption | Tail latency >10x under concurrency | Same | Slowdown only |
| Orion | 2024, EuroSys (PR) | Fine-grained GPU sharing | Operator-level scheduling | Interference-aware tail latency | Same | Slowdown only |
| Edge governors need memory-clock state | 2026, arXiv:2606.16106 (PP) | Jetson DVFS latency estimation | Measured governor | EMC-blind models miss 25-28% deadlines; co-tenancy tails | Parallels E-SPLIT memory-clock result | Integrated SoC |
| CAGRA persistent kernel / ALGAS | 2025, NVIDIA docs (DOC) / IPDPS 2025 (PR) | Small-batch GPU ANN latency | Persistent kernels, dynamic batching | Low one-at-a-time latency by keeping kernel resident | Deployment mitigation that alters GPU load state | No power-state analysis |
| Ernst & Baldassarre | 2023, EMSE (PR) | Registered reports in SE | Editorial/process | RRs since MSR 2020 | Context for "pre-registered" wording | Not systems/GPU specific |

---

## Verdicts on priority/novelty claims, with recommended wording

1. Claim: "We have not found a report that this [default GPU DVFS under-clocking latency-bound work] changes the measured margin between two implementations of the same GPU operation."
   - Verdict: NEEDS QUALIFICATION (borderline contradicted for the general statement). TensorRT documentation states floating clocks cause non-determinism in tactic selection, i.e., timing-based choice among implementations of the same operation; Nsight Compute documents that measured kernel duration depends on clock state left by prior kernels; Agarwal et al. (arXiv, Sep 2026) report idle gaps letting clocks drop and spuriously retracting a real speedup. What remains unreported (to our search) is the directional, controlled result that the work-efficient implementation is penalized because its own light load induces a low power state while the wasteful one holds boost.
   - Suggested wording: "Vendor tooling documents that clock state affects timing-based comparisons between kernel implementations (e.g., TensorRT tactic selection; Nsight Compute clock control), and recent kernel-generation work reports idle-induced clock drops corrupting measured speedups. We have not found a controlled study showing that a work-efficient implementation is systematically disadvantaged because the lighter load it places on the GPU induces a lower power state."

2. Claim: "...or that an unrelated co-running GPU load speeds up a latency-bound GPU kernel."
   - Verdict: NEEDS QUALIFICATION. The mechanism and practice are documented: dummy/keep-busy kernels to keep clocks up (NVIDIA forums 2017, 2022; Volkov 2016 used a compute kernel to raise clocks before measurement), NVIDIA Reflex Boost keeps clocks high when the GPU is underutilized, and Patel et al. (IEEE CAL 2023) show MIG co-runners force a light partition to a higher shared clock. CPU analogues (Kanev 2014; Li et al. 2014) show higher load lowering latency. No peer-reviewed measurement of a separate-process co-runner speeding up a foreground kernel's device time was found.
   - Suggested wording: "Keeping a GPU busy with dummy work is a known practitioner remedy for idle-state slowdowns, and shared clock domains mean co-located work can raise a light workload's clock (Patel et al., 2023). We use a separate low-utilization process as a controlled intervention with a heavy-path negative control; we have not found a prior measurement of this effect on device time across hosts."

3. Claim: variation "up to 4.6× rather than tens of percent" is much larger than prior reports.
   - Verdict: NEEDS QUALIFICATION. It is larger than published same-model device variability under sustained load (Sinha 8-22%; Slynko up to 38%; Mogilevsky ≤12%), but those studies measure a different regime. Idle/power-state latency inflation of 2-3x for short GPU work is documented informally (forum reports ~3x, 2x; Jetson ~2x) and 2.4x on CPUs (Kanev 2014). The 4.6x figure is also across five pods with host and process confounded and two driver branches.
   - Suggested wording: "This spread is larger than reported same-model variability under sustained load (8-38%), which measures a different regime; it is comparable to informal reports of 2-3x idle-state slowdowns of short GPU work, and on the pods it confounds host, process and driver."

4. Claim (contribution): reporting rules for latency-bound GPU work.
   - Verdict: LARGELY ESTABLISHED; NOVEL ONLY IN PART. (i) process-level replication: Kalibera & Jones 2013 (cited), Maricq 2018 (cited); (ii) continuous clock logging: TensorRT docs (`nvidia-smi dmon`); (iii) device + end-to-end time: TensorRT/trtexec reports GPU compute time and end-to-end separately; (iv-a) lock clocks and report both: nvbench, Nsight Compute, TensorRT, SetStablePowerState; (iv-b) load-raising control + heavy negative control: not found as a stated rule (Volkov 2016 used a load-raising kernel as a measurement aid); (v) state power state of compared implementations: not found as a stated rule.
   - Suggested wording: "Rules (i)-(iv-a) consolidate existing guidance [cite]; we add (iv-b) and (v)."

5. Implicit claim (Sec. 1/9): kernel-benchmarking practice locks clocks, cited to two 2026 preprints.
   - Verdict: SUPPORTED but MIS-SOURCED. Replace/supplement with nvbench, Nsight Compute, TensorRT docs, NVIDIA SetStablePowerState blog, Triton set_gpu_clock.

6. Implicit claim: "Isolated benchmarks may misstate co-located latency."
   - Verdict: SUPPORTED (known in slowdown direction) — cite Baymax, Prophet, REEF, Orion; the speed-up direction is the new part and is untested with a real co-runner (paper says so).

7. Vector-search-specific: power-state variance in a GPU filtered vector-search kernel.
   - Verdict: SUPPORTED as unreported to our search; frame as a domain instance and cite persistent-kernel/dynamic-batching small-batch GPU ANN work (CAGRA persistent search, ALGAS IPDPS 2025) as mitigations that alter load state.

## Citations to add (missing from manuscript), prioritized
1. NVIDIA TensorRT perf-environment docs (floating clocks, tactic-selection nondeterminism, dmon logging).
2. Nsight Compute Profiling Guide, Clock Control.
3. NVIDIA nvbench (clock locking, throttle detection).
4. Patel et al., IEEE CAL 2023, DOI 10.1109/LCA.2023.3278652.
5. NVIDIA Developer Forum threads 54181 (2017), 221059 (2022), 48565, 232943 (as grey literature) and NVIDIA Reflex Low Latency Boost page.
6. Agarwal et al., arXiv:2609.21058 (2026, preprint).
7. Zhang et al., CORE, MLSys 2026 (replace arXiv 2507.02135 citation or add).
8. Velicka et al., FGCS 179 (2026) 108331, DOI 10.1016/j.future.2025.108331.
9. Li et al., "Tales of the Tail", SoCC 2014 (CPU low-utilization tail inflation).
10. Hoefler & Belli, SC'15, DOI 10.1145/2807591.2807644.
11. Slynko et al., GPGPU '26, DOI 10.1145/3818671.3818674; Mogilevsky et al., IEEE TPDS 2026, DOI 10.1109/TPDS.2026.3684387.
12. Baymax (ASPLOS '16, 10.1145/2872362.2872368), Prophet (ASPLOS '17, 10.1145/3037697.3037700), REEF (OSDI '22), Orion (EuroSys '24, 10.1145/3627703.3629578).
13. Volkov 2016 EECS-2016-143 (load-raising kernel as measurement control).
14. Coplin & Burtscher GPGPU-8 2015 (10.1145/2716282.2716292); Mei, Wang, Chu DCN 2017 (10.1016/j.dcan.2016.10.001).
15. Ernst & Baldassarre, EMSE 2023 (10.1007/s10664-022-10277-5); ACM Artifact Review and Badging v1.1.
16. CAGRA persistent kernel docs; ALGAS IPDPS 2025.
