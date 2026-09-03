#!/usr/bin/env bash
set -Eeuo pipefail

ROOT=${QENLO_REMOTE_ROOT:-/workspace/qenlo-campaign}
REPO="$ROOT/repo"
OUT="$ROOT/artifacts/phase0-baseline"
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
printf '%s\n' '3faf32e9308e4351f02d017ba16bc12ee479cb8b' > "$OUT/git-revision.txt"
sha256sum Cargo.lock Cargo.toml crates/qenlo/src/lib.rs crates/qenlo/src/gpu.rs \
  crates/qenlo/src/gpu_exact.wgsl crates/qenlo-bench/src/main.rs > "$OUT/source-checksums.txt"
sha256sum apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/src/main.rs \
  >> "$OUT/source-checksums.txt"
nvidia-smi -q > "$OUT/nvidia-smi.txt"
vulkaninfo --summary > "$OUT/vulkaninfo.txt"
rustc -Vv > "$OUT/rustc.txt"
cargo -V > "$OUT/cargo.txt"
lscpu > "$OUT/lscpu.txt"

cargo test -p qenlo --features gpu-wgpu gpu_exact_mask_smoke_if_adapter_is_available \
  -- --nocapture > "$OUT/gpu-test.log" 2>&1
cargo build --release -p qenlo-bench --features gpu-wgpu > "$OUT/build.log" 2>&1

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

# Alternate devices across the two headline eligible-count cells.
run_cell e2000-cpu cpu 2000
run_cell e2000-gpu-rows gpu-rows 2000
run_cell e3000-gpu-rows gpu-rows 3000
run_cell e3000-cpu cpu 3000

printf 'status=completed\ncompleted_utc=%s\n' "$(date -u +%FT%TZ)" > "$OUT/status.txt"
find "$OUT" -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 sha256sum > "$OUT/SHA256SUMS"
