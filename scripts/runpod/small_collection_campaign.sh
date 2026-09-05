#!/usr/bin/env bash
set -Eeuo pipefail

MODE=${1:-pilot}
ROOT=${QENLO_REMOTE_ROOT:-/workspace/qenlo-small}
CURRENT="$ROOT/current"
BASELINE="$ROOT/baseline"
OUT="$ROOT/artifacts"
DATA="$ROOT/datasets"
CURRENT_TARGET="$ROOT/target-current"
BASELINE_TARGET="$ROOT/target-baseline"
STATUS="$OUT/status.tsv"
SOURCE_SHA=${QENLO_SOURCE_BUNDLE_SHA256:?source bundle SHA-256 is required}
BASELINE_SHA=${QENLO_BASELINE_BUNDLE_SHA256:?baseline bundle SHA-256 is required}

if [[ ! "$SOURCE_SHA" =~ ^[0-9a-f]{64}$ || ! "$BASELINE_SHA" =~ ^[0-9a-f]{64}$ ]]; then
  echo "invalid source archive digest" >&2
  exit 2
fi
if [[ "$MODE" != pilot && "$MODE" != common && "$MODE" != reference ]]; then
  echo "mode must be pilot, common, or reference" >&2
  exit 2
fi

mkdir -p "$OUT" "$DATA"
printf 'stage\tcell\tengine\tstatus\texit_code\n' > "$STATUS"
exec > >(tee "$OUT/remote.log") 2>&1
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
  build-essential ca-certificates clang curl git libclang-dev libegl1 libgl1 \
  libglvnd0 libglx0 libvulkan1 pkg-config python3-venv time vulkan-tools

if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi
source /root/.cargo/env
rustup toolchain install 1.98.0 --profile minimal
rustup default 1.98.0

python3 -m venv --system-site-packages "$ROOT/venv"
source "$ROOT/venv/bin/activate"
python3 -m pip install --disable-pip-version-check --no-cache-dir \
  numpy==2.3.3 faiss-cpu==1.15.0 chromadb==1.5.9 psutil==7.0.0

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
  echo "mode=$MODE"
  echo "captured_utc=$(date -u +%FT%TZ)"
  echo "current_source_bundle_sha256=$SOURCE_SHA"
  echo "baseline_source_bundle_sha256=$BASELINE_SHA"
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

export CARGO_BUILD_JOBS=1
export QENLO_REQUIRE_GPU=1
export WGPU_BACKEND=vulkan
export OMP_NUM_THREADS=2 MKL_NUM_THREADS=2 OPENBLAS_NUM_THREADS=2
export QENLO_SOURCE_BUNDLE_SHA256="$SOURCE_SHA"

build_started=$(date +%s%N)
CARGO_TARGET_DIR="$CURRENT_TARGET" cargo build --manifest-path "$CURRENT/Cargo.toml" \
  --release -p qenlo-bench --features gpu-wgpu -j 1
build_finished=$(date +%s%N)
printf 'current_build_ns=%s\n' "$((build_finished-build_started))" > "$OUT/build-times.txt"

baseline_build_started=$(date +%s%N)
CARGO_TARGET_DIR="$BASELINE_TARGET" cargo build --manifest-path "$BASELINE/Cargo.toml" \
  --release -p qenlo-bench --features gpu-wgpu -j 1
baseline_build_finished=$(date +%s%N)
printf 'baseline_build_ns=%s\n' "$((baseline_build_finished-baseline_build_started))" >> "$OUT/build-times.txt"

CURRENT_BENCH="$CURRENT_TARGET/release/qenlo-bench"
BASELINE_BENCH="$BASELINE_TARGET/release/qenlo-bench"

prepare_dataset() {
  local rows=$1 dimension=$2
  local path="$DATA/r${rows}-d${dimension}.qnb"
  if [[ ! -f "$path" ]]; then
    "$CURRENT_BENCH" prepare --dataset "$path" --rows "$rows" --dimensions "$dimension" \
      --tuning 16 --evaluation 64 --seed 20260905 >&2
  fi
  printf '%s' "$path"
}

run_current_gpu() {
  local cell=$1 rows=$2 dimension=$3 batch=$4 fraction=$5
  local dataset output
  dataset=$(prepare_dataset "$rows" "$dimension")
  output="$OUT/runs/$cell/current-gpu"
  mkdir -p "$(dirname "$output")"
  record_command common "$cell" current-gpu "$OUT/runs/$cell/current-gpu.log" \
    /usr/bin/time -v -o "$OUT/runs/$cell/current-gpu.time" \
    "$CURRENT_BENCH" run --dataset "$dataset" --output "$output" --dimensions "$dimension" \
    --backend gpu-rows --gpu-row-preparation cached --distribution independent \
    --fraction "$fraction" --batch "$batch" --k 10 --warmups 8 --repetitions 3 \
    --order-seed 20260905 --recall-target 0.99 --diagnostics detailed \
    --vector-budget-mib 512 --gpu-budget-mib 512
}

