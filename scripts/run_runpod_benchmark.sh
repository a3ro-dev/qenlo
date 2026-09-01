#!/usr/bin/env bash
set -Eeuo pipefail

ROOT=${1:-/workspace/qenlo-runpod}
REPO=${2:-/workspace/qenloDB}
OUT="$ROOT/artifacts"
DATA="$ROOT/data"
mkdir -p "$OUT/logs" "$DATA"
STARTED=$(date -u +%Y-%m-%dT%H:%M:%SZ)

capture() {
  local name=$1
  shift
  set +e
  "$@" > >(tee "$OUT/logs/$name.stdout.log") 2> >(tee "$OUT/logs/$name.stderr.log" >&2)
  local code=$?
  set -e
  printf '%s\n' "$code" > "$OUT/logs/$name.exit-code"
  return "$code"
}

{
  echo "started_utc=$STARTED"
  echo "repo_revision=$(git -C "$REPO" rev-parse HEAD)"
  echo "repo_status=$(git -C "$REPO" status --porcelain | wc -l) changed paths"
  uname -a
  lscpu
  free -h
  df -h
  nvidia-smi -q
  vulkaninfo --summary || true
} > "$OUT/environment.txt" 2>&1

python3 -m venv --system-site-packages "$ROOT/venv"
PY="$ROOT/venv/bin/python"
capture pip-install "$PY" -m pip install --upgrade pip
capture dependencies "$PY" -m pip install -r "$REPO/scripts/runpod-benchmark-requirements.txt"
"$PY" -m pip freeze --all > "$OUT/pip-freeze.txt"

capture embeddings "$PY" "$REPO/scripts/prepare_local_embeddings.py" \
  --output "$DATA/ag-news-local-100k-384.f32" --rows 100000 --tuning 100 --evaluation 500 --batch-size 512
CRC=$("$PY" -c 'import json,sys; print(json.load(open(sys.argv[1]))["output_hashes"]["crc32"])' \
  "$DATA/ag-news-local-100k-384.f32.json")
cp "$DATA/ag-news-local-100k-384.f32.json" "$OUT/embedding-provenance.json"

capture rust-toolchain bash -lc 'rustc -Vv; cargo -V; clang --version; cmake --version'
capture cargo-build cargo build --manifest-path "$REPO/Cargo.toml" --release -p qenlo-bench --features usearch,gpu-wgpu
BENCH="$REPO/target/release/qenlo-bench"
capture prepare "$BENCH" prepare --dataset "$DATA/ag-news-local-100k-384.qnb" \
  --input "$DATA/ag-news-local-100k-384.f32" --expect-crc32 "$CRC" \
  --rows 100000 --dimensions 384 --tuning 100 --evaluation 500 --seed 42

COMMON=(--dataset "$DATA/ag-news-local-100k-384.qnb" --dimensions 384 --distribution independent \
  --fraction 1 --batch 1 --warmups 50 --repetitions 3 --recall-target 0.95 \
  --diagnostics basic --vector-budget-mib 1024 --gpu-budget-mib 2048)
capture qenlo-cpu "$BENCH" run "${COMMON[@]}" --output "$OUT/qenlo-cpu" --backend cpu
capture qenlo-usearch "$BENCH" run "${COMMON[@]}" --output "$OUT/qenlo-usearch" --backend usearch \
  --tune-expansion-search 128,256,512,1024,2048 --oracle-reference "$OUT/qenlo-cpu" || true
capture qenlo-gpu "$BENCH" run "${COMMON[@]}" --output "$OUT/qenlo-gpu-predicate" --backend gpu-predicate \
  --oracle-reference "$OUT/qenlo-cpu" || true

capture chroma "$PY" "$REPO/scripts/chroma_replay.py" --reference "$OUT/qenlo-cpu" \
  --dataset "$DATA/ag-news-local-100k-384.qnb" --output "$OUT/chroma" --ef-search 256 || true
for backend in faiss-flat faiss-hnsw qdrant milvus lancedb-flat; do
  capture "$backend" "$PY" "$REPO/scripts/oss_replay.py" --backend "$backend" \
    --reference "$OUT/qenlo-cpu" --dataset "$DATA/ag-news-local-100k-384.qnb" \
    --output "$OUT/$backend" --warmups 50 || true
done

{
  echo "started_utc=$STARTED"
  echo "finished_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  find "$OUT" -name 'summary.txt' -o -name 'summary.json' -o -name 'failure.json' -o -name 'tuning-failure.json'
} > "$OUT/completion.txt"
tar --exclude='*/database' --exclude='*/database/*' -C "$ROOT" -czf "$ROOT/qenlo-runpod-artifacts.tar.gz" artifacts
sha256sum "$ROOT/qenlo-runpod-artifacts.tar.gz" > "$ROOT/qenlo-runpod-artifacts.tar.gz.sha256"
echo "benchmark suite finished: $ROOT/qenlo-runpod-artifacts.tar.gz"
