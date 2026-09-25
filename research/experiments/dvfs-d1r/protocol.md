# D1R protocol — cloud replication of D1 on RTX 4090 (Linux, Vulkan)

Written 2026-09-25 before any D1R data and before inspecting D1 results (D1 still running locally).
User-approved paid compute: 5 x RTX 4090 Runpod pods, budget cap USD 10, all pods deleted at end.

Identical to D1 (`research/experiments/dvfs-d1/protocol.md`) — same 30 conditions, heater (500 iters, 1 ms),
B=1, k=10, 200 warmups, 3 x 5000 queries, same dataset bytes (sha256 ba87a322...) — except:
 - Host: one pod per block (block b on pod b, b=0..4); Linux, WGPU_BACKEND=vulkan; condition order uses the D1
   shuffle seed 1000+b and order seed 9001+b. Pairing (heater on/off) is within a pod, so host differences enter
   only the between-block variance.
 - Binary and heater built on the pod from the same working tree (source tarball sha256 recorded per pod).
 - Telemetry: nvidia-smi 50 ms (clocks.sm, clocks.mem, pstate, utilization, power, temperature).
Predictions P1-P4 and refutation criteria are exactly D1's. Additionally recorded: pod id, GPU UUID, driver,
host CPU model, and whether `nvidia-smi -q -d PERFORMANCE` reports clock-event reasons.
