#!/usr/bin/env bash
set -euo pipefail
# Run inside a CUDA-enabled Python image after installing faiss-gpu-cu12.
python research/scripts/run_cuda_synthetic_matrix.py \
  --output research/data/raw/a6000-tuning --seed 20260902
python research/scripts/run_cuda_synthetic_matrix.py \
  --output research/data/raw/a6000-heldout --seed 20260903
python research/scripts/analyze_results.py
python research/scripts/generate_plots.py
