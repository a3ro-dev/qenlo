#!/usr/bin/env bash
set -Eeuo pipefail

ROOT=${QENLO_REMOTE_ROOT:-/workspace/qenlo-phase1}
REPO="$ROOT/repo"
OUT="$ROOT/artifacts"
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

cd "$REPO"
cargo build --release -p qenlo-bench --features gpu-wgpu
BIN="$REPO/target/release/qenlo-bench"
COMMON=(
  run --dataset "$DATA" --dimensions 384 --distribution independent
  --backend gpu-rows --batch 1 --warmups 200 --repetitions 5
  --recall-target 0.99 --diagnostics basic
  --vector-budget-mib 1024 --gpu-budget-mib 2048
)

run_cell() {
  eligible=$1
  preparation=$2
  name="e${eligible}-${preparation}"
  printf '%q ' "$BIN" "${COMMON[@]}" --eligible-count "$eligible" \
    --gpu-row-preparation "$preparation" --output "$OUT/$name" > "$OUT/$name.command.txt"
  printf '\n' >> "$OUT/$name.command.txt"
  "$BIN" "${COMMON[@]}" --eligible-count "$eligible" \
    --gpu-row-preparation "$preparation" --output "$OUT/$name" \
    > "$OUT/$name.stdout.txt" 2> "$OUT/$name.stderr.txt"
}

for eligible in 1000 4000 6000 100000; do
  for preparation in legacy-two-pass one-pass cached; do
    run_cell "$eligible" "$preparation"
  done
done

printf 'status=completed\ncompleted_utc=%s\n' "$(date -u +%FT%TZ)" > "$OUT/status.txt"
find "$OUT" -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 sha256sum > "$OUT/SHA256SUMS"
