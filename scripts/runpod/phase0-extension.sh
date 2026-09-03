#!/usr/bin/env bash
set -Eeuo pipefail

ROOT=${QENLO_REMOTE_ROOT:-/workspace/qenlo-campaign}
REPO="$ROOT/repo"
OUT="$ROOT/artifacts/phase0-extension"
DATA="$REPO/data/ag-news/ag-news-100k-384.qnb"
mkdir -p "$OUT"

on_error() {
  status=$?
  printf 'status=failed\nexit_code=%s\nfailed_utc=%s\n' "$status" "$(date -u +%FT%TZ)" > "$OUT/status.txt"
  exit "$status"
}
trap on_error ERR

source /root/.cargo/env
export QENLO_REQUIRE_GPU=1
export WGPU_BACKEND=vulkan
export RAYON_NUM_THREADS=1
export RUST_BACKTRACE=1
export RUSTUP_TOOLCHAIN=1.98.0

cd "$REPO"
BIN="$REPO/target/release/qenlo-bench"
COMMON=(
  run --dataset "$DATA" --dimensions 384 --distribution independent
  --batch 1 --warmups 200 --repetitions 5 --recall-target 0.99
  --diagnostics basic --vector-budget-mib 1024 --gpu-budget-mib 2048
)

run_cell() {
  name=$1
  backend=$2
  eligible=$3
  printf '%q ' "$BIN" "${COMMON[@]}" --output "$OUT/$name" --backend "$backend" --eligible-count "$eligible" \
    > "$OUT/$name.command.txt"
  printf '\n' >> "$OUT/$name.command.txt"
  "$BIN" "${COMMON[@]}" --output "$OUT/$name" --backend "$backend" --eligible-count "$eligible" \
    > "$OUT/$name.stdout.txt" 2> "$OUT/$name.stderr.txt"
}

run_cell e1000-cpu cpu 1000
run_cell e1000-gpu-rows gpu-rows 1000
run_cell e1000-gpu-predicate gpu-predicate 1000

for eligible in 4000 6000 8000 10000; do
  run_cell "e${eligible}-gpu-rows" gpu-rows "$eligible"
  run_cell "e${eligible}-cpu" cpu "$eligible"
done

run_cell e100000-gpu-predicate gpu-predicate 100000
run_cell e100000-cpu cpu 100000
run_cell e100000-gpu-rows gpu-rows 100000
run_cell e100000-gpu-mask gpu-mask 100000

printf 'status=completed\ncompleted_utc=%s\n' "$(date -u +%FT%TZ)" > "$OUT/status.txt"
find "$OUT" -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 sha256sum > "$OUT/SHA256SUMS"
