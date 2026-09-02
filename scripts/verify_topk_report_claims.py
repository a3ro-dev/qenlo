#!/usr/bin/env python3
"""Fail if the published valid-cell table disagrees with retained raw summary."""
import csv
from pathlib import Path

root = Path("benchmark-results/sota-a6000-topk-2026-09-02/cosine_inclusive")
report = Path("docs/reports/2026-09-02-topk-a6000-assessment.md").read_text(encoding="utf-8")
labels = {"qenlo_cuda_predicate_prototype": "Qenlo CUDA prototype",
          "faiss_gpu_flat_ip": "Faiss GPU Flat", "cuvs_brute_force_ip": "cuVS brute force"}
rows = list(csv.DictReader((root / "summary.csv").open()))
for system, label in labels.items():
    row = next(r for r in rows if r["system"] == system and r["int_filter_lte"] == "100")
    raw = list(csv.DictReader((root / {"qenlo_cuda_predicate_prototype": "qenlo", "faiss_gpu_flat_ip": "faiss", "cuvs_brute_force_ip": "cuvs"}[system] / "raw_samples.csv").open()))
    assert len([r for r in raw if r["threshold"] == "100"]) == 10_000
    assert row["mean_recall_at_10"] == "1.0" and row["min_recall_at_10"] == "1.0"
    table = f"| {label} | 1.0000 | {float(row['p50_ms']):.4f} | {float(row['p95_ms']):.4f} | {float(row['p99_ms']):.4f}"
    assert table in report, table
print("report claims match retained 1%-cell summary and 10,000 raw calls per system")
