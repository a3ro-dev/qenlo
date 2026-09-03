#!/usr/bin/env bash
set -Eeuo pipefail

STAGE=${1:-compatibility}
ROOT=${QENLO_REMOTE_ROOT:-/workspace/qenlo-campaign}
REPO="$ROOT/repo"
OUT="$ROOT/artifacts"
mkdir -p "$OUT"
exec > >(tee "$OUT/bootstrap.log") 2>&1

export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends \
  build-essential ca-certificates clang cmake curl git jq libclang-dev \
  libegl1 libgl1 libglvnd0 libglx0 libvulkan1 pkg-config python3 python3-pip \
  python3-venv rsync time unzip vulkan-tools
if [[ "$STAGE" == reference ]]; then
  apt-get install -y --no-install-recommends latexmk poppler-utils texlive-fonts-recommended \
    texlive-latex-extra texlive-latex-recommended
  python3 -m pip install --break-system-packages --no-cache-dir faiss-cpu==1.15.0 \
    matplotlib==3.10.6 numpy==2.3.3 pandas==2.3.2 psutil==7.0.0
fi

if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi
source /root/.cargo/env
rustup toolchain install 1.98.0 --profile minimal
rustup default 1.98.0

driver_version=$(nvidia-smi --query-gpu=driver_version --format=csv,noheader 2>/dev/null | head -n1 || true)
if [[ -n "$driver_version" ]] && ! ldconfig -p | grep -q 'libGLX_nvidia.so.0'; then
  driver_major=${driver_version%%.*}
  curl -fsSLo /tmp/cuda-keyring.deb \
    https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2404/x86_64/cuda-keyring_1.1-1_all.deb
  dpkg -i /tmp/cuda-keyring.deb
  apt-get update
  if apt-cache show "libnvidia-gl-$driver_major" >/dev/null 2>&1; then
    apt-get install -y --no-install-recommends "libnvidia-gl-$driver_major"
  fi
  ldconfig
fi

if [[ -n "$driver_version" ]] && ! find /usr/share/vulkan/icd.d -type f -iname '*nvidia*.json' -print -quit | grep -q .; then
  nvidia_glx=$(ldconfig -p | awk '/libGLX_nvidia.so.0/{print $NF; exit}')
  if [[ -n "$nvidia_glx" ]]; then
    cat > /usr/share/vulkan/icd.d/nvidia_icd.json <<JSON
{"file_format_version":"1.0.0","ICD":{"library_path":"$nvidia_glx","api_version":"1.3.280"}}
JSON
  fi
fi

{
  echo "stage=$STAGE"
  echo "captured_utc=$(date -u +%FT%TZ)"
  uname -a
  lscpu
  free -b
  df -B1
  nvidia-smi -q || true
  echo "driver_version=$driver_version"
  echo "image=${RUNPOD_POD_IMAGE_NAME:-unknown}"
  rustc -Vv
  cargo -V
  python3 --version
  dpkg-query -W -f='${Package}=${Version}\n' | sort
} > "$OUT/environment.txt" 2>&1

WGPU_BACKEND=vulkan
if ! vulkaninfo --summary > "$OUT/vulkaninfo.txt" 2>&1; then
  WGPU_BACKEND=gl
  echo "vulkan_validation=failed" | tee "$OUT/status.txt"
  echo "compatibility_fallback=gl" >> "$OUT/status.txt"
else
  echo "vulkan_validation=passed" | tee "$OUT/status.txt"
fi

cd "$REPO"
export QENLO_REQUIRE_GPU=1
export WGPU_BACKEND
cargo test -p qenlo --features gpu-wgpu gpu_row_preparation_modes_are_observable_and_exact

SMOKE="$ROOT/smoke.qnb"
cargo run --release -p qenlo-bench --features gpu-wgpu -- prepare \
  --dataset "$SMOKE" --rows 256 --dimensions 16 --tuning 8 --evaluation 32 --seed 42
cargo run --release -p qenlo-bench --features gpu-wgpu -- run \
  --dataset "$SMOKE" --output "$OUT/gpu-smoke" --dimensions 16 \
  --backend gpu-rows --gpu-row-preparation one-pass --eligible-count 128 \
  --distribution independent --batch 1 --warmups 2 --repetitions 1 \
  --order-seed 9001 --diagnostics basic

echo "compatibility=passed" >> "$OUT/status.txt"
echo "wgpu_backend=$WGPU_BACKEND" >> "$OUT/status.txt"
find "$OUT" -type f -print0 | sort -z | xargs -0 sha256sum > "$OUT/SHA256SUMS"
