# Reproducing the paper

Use immutable tag `paper-adaptive-routing-v1`. The tag contains the exact Rust
source, `Cargo.lock`, analysis and plotting scripts, paper source, processed data,
and retained raw archives used by this revision.

## Verify code and retained results

```sh
cargo test --workspace --no-default-features -- --test-threads=1
cargo test -p qenlo-bench --no-default-features
cargo test -p qenlo --features gpu-wgpu -- --test-threads=1
python -m unittest discover -s scripts -p "test_*.py"
python scripts/verify_strict_research_gate.py --help
python research/scripts/analyze_results.py
python research/scripts/generate_plots.py
```

## Re-run the CUDA development and held-out cohorts

On a single RTX A6000 CUDA 12.8 host with Python 3.12, PyTorch 2.9.1, NumPy 2.x, and `faiss-gpu-cu12==1.14.1.post1`:

```sh
python research/scripts/run_cuda_synthetic_matrix.py --output research/data/raw/a6000-tuning --seed 20260902
python research/scripts/run_cuda_synthetic_matrix.py --output research/data/raw/a6000-heldout --seed 20260903
```

Each output directory must be new. Do not overwrite the retained run. The script generates normalized FP32 vectors, uses 100 queries, 50 warmups, five repetitions, and E={1k,10k,100k,1M}. Timing includes query upload, GPU execution, synchronization, and ten-ID readback; preparation is excluded.

## Re-run the native crossover sweep

Using the prepared `data/ag-news/ag-news-100k-384.qnb` corpus on the recorded RTX 4050/Vulkan host, run both `cpu` and `gpu-rows` backends for eligible counts 2000, 3000, 4000, 6000, 8000, and 10000. Each invocation uses:

```sh
cargo run --release -p qenlo-bench --features gpu-wgpu -- run \
  --dataset data/ag-news/ag-news-100k-384.qnb --output NEW_OUTPUT_DIR \
  --dimensions 384 --backend BACKEND --distribution independent --fraction 1 \
  --eligible-count ELIGIBLE --batch 1 --warmups 200 --repetitions 5 \
  --recall-target 0.95
```

Use new output directories. The retained samples and exact commands are recorded under `research/data/raw/2026-09-02-native-crossover`.

## Re-run the eligibility-materialization ablation

Do not run this campaign on an uncontrolled laptop if the paper result is the
target. On a Vulkan-capable NVIDIA Runpod, prepare the retained AG News corpus,
copy the tagged repository, then run:

```sh
export QENLO_REMOTE_ROOT=/workspace/qenlo-phase1
bash scripts/runpod/bootstrap.sh compatibility
bash scripts/runpod/phase1-eligibility-ablation.sh
```

The script writes new cell directories and never overwrites the committed raw
archive. The authoritative archive is
`research/artifacts/qenlo-phase1-eligibility-2026-09-04.tar.gz`, SHA-256
`26e0ccc057c59c0fb4687f8d85a923e88b7bb25257e4e551e22cffe13c1b821f`.
It includes exact commands, source patch, environment capture, raw calls,
complete-run summaries, oracle output, failures, and internal checksums.

## Compile

The Android device-lab cohort was supplied as schema-v1 app output. Its exact run IDs and transcription limitations are recorded in `research/data/raw/2026-09-02-android/PROVENANCE.md`; the 21-cell table is `research/data/processed/android_device_lab.csv`.

```sh
cd paper
pdflatex paper.tex
bibtex paper
pdflatex paper.tex
pdflatex paper.tex
cp paper.pdf output/pdf/qenlo-routing-filtered-vector-search.pdf
```

The validated revision was compiled with MiKTeX pdfTeX 1.40.28 and visually
checked from Poppler page renders. `scripts/runpod/bootstrap.sh reference` pins
the paper-analysis Python packages used in the retained Linux reproduction
environment. Tectonic remains an alternative: run
`tectonic paper.tex --keep-logs` from the `paper` directory.
