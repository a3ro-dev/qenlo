#!/usr/bin/env bash
set -Eeuo pipefail

ROOT=${QENLO_REMOTE_ROOT:-/workspace/qenlo-deep}
CURRENT="$ROOT/current"
BASELINE="$ROOT/baseline"
OUT="$ROOT/artifacts"
DATA="$ROOT/datasets"
CURRENT_TARGET="$ROOT/target-current"
BASELINE_TARGET="$ROOT/target-baseline"
STATUS="$OUT/status.tsv"
SOURCE_SHA=${QENLO_SOURCE_BUNDLE_SHA256:?source bundle SHA-256 is required}
BASELINE_SHA=${QENLO_BASELINE_BUNDLE_SHA256:?baseline bundle SHA-256 is required}
WORKLOAD_SHA=${QENLO_WORKLOAD_SCRIPT_SHA256:?workload script SHA-256 is required}
REAL_DATASET=${QENLO_REAL_DATASET:-/workspace/ag-news-100k-384.qnb}
REAL_SHA=${QENLO_REAL_DATASET_SHA256:?real dataset SHA-256 is required}

for value in "$SOURCE_SHA" "$BASELINE_SHA" "$WORKLOAD_SHA" "$REAL_SHA"; do
  [[ "$value" =~ ^[0-9a-f]{64}$ ]] || { echo "invalid SHA-256 input" >&2; exit 2; }
