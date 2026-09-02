#!/usr/bin/env python3
"""Render a clearly validity-labelled P95 plot from retained CSV summary."""
import csv
from pathlib import Path
import matplotlib.pyplot as plt

root = Path("benchmark-results/sota-a6000-topk-2026-09-02/cosine_inclusive")
rows = list(csv.DictReader((root / "summary.csv").open()))
labels = {"qenlo_cuda_predicate_prototype": "Qenlo CUDA prototype",
          "faiss_gpu_flat_ip": "Faiss GPU Flat", "cuvs_brute_force_ip": "cuVS brute force"}
thresholds = [100, 1000, 10000]
fig, ax = plt.subplots(figsize=(8, 4.6), layout="constrained")
for system, label in labels.items():
    points = {int(r["int_filter_lte"]): float(r["p95_ms"]) for r in rows if r["system"] == system}
    ax.plot(["~1%\n(<=100)", "~10%\n(<=1000)", "100%\n(<=10000)"],
            [points[t] for t in thresholds], marker="o", label=label)
ax.set_yscale("log")
ax.set_ylabel("end-to-end P95 latency (ms, log scale)")
ax.set_title("TopK 1M × 768 cosine: provider-compatible cohort")
ax.legend()
ax.text(0.01, -0.29,
        "Only the ~1% cell met Recall@10 = 1.0 for all adapters. The Qenlo series is a PyTorch prototype,\n"
        "not the shipped Qenlo binary. 10% and 100% cells are retained but invalid for exact claims.",
        transform=ax.transAxes, fontsize=8)
fig.savefig(root / "p95_latency.png", dpi=180, bbox_inches="tight")
