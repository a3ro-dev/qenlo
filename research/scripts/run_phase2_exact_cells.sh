#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 QENLO_BENCH DATASET OUTPUT_ROOT" >&2
  exit 2
fi

bench=$1
dataset=$2
output_root=$3
warmups=${QENLO_WARMUPS:-200}
repetitions=${QENLO_REPETITIONS:-5}
repo_root=$(cd "$(dirname "$dataset")/../.." && pwd)

mkdir -p "$output_root"
cd "$repo_root"
for eligible in 1000 4000 100000; do
  output="$output_root/cpu-e${eligible}"
  command=(
    "$bench" run
    --dataset "$dataset"
    --dimensions 384
    --distribution independent
    --backend cpu
    --eligible-count "$eligible"
    --batch 1
    --warmups "$warmups"
    --repetitions "$repetitions"
    --recall-target 0.99
    --diagnostics detailed
    --vector-budget-mib 1024
    --gpu-budget-mib 2048
    --output "$output"
  )
  printf '%q ' "${command[@]}" > "$output_root/cpu-e${eligible}.command.txt"
  printf '\n' >> "$output_root/cpu-e${eligible}.command.txt"
  "${command[@]}" > "$output_root/cpu-e${eligible}.stdout.txt" \
    2> "$output_root/cpu-e${eligible}.stderr.txt"
done
