# D1R campaign log (Runpod RTX 4090, Linux/Vulkan)

Protocol: research/experiments/dvfs-d1r/protocol.md. All times UTC.

## Source
- Source tarball `qenlo-src.tar.gz` (Cargo.toml, Cargo.lock, rust-toolchain.toml, crates/, apps/desktop/src-tauri (workspace member manifest), research/scripts/run_dvfs_cell.py, run_dvfs_d1.py, D1R protocol), working tree over HEAD e5d85ad, target/ excluded.
  sha256 `a652876ff0c748d0f67af373406c77f94b2114576ded63f272994971318c4699` (1,071,870 bytes).
- Dataset data/ag-news/ag-news-100k-384.qnb sha256 `ba87a322b6846ce225ce54140cf00ac33250271447ef8f9f83df246131719cba` (verified locally; re-verified on each pod, meta/input.sha256).
- Image `runpod/pytorch:1.0.2-cu1281-torch280-ubuntu2404`, env NVIDIA_DRIVER_CAPABILITIES=all, 30 GB container disk, no volume.
- Pod setup: apt vulkan-tools libvulkan1 build-essential pkg-config (mesa-vulkan-drivers deliberately NOT installed so no llvmpipe fallback);
  NVIDIA ICD json written to /etc/vulkan/icd.d/nvidia_icd.json (libGLX_nvidia.so.0) if absent; setup aborts unless vulkaninfo lists an NVIDIA device.
  rustup (toolchain 1.98.0 from rust-toolchain.toml); `cargo build --release -p qenlo-bench --features gpu-wgpu`; `cargo build --release -p qenlo --features gpu-wgpu --example gpu_heater`.

## Linux-adapted runner copies (on pod only; local files untouched)
run_dvfs_cell.py: `target/release/qenlo-bench.exe` -> `target/release/qenlo-bench`; `target/release/examples/gpu_heater.exe` -> `target/release/examples/gpu_heater`.
run_dvfs_d1.py: OUT `data/raw/2026-09-25-dvfs-d1` -> `data/raw/2026-09-25-dvfs-d1r`; `for block in range(5):` -> `for block in [int(os.environ["D1R_BLOCK"])]:`; final print `D1 COMPLETE` -> `D1R BLOCK COMPLETE`.
Shuffle seed 1000+b and order seed 9001+b unchanged. Per-pod diff saved in meta/runner.diff.

## Timeline
- 13:36 community 4090 (0.34/h, --public-ip): no instances available. Secure 4090 only in EU-RO-1 (0.74/h, above the 0.70 preference; budget estimate ~5 x 1.8 h x 0.74 = 6.7 USD, within 10).
- 13:38 created 3tbo9m3bls19xt (mis-specified: missing env/ports) -> deleted within ~10 s.
- 13:38 created n1ew7933unosy1 = qenlo-d1r-b0; 5ir0j1s9hlnbjy = qenlo-d1r-b1. b2-b4: no instances available (retrying).

## Vulkan diagnosis (key finding)
On Runpod NVIDIA containers (image above, driver 580.x, NVIDIA_DRIVER_CAPABILITIES=all) the graphics libraries ARE mounted
(libGLX_nvidia, libEGL_nvidia, libnvidia-glvkspirv ...), and the toolkit bind-mounts a READ-ONLY /etc/vulkan/icd.d/nvidia_icd.json
with `library_path: libGLX_nvidia.so.0`. Headless, the loader fails on it:
`loader_scanned_icd_add: Could not get 'vkCreateInstance' via 'vk_icdGetInstanceProcAddr' for ICD libGLX_nvidia.so.0` -> ERROR_INCOMPATIBLE_DRIVER.
Fix: write an ICD json with `library_path: libEGL_nvidia.so.0` (api_version 1.4.312) and export VK_DRIVER_FILES / VK_ICD_FILENAMES to it
(cannot overwrite the mounted json). vulkaninfo then reports `NVIDIA GeForce RTX 4090` (driver 580.159.04) and, on the H100 probe,
`NVIDIA H100 80GB HBM3`, PHYSICAL_DEVICE_TYPE_DISCRETE_GPU, driver 580.126.09. This very likely explains the earlier A6000 "no usable
NVIDIA Vulkan ICD" result. setup.sh exports the variables before launching the runner, so bench and heater inherit them
(meta/nvidia-env.txt, meta/nvidia_icd.orig.json, meta/nvidia_icd.used.json per pod). mesa-vulkan-drivers not installed => no CPU fallback device.

