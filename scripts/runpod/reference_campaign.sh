#!/usr/bin/env bash
set -Eeuo pipefail

ROOT=${QENLO_REMOTE_ROOT:-/workspace/qenlo-campaign}
REPO="$ROOT/repo"
OUT="$ROOT/artifacts/reference"
DATA="$REPO/data/ag-news/ag-news-100k-384.qnb"
BENCH="$REPO/target/release/qenlo-bench"
mkdir -p "$OUT/logs" "$OUT/runs"
export QENLO_REQUIRE_GPU=1 WGPU_BACKEND=vulkan RAYON_NUM_THREADS=1 OMP_NUM_THREADS=1

bash "$REPO/scripts/runpod/bootstrap.sh" reference
cd "$REPO"
cargo build --release -p qenlo-bench --features usearch,gpu-wgpu
sha256sum "$BENCH" "$DATA" > "$OUT/input-checksums.txt"
seq 0 19 | awk '{printf "%d,%d\n",$1,424242+$1*104729}' > "$OUT/headline-seeds.csv"

run_logged() {
  local name=$1
  shift
  local destination="$OUT/runs/$name"
  [[ -f "$destination/result/summary.txt" || -f "$destination/result/summary.json" ]] && return 0
  mkdir -p "$destination"
  printf '%q ' "$@" > "$destination/command.txt"
  printf '\n' >> "$destination/command.txt"
  set +e
  /usr/bin/time -v -o "$destination/resources.txt" timeout --signal=TERM 25m "$@" \
    > "$destination/stdout.log" 2> "$destination/stderr.log"
  local code=$?
  set -e
  echo "$code" > "$destination/exit-code.txt"
  return "$code"
}

qenlo() {
  local name=$1 backend=$2 eligible=$3 seed=$4 mode=${5:-one-pass}
  local reference_args=()
  [[ -n "${ORACLE_REFERENCE:-}" ]] && reference_args=(--oracle-reference "$ORACLE_REFERENCE")
  run_logged "$name" "$BENCH" run --dataset "$DATA" --output "$OUT/runs/$name/result" \
    --dimensions 384 --backend "$backend" --gpu-row-preparation "$mode" \
    --distribution independent --eligible-count "$eligible" --batch 1 --warmups 200 \
    --repetitions 1 --order-seed "$seed" --recall-target 0.99 --diagnostics basic \
    --vector-budget-mib 1024 --gpu-budget-mib 2048 "${reference_args[@]}"
}

# Establish one immutable FP64 truth set. The run is also headline replicate zero.
qenlo headline-dense-cpu-r00 cpu 100000 424242
ORACLE_REFERENCE="$OUT/runs/headline-dense-cpu-r00/result"
export ORACLE_REFERENCE

# Matched dense Qenlo CPU and FAISS IndexFlatIP, one fresh process per replicate.
for replicate in $(seq 0 19); do
  seed=$((424242 + replicate * 104729))
  if (( replicate > 0 )); then
    qenlo "headline-dense-cpu-r$(printf '%02d' "$replicate")" cpu 100000 "$seed"
  fi
  run_logged "headline-faiss-flat-r$(printf '%02d' "$replicate")" \
    python3 "$REPO/scripts/oss_replay.py" --backend faiss-flat \
    --reference "$OUT/runs/headline-dense-cpu-r$(printf '%02d' "$replicate")/result" \
    --dataset "$DATA" --output "$OUT/runs/headline-faiss-flat-r$(printf '%02d' "$replicate")/result" \
    --warmups 200 --threads 1
done

# Compact-row discovery and preregistered 20-process headline cells.
for eligible in 1000 2000 3000 5000 10000 25000 50000 100000; do
  processes=5
  [[ "$eligible" == 2000 || "$eligible" == 3000 ]] && processes=20
  for replicate in $(seq 0 $((processes - 1))); do
    seed=$((700001 + eligible * 17 + replicate * 104729))
    systems=(cpu:one-pass gpu-rows:legacy-two-pass gpu-rows:one-pass gpu-rows:cached)
    (( seed % 2 )) && systems=(gpu-rows:cached gpu-rows:one-pass gpu-rows:legacy-two-pass cpu:one-pass)
    for system in "${systems[@]}"; do
      backend=${system%%:*}; mode=${system##*:}
      qenlo "compact-e${eligible}-${backend}-${mode}-r$(printf '%02d' "$replicate")" \
        "$backend" "$eligible" "$seed" "$mode"
    done
  done
done

# Complete ANN recall-latency curves. Recall misses are data, not failed runs.
for eligible in 1000 2000 3000 10000 25000 50000 100000; do
  for expansion in 16 32 64 128 256 512 1024; do
    for replicate in $(seq 0 4); do
      seed=$((900001 + eligible * 19 + expansion * 101 + replicate * 104729))
      name="ann-e${eligible}-ef${expansion}-r$(printf '%02d' "$replicate")"
      run_logged "$name" "$BENCH" run --dataset "$DATA" --output "$OUT/runs/$name/result" \
        --dimensions 384 --backend usearch --distribution independent --eligible-count "$eligible" \
        --batch 1 --warmups 200 --repetitions 1 --order-seed "$seed" --recall-target 0.99 \
        --expansion-search "$expansion" --tune-expansion-search "$expansion" \
        --allow-recall-miss true --oracle-reference "$ORACLE_REFERENCE" --diagnostics basic \
        --vector-budget-mib 1024 --gpu-budget-mib 2048
    done
  done
done

# Remaining dense headline systems. The analysis freezes the smallest tuning expansion at recall >= .99.
for replicate in $(seq 0 19); do
  seed=$((1200001 + replicate * 104729))
  qenlo "headline-dense-gpu-predicate-r$(printf '%02d' "$replicate")" gpu-predicate 100000 "$seed"
  name="headline-dense-usearch-r$(printf '%02d' "$replicate")"
  run_logged "$name" "$BENCH" run --dataset "$DATA" --output "$OUT/runs/$name/result" \
    --dimensions 384 --backend usearch --distribution independent --eligible-count 100000 \
    --batch 1 --warmups 200 --repetitions 1 --order-seed "$seed" --recall-target 0.99 \
    --expansion-search 16 --tune-expansion-search 16,32,64,128,256,512,1024 \
    --oracle-reference "$ORACLE_REFERENCE" --diagnostics basic \
    --vector-budget-mib 1024 --gpu-budget-mib 2048
done

python3 "$REPO/research/scripts/analyze_runpod_campaign.py" --input "$OUT" --output "$OUT/analysis"
find "$OUT" -type f -print0 | sort -z | xargs -0 sha256sum > "$OUT/SHA256SUMS"
echo "reference_campaign=complete" > "$OUT/COMPLETE"
