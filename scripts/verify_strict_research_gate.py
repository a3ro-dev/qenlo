#!/usr/bin/env python3
"""Independently verify retained valid raw rows for the strict A6000 gate."""
from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path


def read_rows(path: Path):
    with path.open(newline="") as handle:
        return list(csv.DictReader(handle))


def verify(name: str, rows: list[dict]):
    assert len(rows) == 5_000, f"{name}: expected 5,000 rows, got {len(rows)}"
    assert {int(row["rep"]) for row in rows} == set(range(5)), f"{name}: repetitions are incomplete"
    assert {int(row["query_id"]) for row in rows} == set(range(1_000)), f"{name}: query set differs"
    for row in rows:
        returned = json.loads(row["returned_ids"])
        oracle = json.loads(row["oracle_ids"])
        assert len(returned) == len(oracle) == 10, f"{name}: malformed top-k row"
        assert set(returned) == set(oracle), f"{name}: recall miss at query {row['query_id']}"
        assert float(row["recall_at_10"]) == 1.0, f"{name}: recorded recall miss"
    values = sorted(int(row["latency_ns_e2e"]) / 1e6 for row in rows)
    def pct(p):
        index = (len(values) - 1) * p
        lo, hi = int(index), min(int(index) + 1, len(values) - 1)
        return values[lo] + (values[hi] - values[lo]) * (index - lo)
    return {"system": name, "samples": len(values), "p50_ms": pct(.50), "p95_ms": pct(.95), "p99_ms": pct(.99)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path("benchmark-results/2026-09-02-runpod-a6000-strict-research-gate"))
    args = parser.parse_args()
    root = args.root
    q = verify("qenlo_cuda_predicate_prototype", read_rows(root / "strict-qenlo-isolated" / "raw_samples.csv"))
    initial = read_rows(root / "strict-gate" / "raw_samples.csv")
    f = verify("faiss_gpu_flat_ip", [r for r in initial if r["system"] == "faiss_gpu_flat_ip"])
    c = verify("numpy_cpu_exact_fp32", [r for r in initial if r["system"] == "numpy_cpu_exact_fp32"])
    print(json.dumps({"verified": [q, f, c], "qenlo_vs_faiss_p95_ratio": q["p95_ms"] / f["p95_ms"]}, indent=2))


if __name__ == "__main__":
    main()
