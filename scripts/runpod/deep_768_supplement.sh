#!/usr/bin/env bash
set -Eeuo pipefail

ROOT=${QENLO_REMOTE_ROOT:-/workspace/qenlo-deep768}
CURRENT="$ROOT/current"
OUT="$ROOT/artifacts"
DATA="$ROOT/datasets"
TARGET="$ROOT/target"
STATUS="$OUT/status.tsv"
SOURCE_SHA=${QENLO_SOURCE_BUNDLE_SHA256:?source bundle SHA-256 is required}
WORKLOAD_SHA=${QENLO_WORKLOAD_SCRIPT_SHA256:?workload script SHA-256 is required}

for value in "$SOURCE_SHA" "$WORKLOAD_SHA"; do
  [[ "$value" =~ ^[0-9a-f]{64}$ ]] || { echo "invalid SHA-256 input" >&2; exit 2; }
done
[[ $(sha256sum "${BASH_SOURCE[0]}" | cut -d' ' -f1) == "$WORKLOAD_SHA" ]] || {
  echo "workload script SHA-256 mismatch" >&2
  exit 2
}

mkdir -p "$OUT" "$DATA"
printf 'stage\tcell\tengine\tstatus\texit_code\n' > "$STATUS"
exec > "$OUT/remote.log" 2>&1
started=$(date +%s)

record_command() {
  local stage=$1 cell=$2 engine=$3 log=$4
  shift 4
  set +e
  "$@" >"$log" 2>&1
  local code=$?
  set -e
  local status=completed
  [[ $code -eq 0 ]] || status=failed
  printf '%s\t%s\t%s\t%s\t%s\n' "$stage" "$cell" "$engine" "$status" "$code" >> "$STATUS"
  return $code
}

export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends \
  build-essential ca-certificates clang curl libclang-dev libegl1 libgl1 libglvnd0 \
  libglx0 libvulkan1 pkg-config python3-venv time vulkan-tools
if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi
source /root/.cargo/env
rustup toolchain install 1.98.0 --profile minimal
rustup default 1.98.0
python3 -m venv --system-site-packages "$ROOT/venv"
source "$ROOT/venv/bin/activate"
python3 -m pip install --disable-pip-version-check --no-cache-dir \
  numpy==2.3.3 faiss-cpu==1.15.0 psutil==7.0.0

driver_version=$(nvidia-smi --query-gpu=driver_version --format=csv,noheader | head -n1)
if ! ldconfig -p | grep -q 'libGLX_nvidia.so.0'; then
  driver_major=${driver_version%%.*}
  curl -fsSLo /tmp/cuda-keyring.deb \
    https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2404/x86_64/cuda-keyring_1.1-1_all.deb
  dpkg -i /tmp/cuda-keyring.deb
  apt-get update
  apt-get install -y --no-install-recommends "libnvidia-gl-$driver_major"
  ldconfig
fi
bash "$CURRENT/scripts/runpod/fix-vulkan-icd.sh" > "$OUT/vulkaninfo.txt"

{
  echo "captured_utc=$(date -u +%FT%TZ)"
  echo "current_source_bundle_sha256=$SOURCE_SHA"
  echo "workload_script_sha256=$WORKLOAD_SHA"
  uname -a
  lscpu
  free -b
  df -B1
  nvidia-smi -q
  rustc -Vv
  cargo -V
  python3 --version
  python3 -m pip freeze | sort
} > "$OUT/environment.txt" 2>&1

export CARGO_BUILD_JOBS=1 QENLO_REQUIRE_GPU=1 WGPU_BACKEND=vulkan
export OMP_NUM_THREADS=2 MKL_NUM_THREADS=2 OPENBLAS_NUM_THREADS=2
export QENLO_SOURCE_BUNDLE_SHA256="$SOURCE_SHA"

build_started=$(date +%s%N)
CARGO_TARGET_DIR="$TARGET" cargo build --manifest-path "$CURRENT/Cargo.toml" \
  --release -p qenlo-bench --features gpu-wgpu -j 1
build_finished=$(date +%s%N)
printf 'current_build_ns=%s\n' "$((build_finished-build_started))" > "$OUT/build-times.txt"
BENCH="$TARGET/release/qenlo-bench"
DATASET="$DATA/r100000-d768.qnb"
"$BENCH" prepare --dataset "$DATASET" --rows 100000 --dimensions 768 \
  --tuning 16 --evaluation 64 --seed 20260905

run_native() {
  local cell=$1 engine=$2 backend=$3 batch=$4 k=$5 fraction=$6
  shift 6
  local output="$OUT/runs/$cell/$engine"
  mkdir -p "$(dirname "$output")"
  record_command supplement "$cell" "$engine" "$OUT/runs/$cell/$engine.log" \
    /usr/bin/time -v -o "$OUT/runs/$cell/$engine.time" \
    "$BENCH" run --dataset "$DATASET" --output "$output" --dimensions 768 \
    --backend "$backend" --distribution independent --fraction "$fraction" --batch "$batch" \
    --k "$k" --warmups 8 --repetitions 3 --order-seed 20260905 --recall-target 0.99 \
    --diagnostics detailed --vector-budget-mib 2048 --gpu-budget-mib 1024 "$@"
}

run_cell() {
  local cell=$1 batch=$2 k=$3 fraction=$4 cpu engine output
  cpu="$OUT/runs/$cell/current-cpu"
  run_native "$cell" current-cpu cpu "$batch" "$k" "$fraction" || return 0
  run_native "$cell" current-gpu gpu-rows "$batch" "$k" "$fraction" \
    --gpu-row-preparation cached --oracle-reference "$cpu" || true
  for engine in numpy faiss-flat torch-cpu torch-cuda; do
    output="$OUT/runs/$cell/$engine"
    record_command supplement "$cell" "$engine" "$OUT/runs/$cell/$engine.log" \
      python3 "$CURRENT/scripts/benchmark_small_vectors.py" --backend "$engine" \
      --reference "$cpu" --dataset "$DATASET" --output "$output" --threads 2 || true
  done
}

run_cell d4-r100000-d768-b1-k1-f1 1 1 1
run_cell d5-r100000-d768-b8-k64-f01 8 64 0.1

printf 'elapsed_seconds=%s\ncompleted_utc=%s\n' "$(( $(date +%s) - started ))" \
  "$(date -u +%FT%TZ)" > "$OUT/completion.txt"
find "$OUT" -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 sha256sum > "$OUT/SHA256SUMS"
