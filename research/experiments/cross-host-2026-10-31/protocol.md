# Cross-host exploratory run (registered 2026-10-31, before any pod exists)

EXPLORATORY: this does not replace the laptop gates (sq8-certified, gpu-crossover-int8). It asks
whether those effects hold on other CPUs and GPUs, and gives numbers for later decisions.

Hosts: RunPod RTX 3090 (community), RTX 2000 Ada (secure), RTX A6000 (community), RTX 4090
(community), one CPU-only pod. Every pod is deleted by the driver; a separate watchdog deletes any
`qenlo-xh-*` pod 40 min after launch. Worst-case spend <= $1 (price cap x 35 min, summed).

Trees: base = 939f7f0/fbe2853 (identical qenlo-core), cand = working tree with the chunked arena +
int8 certified scan (the sq8-certified candidate). Script: `scripts/runpod/cross_host_bench.sh`,
driver: `research/scripts/run_cross_host_pods.py`.

Per host, in priority order: cpu_cliff_bench base/cand (round 1); sq8_probe base/cand (random data:
exactness + heap); on GPU hosts with working Vulkan, qenlo-bench synthetic 100k x 384, batch 1,
k=10, warmups 200, repetitions 3, backends cpu and gpu-rows at E in {4k, 10k, 30k, 100k}; then
cpu_cliff_bench round 2. Deviations from the laptop crossover protocol: synthetic data, 3
repetitions, one session per host. Cloud vCPUs are shared, so per-host noise is expected to be
higher than on the laptop.

Reported per host: Auto and Int8 vs base ratios (median, P95) per E; exactness mismatches; heap
B/row; CPU and gpu-rows p95 and median per E; the smallest E at which gpu-rows beats cpu, if any.