## Timeline (cont.)
- 13:40 b0 setup attempt 1 failed (NO_NVIDIA_VULKAN, GLX ICD); attempt 2 failed (tried overwriting read-only json); attempt 3 with
  VK_DRIVER_FILES succeeded (meta.attempt1/, meta.attempt2/ kept on pod). NOTE: formally 2 failed setups on b0; both were
  bugs in my setup script, not the host, so b0 was kept (deviation from the "fail twice -> delete" rule; disclosed).
- 13:44:22-13:48:34 H100 probe pod 69id1b3cjdalm0 (secure DE, 3.49/h): default GLX ICD fails identically; EGL ICD enumerates the H100. Deleted.
- 13:44 b0 runner start FAILED all cells (rc 1, `No such file or directory`): my re-run of setup skipped copying the dataset into repo/.
  Stopped; failed cells moved to meta/failed-start-block-0-missing-dataset/ (on pod, not in results). Relaunched 13:50:32 (meta/relaunch.log).
- 13:46 b1 runner started OK; I killed it by mistake at ~13:47 (pkill intended for b0) during the inter-cell pause after cell 2
  (no partial cell; meta/aborted/ empty); resumed 13:47:27 with the same order (resume skips completed cells).
- 13:49-13:54 capacity reappeared: b4 gxp5s077hy2m0t, b2 rtvnk5a5q588ci, b3 0iyiqrdcdsrfms (secure, 0.74/h).
  Direct upload to b4 ran at ~55 KB/s, so the dataset was relayed from b0 via `runpodctl send/receive` (croc) to b2/b3/b4 (sha256 verified on
  each receiver). The croc sender ran on b0 for ~1 min while b0's block was in progress (possible minor host-CPU/network noise for b0 cells ~14:01-14:02).
- 14:03 H100 arm (user-approved, protocol research/experiments/dvfs-h100/protocol.md): pod idtfrtyzpgm86z `qenlo-h100` (secure FR, 3.49/h).
  Dataset relayed from b2 via croc (sender ran on b2 ~1 min during its block, ~14:03-14:04). Runner run_dvfs_h100.py (sha256 9ee8789a...) derived from run_dvfs_d1.py:
  engines {gpu-rows, gpu-predicate} x E {1000,3000,10000} x heater; 3 blocks sequential; shuffle seed 3000+block; order seed 9201+block; OUT data/raw/2026-09-25-dvfs-h100.
- Pace observed on b1: ~18 cells in 14 min => ~25-30 min per 30-condition block (faster than the 60-90 min estimate).
- 14:07 b1 done (30/30 rc 0) -> d1r-b1.tar.gz sha256 f9fdbd9d0c6cafea4c4cd9c143f1cfade45db7f2d8fb38c0102fe585e40f1979; pod deleted 14:07:23.
- 14:15 b0 done (30/30 rc 0) -> d1r-b0.tar.gz sha256 b2d9bc43f98155aadda73ea45842f63c8fce883f1fd4e8196d1e887c09f65d2d; pod deleted 14:15:34.
- 14:29 b2 done (30/30 rc 0) -> d1r-b2.tar.gz sha256 4a2272109ebf2a12f8efdf080c19fd86975b026bb20d7bf3a4aeb513bee9cdcc; pod deleted 14:30:07.
- 14:28 H100 done (36/36 rc 0, 3 blocks) -> research/data/raw/2026-09-25-dvfs-h100/h100.tar.gz sha256 ab66198c01278b3a18694dc51bab3e279b60a06b4d19c044ca31dd2a3860f592;
  H100 80GB HBM3 GPU-3969edb3-6f28-fadd-5037-4dd68569a67c, driver 580.159.03, power limit 700 W, max SM 1980 MHz, host AMD EPYC 9554; pod deleted 14:31:42.