done
[[ $(sha256sum "${BASH_SOURCE[0]}" | cut -d' ' -f1) == "$WORKLOAD_SHA" ]] || {
  echo "workload script SHA-256 mismatch" >&2
  exit 2
}
[[ $(sha256sum "$REAL_DATASET" | cut -d' ' -f1) == "$REAL_SHA" ]] || {
  echo "real dataset SHA-256 mismatch" >&2
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
  echo "captured_utc=$(date -u +%FT%TZ)"
  echo "current_source_bundle_sha256=$SOURCE_SHA"
  echo "baseline_source_bundle_sha256=$BASELINE_SHA"
  echo "workload_script_sha256=$WORKLOAD_SHA"
  echo "real_dataset_sha256=$REAL_SHA"
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
CARGO_TARGET_DIR="$CURRENT_TARGET" cargo build --manifest-path "$CURRENT/Cargo.toml" \
  --release -p qenlo-bench --features gpu-wgpu,usearch -j 1
build_finished=$(date +%s%N)
CARGO_TARGET_DIR="$BASELINE_TARGET" cargo build --manifest-path "$BASELINE/Cargo.toml" \
  --release -p qenlo-bench --features gpu-wgpu -j 1
baseline_finished=$(date +%s%N)
printf 'current_build_ns=%s\nbaseline_build_ns=%s\n' \
  "$((build_finished-build_started))" "$((baseline_finished-build_finished))" > "$OUT/build-times.txt"

CURRENT_BENCH="$CURRENT_TARGET/release/qenlo-bench"
BASELINE_BENCH="$BASELINE_TARGET/release/qenlo-bench"

prepare_dataset() {
  local rows=$1 dimension=$2 path="$DATA/r${1}-d${2}.qnb"
  if [[ ! -f "$path" ]]; then
    "$CURRENT_BENCH" prepare --dataset "$path" --rows "$rows" --dimensions "$dimension" \
      --tuning 16 --evaluation 64 --seed 20260905 >&2
  fi
  printf '%s' "$path"
}

run_native() {
  local cell=$1 engine=$2 binary=$3 backend=$4 dataset=$5 dimension=$6 batch=$7 k=$8 fraction=$9
  shift 9
  local output="$OUT/runs/$cell/$engine"
  mkdir -p "$(dirname "$output")"
  record_command deep "$cell" "$engine" "$OUT/runs/$cell/$engine.log" \
    /usr/bin/time -v -o "$OUT/runs/$cell/$engine.time" \
    "$binary" run --dataset "$dataset" --output "$output" --dimensions "$dimension" \
    --backend "$backend" --distribution independent --fraction "$fraction" --batch "$batch" \
    --k "$k" --warmups 8 --repetitions 3 --order-seed 20260905 --recall-target 0.99 \
    --diagnostics detailed --vector-budget-mib 1024 --gpu-budget-mib 1024 "$@"
}

run_baseline() {
  local cell=$1 engine=$2 backend=$3 dataset=$4 dimension=$5 batch=$6 fraction=$7
  shift 7
  local output="$OUT/runs/$cell/$engine"
  mkdir -p "$(dirname "$output")"
  # The frozen revision has fixed k=10 semantics and therefore has no --k flag.
  record_command deep "$cell" "$engine" "$OUT/runs/$cell/$engine.log" \
    /usr/bin/time -v -o "$OUT/runs/$cell/$engine.time" \
    "$BASELINE_BENCH" run --dataset "$dataset" --output "$output" --dimensions "$dimension" \
    --backend "$backend" --distribution independent --fraction "$fraction" --batch "$batch" \
    --warmups 8 --repetitions 3 --order-seed 20260905 --recall-target 0.99 \
    --diagnostics detailed --vector-budget-mib 1024 --gpu-budget-mib 1024 "$@"
}

run_replays() {
  local cell=$1 dataset=$2 reference=$3 k=$4 engine output
  for engine in numpy faiss-flat torch-cpu torch-cuda; do
    output="$OUT/runs/$cell/$engine"
    record_command deep "$cell" "$engine" "$OUT/runs/$cell/$engine.log" \
      python3 "$CURRENT/scripts/benchmark_small_vectors.py" --backend "$engine" \
      --reference "$reference" --dataset "$dataset" --output "$output" --threads 2 || true
  done
  if [[ "$k" == 10 ]]; then
    output="$OUT/runs/$cell/chroma-ef512"
    record_command deep "$cell" chroma-ef512 "$OUT/runs/$cell/chroma-ef512.log" \
      python3 "$CURRENT/scripts/chroma_replay.py" --reference "$reference" --dataset "$dataset" \
      --output "$output" --ef-search 512 --threads 2 || true
    [[ ! -d "$output/database" ]] || rm -rf -- "$output/database"
  fi
}

run_cell() {
  local cell=$1 rows=$2 dimension=$3 batch=$4 k=$5 fraction=$6 user=${7:-}
  local dataset cpu gpu extra=()
  dataset=$(prepare_dataset "$rows" "$dimension")
  [[ -z "$user" ]] || extra+=(--user-id "$user")
  cpu="$OUT/runs/$cell/current-cpu"
  gpu="$OUT/runs/$cell/current-gpu"
  run_native "$cell" current-cpu "$CURRENT_BENCH" cpu "$dataset" "$dimension" "$batch" "$k" "$fraction" "${extra[@]}" || return 0
  run_native "$cell" current-gpu "$CURRENT_BENCH" gpu-rows "$dataset" "$dimension" "$batch" "$k" "$fraction" \
    --gpu-row-preparation cached --oracle-reference "$cpu" "${extra[@]}" || true
  run_replays "$cell" "$dataset" "$cpu" "$k"
  if [[ "$k" == 10 ]]; then
    run_native "$cell" usearch "$CURRENT_BENCH" usearch "$dataset" "$dimension" "$batch" "$k" "$fraction" \
      --tune-expansion-search 128,256,512,1024 --allow-recall-miss true --oracle-reference "$cpu" "${extra[@]}" || true
  fi
  if [[ "$k" == 10 && ( "$batch" == 1 || "$batch" == 8 ) ]]; then
    local baseline_cpu="$OUT/runs/$cell/baseline-cpu"
    run_baseline "$cell" baseline-cpu cpu "$dataset" "$dimension" "$batch" "$fraction" "${extra[@]}" || return 0
    run_baseline "$cell" baseline-gpu gpu-rows "$dataset" "$dimension" "$batch" "$fraction" \
      --gpu-row-preparation one-pass --oracle-reference "$baseline_cpu" "${extra[@]}" || true
    printf '%s\n' "$BASELINE_SHA" > "$OUT/runs/$cell/baseline-source-bundle.sha256"
  fi
}

run_cell d1-r5000-d384-b8-k10-f01 5000 384 8 10 0.1
run_cell d2-r25000-d384-b64-k10-f1 25000 384 64 10 1
run_cell d3-r50000-d384-b1-k10-compound 50000 384 1 10 0.1 7
run_cell d4-r100000-d768-b1-k1-f1 100000 768 1 1 1
run_cell d5-r100000-d768-b8-k64-f01 100000 768 8 64 0.1
run_cell d6-r25000-d768-b64-k64-f001 25000 768 64 64 0.01

real_cell=real-r100000-d384-b16-k10-f001
real_cpu="$OUT/runs/$real_cell/current-cpu"
run_native "$real_cell" current-cpu "$CURRENT_BENCH" cpu "$REAL_DATASET" 384 16 10 0.01 || true
if [[ -d "$real_cpu" ]]; then
  run_native "$real_cell" current-gpu "$CURRENT_BENCH" gpu-rows "$REAL_DATASET" 384 16 10 0.01 \
    --gpu-row-preparation cached --oracle-reference "$real_cpu" || true
  run_replays "$real_cell" "$REAL_DATASET" "$real_cpu" 10
  run_native "$real_cell" usearch "$CURRENT_BENCH" usearch "$REAL_DATASET" 384 16 10 0.01 \
    --tune-expansion-search 128,256,512,1024 --allow-recall-miss true --oracle-reference "$real_cpu" || true
fi

printf 'elapsed_seconds=%s\ncompleted_utc=%s\n' "$(( $(date +%s) - started ))" \
  "$(date -u +%FT%TZ)" > "$OUT/completion.txt"
find "$OUT" -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 sha256sum > "$OUT/SHA256SUMS"
