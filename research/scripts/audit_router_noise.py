#!/usr/bin/env python3
"""Measurement-reliability audit of the alpha.5 held-out router suite.

Reads the retained archive read-only. The automatic engine executes the same
code path as one forced engine, so automatic-vs-same-backend-forced is an
A/A comparison that estimates the suite's noise floor. Writes
research/data/processed/router-noise-audit/.
"""

from __future__ import annotations

import csv
import io
import json
import tarfile
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
ARCHIVE = ROOT / "research/data/raw/alpha5-router-heldout-rtx4050.tar.gz"
OUT = ROOT / "research/data/processed/router-noise-audit"
RNG = np.random.default_rng(20260923)
BOOT = 4000


def nearest_rank(values: np.ndarray, q: float) -> float:
    ordered = np.sort(values)
    return float(ordered[max(0, int(np.ceil(q * len(ordered))) - 1)])


def load() -> dict:
    cells: dict = {}
    with tarfile.open(ARCHIVE) as tar:
        for member in tar.getmembers():
            parts = Path(member.name).parts
            if len(parts) != 4 or parts[3] != "samples.csv":
                continue
            _, cell, engine, _ = parts
            rows = list(csv.DictReader(io.TextIOWrapper(tar.extractfile(member), "utf-8")))
            runs = {}
            for row in rows:
                runs.setdefault(int(row["run"]), []).append(float(row["batch_latency_ns"]))
            backends = sorted({row["actual_backend"] for row in rows})
            cells.setdefault(cell, {})[engine] = {
                "runs": [np.array(runs[r]) for r in sorted(runs)],
                "backend": "/".join(backends).lower(),
            }
    return cells


def stat(runs: list[np.ndarray], q: float) -> float:
    # Harness convention: lower-middle median of per-run nearest-rank percentiles.
    per_run = sorted(nearest_rank(r, q) for r in runs)
    return per_run[(len(per_run) - 1) // 2]


def boot(runs: list[np.ndarray], q: float) -> np.ndarray:
    # Hierarchical bootstrap: resample runs, then batches within each run.
    out = np.empty(BOOT)
    n = len(runs)
    for i in range(BOOT):
        picked = [runs[j] for j in RNG.integers(0, n, n)]
        out[i] = stat([r[RNG.integers(0, len(r), len(r))] for r in picked], q)
    return out


def main() -> None:
    cells = load()
    OUT.mkdir(parents=True, exist_ok=True)
    rows = []
    for name in sorted(cells):
        c = cells[name]
        cpu, gpu, auto = c["cpu"], c["gpu-rows"], c["automatic"]
        same = cpu if auto["backend"] == "cpu" else gpu
        row = {"cell": name, "batches_per_run": len(cpu["runs"][0]), "automatic_backend": auto["backend"]}
        for q, label in ((0.5, "p50"), (0.95, "p95")):
            bc, bg = boot(cpu["runs"], q), boot(gpu["runs"], q)
            ba, bs = boot(auto["runs"], q), boot(same["runs"], q)
            ratio = bc / bg
            aa = ba / bs
            row |= {
                f"cpu_{label}_us": stat(cpu["runs"], q) / 1e3,
                f"gpu_{label}_us": stat(gpu["runs"], q) / 1e3,
                f"cpu_over_gpu_{label}": stat(cpu["runs"], q) / stat(gpu["runs"], q),
                f"cpu_over_gpu_{label}_ci_lo": float(np.quantile(ratio, 0.025)),
                f"cpu_over_gpu_{label}_ci_hi": float(np.quantile(ratio, 0.975)),
                f"p_gpu_faster_{label}": float(np.mean(ratio > 1)),
                f"aa_auto_over_same_{label}": stat(auto["runs"], q) / stat(same["runs"], q),
                f"aa_{label}_ci_lo": float(np.quantile(aa, 0.025)),
                f"aa_{label}_ci_hi": float(np.quantile(aa, 0.975)),
            }
        run_p95 = [nearest_rank(r, 0.95) for r in cpu["runs"] + gpu["runs"]]
        row["max_run_spread_p95"] = max(
            max(nearest_rank(r, 0.95) for r in e["runs"]) / min(nearest_rank(r, 0.95) for r in e["runs"])
            for e in (cpu, gpu, auto)
        )
        row["winner_p95"] = "gpu" if row["cpu_over_gpu_p95"] > 1 else "cpu"
        row["winner_p50"] = "gpu" if row["cpu_over_gpu_p50"] > 1 else "cpu"
        lo, hi = row["cpu_over_gpu_p95_ci_lo"], row["cpu_over_gpu_p95_ci_hi"]
        row["p95_winner_decisive"] = bool(lo > 1 or hi < 1)
        del run_p95
        rows.append(row)

    with open(OUT / "cells.csv", "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0]))
        w.writeheader()
        w.writerows(rows)

    aa95 = np.array([r["aa_auto_over_same_p95"] for r in rows])
    aa50 = np.array([r["aa_auto_over_same_p50"] for r in rows])
    summary = {
        "cells": len(rows),
        "aa_p95_log_ratio_sd": float(np.std(np.log(aa95), ddof=1)),
        "aa_p50_log_ratio_sd": float(np.std(np.log(aa50), ddof=1)),
        "aa_p95_range": [float(aa95.min()), float(aa95.max())],
        "aa_p50_range": [float(aa50.min()), float(aa50.max())],
        "aa_p95_outside_25pct": int(np.sum((aa95 > 1.25) | (aa95 < 0.8))),
        "decisive_p95_winners": int(sum(r["p95_winner_decisive"] for r in rows)),
        "p50_p95_winner_disagreements": [r["cell"] for r in rows if r["winner_p50"] != r["winner_p95"]],
    }
    (OUT / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2))
    cols = ["cell", "batches_per_run", "automatic_backend", "cpu_over_gpu_p95", "cpu_over_gpu_p95_ci_lo",
            "cpu_over_gpu_p95_ci_hi", "cpu_over_gpu_p50", "aa_auto_over_same_p95", "aa_auto_over_same_p50",
            "max_run_spread_p95"]
    print("\t".join(cols))
    for r in rows:
        print("\t".join(f"{r[c]:.3f}" if isinstance(r[c], float) else str(r[c]) for c in cols))


if __name__ == "__main__":
    main()
