#!/usr/bin/env bash
# Cross-host exploratory run (research/experiments/cross-host-2026-10-31/protocol.md).
# Expects /workspace/base and /workspace/cand source trees. Writes /workspace/out, then
# /workspace/out/DONE. Steps run in priority order so a deadline kill keeps the most useful data.
set -uo pipefail
OUT=/workspace/out
mkdir -p "$OUT"
exec > >(tee -a "$OUT/log.txt") 2>&1
step() { echo "=== $(date -u +%T) $*"; }

step bootstrap
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq --no-install-recommends build-essential ca-certificates curl pkg-config \
  libvulkan1 vulkan-tools libegl1 libglvnd0 libglx0 >/dev/null
if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal -q
fi
source /root/.cargo/env
rustup toolchain install 1.98.0 --profile minimal -q
GPU=0
driver=$(nvidia-smi --query-gpu=driver_version --format=csv,noheader 2>/dev/null | head -n1 || true)
if [[ -n "$driver" ]]; then
  GPU=1
  # Same NVIDIA Vulkan ICD fix as scripts/runpod/bootstrap.sh.
  if ! ldconfig -p | grep -q 'libGLX_nvidia.so.0'; then
    curl -fsSLo /tmp/kr.deb https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2404/x86_64/cuda-keyring_1.1-1_all.deb
    dpkg -i /tmp/kr.deb >/dev/null && apt-get update -qq
    apt-get install -y -qq --no-install-recommends "libnvidia-gl-${driver%%.*}" >/dev/null || true
    ldconfig
  fi
  if ! find /usr/share/vulkan/icd.d -iname '*nvidia*.json' 2>/dev/null | grep -q .; then
    glx=$(ldconfig -p | awk '/libGLX_nvidia.so.0/{print $NF; exit}')
    mkdir -p /usr/share/vulkan/icd.d
    [[ -n "$glx" ]] && printf '{"file_format_version":"1.0.0","ICD":{"library_path":"%s","api_version":"1.3.280"}}\n' "$glx" \
      > /usr/share/vulkan/icd.d/nvidia_icd.json
  fi
  vulkaninfo --summary > "$OUT/vulkaninfo.txt" 2>&1 && echo vulkan=ok >> "$OUT/status.txt" || echo vulkan=failed >> "$OUT/status.txt"
fi
{ lscpu; free -b; nproc; nvidia-smi -q 2>/dev/null | head -40; rustc -V; } > "$OUT/environment.txt" 2>&1

step build-core
for tree in base cand; do
  (cd /workspace/$tree && cargo build -q --release -p qenlo-core --examples) || echo "build_$tree=failed" >> "$OUT/status.txt"
done
B=/workspace/base/target/release/examples
C=/workspace/cand/target/release/examples

step cpu-round-1
"$B/cpu_cliff_bench" > "$OUT/base_1.csv"
"$C/cpu_cliff_bench" > "$OUT/cand_1.csv"
step probe
"$C/sq8_probe" > "$OUT/cand_probe.txt"
"$B/sq8_probe" > "$OUT/base_probe.txt"

if [[ $GPU == 1 ]] && grep -q vulkan=ok "$OUT/status.txt"; then
  step build-bench
  cd /workspace/cand
  cargo build -q --release -p qenlo-bench --features gpu-wgpu || echo build_bench=failed >> "$OUT/status.txt"
  QB=/workspace/cand/target/release/qenlo-bench
  "$QB" prepare --dataset /workspace/syn.qnb --rows 100k --dimensions 384 --tuning 1000 --evaluation 5000 --seed 42
  step crossover
  mkdir -p "$OUT/xover"
  export WGPU_BACKEND=vulkan
  # Order fixed by seed 20261031 (protocol): shuffled once, same on every host.
  for cell in gpu-rows:30000 cpu:4000 gpu-rows:100000 cpu:30000 gpu-rows:4000 cpu:100000 gpu-rows:10000 cpu:10000; do
    be=${cell%%:*}; e=${cell##*:}
    "$QB" run --dataset /workspace/syn.qnb --output "$OUT/xover/$be-e$e" --dimensions 384 \
      --backend "$be" --eligible-count "$e" --distribution independent --batch 1 --k 10 \
      --warmups 200 --repetitions 3 --order-seed 42 >/dev/null 2>"$OUT/xover-$be-e$e.err" \
      || echo "xover_${be}_$e=failed" >> "$OUT/status.txt"
  done
fi

step cpu-round-2
"$B/cpu_cliff_bench" > "$OUT/base_2.csv"
"$C/cpu_cliff_bench" > "$OUT/cand_2.csv"
step done
touch "$OUT/DONE"
