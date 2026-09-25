"""Interleaved local A/B for the timestamp-range materialization change (2026-09-24).

A = baseline binary built from a clean HEAD worktree, B = working-tree binary.
Same host, dataset, seeds, and arguments; order ABAB per cell. Local laptop evidence
only: not comparable to the RTX 4090 E0/E2 host and not a campaign result.

    python research/scripts/run_local_filter_ab.py BASELINE_EXE CANDIDATE_EXE OUT_DIR
"""
import csv
import subprocess
import sys
from pathlib import Path

baseline, candidate, out = sys.argv[1], sys.argv[2], Path(sys.argv[3])
CELLS = [("gpu-rows", 30000), ("gpu-rows", 10000), ("gpu-rows", 1000), ("cpu", 30000)]
rows = []
for backend, eligible in CELLS:
    for rep, (label, exe) in enumerate([("A", baseline), ("B", candidate)] * 2):
        dest = out / f"{backend}-e{eligible}-{rep}{label}"
        subprocess.run(
            [exe, "run", "--dataset", "data/ag-news/ag-news-100k-384.qnb", "--output", str(dest),
             "--dimensions", "384", "--backend", backend, "--distribution", "independent",
             "--eligible-count", str(eligible), "--batch", "1", "--k", "10", "--warmups", "200",
             "--repetitions", "3", "--recall-target", "0.99", "--order-seed", "9003",
             "--diagnostics", "detailed", "--vector-budget-mib", "1024", "--gpu-budget-mib", "2048"],
            check=True,
        )
        runs = list(csv.DictReader(open(dest / "runs.csv")))
        samples = list(csv.DictReader(open(dest / "samples.csv")))
        mid = lambda xs: sorted(xs)[(len(xs) - 1) // 2]
        mat = [int(s["row_materialization_ns"]) for s in samples if s["row_materialization_ns"]]
        rows.append({
            "backend": backend, "eligible": eligible, "order": rep, "binary": label,
            "p50_ms": mid([int(r["p50_batch_ns"]) for r in runs]) / 1e6,
            "p95_ms": mid([int(r["p95_batch_ns"]) for r in runs]) / 1e6,
            "median_row_materialization_ms": (sorted(mat)[len(mat) // 2] / 1e6) if mat else "",
            "recall_at_10": runs[0]["recall_at_10"],
        })
        print(rows[-1], flush=True)
with open(out / "ab_summary.csv", "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0]))
    w.writeheader()
    w.writerows(rows)