run_baseline_pair() {
  local cell=$1 rows=$2 dimension=$3 batch=$4 fraction=$5
  local dataset cpu gpu
  dataset=$(prepare_dataset "$rows" "$dimension")
  cpu="$OUT/runs/$cell/baseline-cpu"
  gpu="$OUT/runs/$cell/baseline-gpu"
  mkdir -p "$(dirname "$cpu")"
  record_command baseline "$cell" baseline-cpu "$OUT/runs/$cell/baseline-cpu.log" \
    /usr/bin/time -v -o "$OUT/runs/$cell/baseline-cpu.time" \
    "$BASELINE_BENCH" run --dataset "$dataset" --output "$cpu" --dimensions "$dimension" \
    --backend cpu --distribution independent --fraction "$fraction" --batch "$batch" \
    --warmups 8 --repetitions 3 --order-seed 20260905 --recall-target 0.99 \
    --diagnostics detailed --vector-budget-mib 512 --gpu-budget-mib 512 || return 0
  record_command baseline "$cell" baseline-gpu "$OUT/runs/$cell/baseline-gpu.log" \
    /usr/bin/time -v -o "$OUT/runs/$cell/baseline-gpu.time" \
    "$BASELINE_BENCH" run --dataset "$dataset" --output "$gpu" --dimensions "$dimension" \
    --backend gpu-rows --gpu-row-preparation one-pass --distribution independent \
    --fraction "$fraction" --batch "$batch" --warmups 8 --repetitions 3 \
    --order-seed 20260905 --recall-target 0.99 --oracle-reference "$cpu" \
    --diagnostics detailed --vector-budget-mib 512 --gpu-budget-mib 512 || true
  printf '%s\n' "$BASELINE_SHA" > "$OUT/runs/$cell/baseline-source-bundle.sha256"
}

run_reference_engines() {
  local cell=$1 rows=$2 dimension=$3 batch=$4 fraction=$5
  local dataset cpu engine output
  dataset=$(prepare_dataset "$rows" "$dimension")
  cpu="$OUT/runs/$cell/current-cpu"
  record_command reference "$cell" current-cpu "$OUT/runs/$cell/current-cpu.log" \
    /usr/bin/time -v -o "$OUT/runs/$cell/current-cpu.time" \
    "$CURRENT_BENCH" run --dataset "$dataset" --output "$cpu" --dimensions "$dimension" \
    --backend cpu --distribution independent --fraction "$fraction" --batch "$batch" --k 10 \
    --warmups 8 --repetitions 3 --order-seed 20260905 --recall-target 0.99 \
    --diagnostics detailed --vector-budget-mib 512 --gpu-budget-mib 512 || return 0
  for engine in numpy faiss-flat torch-cpu torch-cuda; do
    output="$OUT/runs/$cell/$engine"
    record_command reference "$cell" "$engine" "$OUT/runs/$cell/$engine.log" \
      python3 "$CURRENT/scripts/benchmark_small_vectors.py" --backend "$engine" \
      --reference "$cpu" --dataset "$dataset" --output "$output" --threads 2 || true
  done
  output="$OUT/runs/$cell/chroma"
  record_command reference "$cell" chroma "$OUT/runs/$cell/chroma.log" \
    python3 "$CURRENT/scripts/chroma_replay.py" --reference "$cpu" --dataset "$dataset" \
    --output "$output" --ef-search 128 --threads 2 || true
  if [[ -d "$output/database" ]]; then
    rm -rf -- "$output/database"
  fi
}

run_lifecycle() {
  local dataset output
  dataset=$(prepare_dataset 10000 384)
  output="$OUT/lifecycle-current-gpu"
  record_command reference lifecycle current-gpu "$OUT/lifecycle-current-gpu.log" \
    /usr/bin/time -v -o "$OUT/lifecycle-current-gpu.time" \
    "$CURRENT_BENCH" lifecycle --dataset "$dataset" --output "$output" --dimensions 384 \
    --backend gpu-rows --repetitions 3 --write-batch 8 \
    --vector-budget-mib 512 --gpu-budget-mib 512 || true
}

if [[ "$MODE" == pilot ]]; then
  run_current_gpu p1-r1000-d128-b1-f1 1000 128 1 1
  run_baseline_pair p1-r1000-d128-b1-f1 1000 128 1 1
else
  cells=(
    "c1-r1000-d128-b1-f1 1000 128 1 1"
    "c2-r1000-d384-b16-f001 1000 384 16 0.01"
    "c3-r10000-d128-b16-f1 10000 128 16 1"
    "c4-r10000-d384-b1-f001 10000 384 1 0.01"
    "c5-r100000-d128-b1-f001 100000 128 1 0.01"
    "c6-r100000-d384-b16-f1 100000 384 16 1"
  )
  for spec in "${cells[@]}"; do
    read -r cell rows dimension batch fraction <<< "$spec"
    run_current_gpu "$cell" "$rows" "$dimension" "$batch" "$fraction" || true
    if [[ "$batch" == 1 ]]; then
      run_baseline_pair "$cell" "$rows" "$dimension" "$batch" "$fraction"
    fi
    if [[ "$MODE" == reference ]]; then
      run_reference_engines "$cell" "$rows" "$dimension" "$batch" "$fraction"
    fi
  done
  if [[ "$MODE" == reference ]]; then
    run_lifecycle
  fi
fi

printf 'elapsed_seconds=%s\n' "$(( $(date +%s) - started ))" > "$OUT/completion.txt"
printf 'completed_utc=%s\n' "$(date -u +%FT%TZ)" >> "$OUT/completion.txt"
find "$OUT" -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 sha256sum > "$OUT/SHA256SUMS"