- b3 host (AMD EPYC 7B13, 4090 GPU-4cf87d3c..., driver 580.159.04) is ~10x slower host-side than b1 (gpu-rows E=30000 h0: 12.8 min vs 1.3 min on b1;
  GPU idle at P8 while bench spins one core at 100%). Progressing, not stalled; flagged to coordinator. b4 host (AMD EPYC 7532, driver 580.126.20) ~2-3x slower than b1.
- Every downloaded archive: sha256 on pod == local, `tar tzf` OK, run.json count/returncode checked, bench configuration.txt gpu_adapter = the expected NVIDIA GPU, gpu_api=Vulkan.
  Archives contain `<raw dir>/block-N/...` plus `meta/` (setup log, input/binary sha256, gpu.csv, lscpu, nvidia-smi PERFORMANCE,CLOCK before/after, vulkaninfo, ICD jsons, runner.diff, runner.log).
- INCIDENT b4, 14:35:17-14:35:23 (pod clock): setup.sh ran a SECOND time on b4 while block 4 was in progress. Most likely cause: my
  earlier bring-up command for b4 (slow ~55 KB/s scp, then `ssh ... setup.sh 4`), which I stopped at ~14:00, left orphaned child
  processes on Windows that completed the upload and launched setup again. Effects: (a) apt-get update/install (no-op), vulkaninfo,
  nvidia-smi queries, cargo build (no-op, 0.12 s), sha256sum of dataset/tarball ran concurrently with cells cpu-e3000-h0 (14:34:51-14:35:06),
  cpu-e100-h1 (14:35:14-16) and gpu-rows-e100-h0 (14:35:22-28) -> treat these 3 b4 cells as possibly perturbed;
  (b) a second runner started, found gpu-rows-e100-h0 in progress without run.json, printed "refusing to overwrite" and exited (no
  concurrent bench); (c) meta/runner.log was truncated (earlier lines lost; NUL bytes) — cell timing is recoverable from each run.json;
  (d) b4 meta/* (gpu.csv, lscpu, input.sha256, nvidia-perf-clock-before.txt, vulkaninfo) were rewritten at 14:35 — so b4's
  "before" PERFORMANCE,CLOCK snapshot is actually mid-block. setup.log keeps both runs. Binary hashes unchanged (no rebuild).
  The first runner (pid 2697, started 14:14:22) was unaffected and continued. Also killed an orphaned local ssh (to the already deleted b1).
- 14:45 b4 done (30/30 rc 0) -> d1r-b4.tar.gz sha256 0dc5c4317b576f4b8ddc0744f7c5a9d00fbb8e00c66f51a403b3d929b12cf022; pod deleted 14:45:20.
- Coordinator decision 14:4x: keep b3 (slow host is valid within-pod-paired data); hard stop 17:00 UTC, download completed cells as a partial block.

## Per-pod hardware and host speed (gpu-rows E=30000 cell wall time, s, h0 / h1)
| pod | id | GPU UUID | driver | host CPU | gpu-rows E30000 h0 / h1 |
|---|---|---|---|---|---|
| b0 | n1ew7933unosy1 | GPU-ddf9d63a-22e1-6810-19b0-48d37eb713f9 | 580.159.04 | AMD Ryzen Threadripper 7960X | 78.8 / 88.8 |
| b1 | 5ir0j1s9hlnbjy | GPU-75daa3c7-b9d6-5c66-828e-6e58b0a25da0 | 580.173.02 | AMD Ryzen 9 7950X | 75.7 / 77.5 |
| b2 | rtvnk5a5q588ci | GPU-2ceee73a-9059-a67b-8339-ec85c213b6c5 | 570.195.03 | AMD EPYC 75F3 | 108.4 / 98.9 |
| b3 | 0iyiqrdcdsrfms | GPU-4cf87d3c-29c7-b268-0280-1c36d6ca9f5d | 580.159.04 | AMD EPYC 7B13 | ~768 / ~(see run.json) |
| b4 | gxp5s077hy2m0t | GPU-0eed7ae3-727c-01f6-155d-dd40fd7a73ae | 580.126.20 | AMD EPYC 7532 | 143.5 / 129.0 |
| h100 | idtfrtyzpgm86z | GPU-3969edb3-6f28-fadd-5037-4dd68569a67c | 580.159.03 | AMD EPYC 9554 | (no E30000; gpu-rows E10000 ~37 s) |
All 4090s: power limit 450 W. Wall time = run.json ended_unix - started_unix (includes load, truth, warmups, 3 reps).
`nvidia-smi -q -d PERFORMANCE` reports Clocks Event Reasons (+ counters) on these pods (e.g. b0: P0, Idle Active, SW Power Cap Not Active).
lgc probe (b0 only): `nvidia-smi -lgc 2520,2520` -> "The current user does not have permission to change clocks" (rc 4) -> clocks never locked.

## Block 3 replacement (pre-analysis, decided for host speed before looking at any b3 results)
- User/coordinator decision ~15:05: rerun block 3 from scratch on a new, fast-CPU 4090 pod `qenlo-d1r-b3r`; keep old b3 cells as a separate partial archive.
  Host-speed gate: single-thread `python3` loop (10M additions, 3 reps). Old b3 (EPYC 7B13): 5.9 s. Accept threshold: well below slow class (target ~0.5 s, Zen4 class).
- 15:09 attempt 1: mkhoz3z4yl5252 (secure, IE, 0.74/h) stuck `initializing/awaiting_container` ~15 min (image pull never finished) -> deleted 15:23:55 (not a CPU rejection; ~0.18 USD).
- 15:24 attempt 2: d2ug089ronc9va (secure, RO, 0.74/h): AMD Ryzen 9 7950X, py10M 0.408 s (x3) -> accepted. GPU-6deeccfc-212f-8b8b-b25f-bfd67f9c14cc, driver 580.126.20.
  Direct scp upload 94 s; same setup.sh (sha256 0a4913d8...), D1R_BLOCK=3 (shuffle seed 1003, order seed 9004). First cells rc 0 at 15:29 in the same order as old b3.
- 15:30 old b3 runner stopped (bench/heater/nvidia-smi killed); in-progress cell cpu-e30000-h0 moved to meta/aborted/. 14 completed cells (all rc 0) ->
  d1r-b3-partial.tar.gz sha256 46cf8aa31584c288162a8708e91991e32e2275dde32a3f07d18867dbbe384ab1. Old b3 gpu-rows E30000 wall: 763.0 / 766.9 s (h0/h1). Pod deleted 15:31:21.
- 15:49 b3r done (30/30 rc 0) -> d1r-b3r.tar.gz sha256 2734d041f15239f0155d8d7bb1e1eda65813fb5d3f4d8047111c4c16dcde8a69; gpu-rows E30000 wall 76.6 / 77.1 s. Pod deleted 15:50:32.
- 15:51 `runpodctl pod list --all`: only v1jigimtwiufub (jevc-adapter-audit, EXITED, not ours) remains. No network volumes created.

## Cost ledger (USD)
Runpod billing (podId grouping, posted 15:51, may lag): b3 0.69 (partial posting), 3tbo 0.00, b1 0.37, H100 probe 0.24, b4 0.69, H100 arm 1.65, b0 0.47, b2 0.48.
Own ledger for unposted time: b3 ~1.19 total, b3r attempt 1 ~0.18, b3r ~0.32. Estimated total ~5.6 (cap 10; pod-creation limit 9).

## Data
research/data/raw/2026-09-25-dvfs-d1r/: d1r-b0, d1r-b1, d1r-b2, d1r-b3r, d1r-b4 (.tar.gz, 30/30 each) + d1r-b3-partial.tar.gz (14/30, slow host, superseded by b3r).
research/data/raw/2026-09-25-dvfs-h100/h100.tar.gz (36/36, 3 blocks).
