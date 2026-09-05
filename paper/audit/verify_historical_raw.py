#!/usr/bin/env python3
"""Verify historical reduced values against retained raw run/sample artifacts.

This is an audit helper, not a benchmark. It reads only retained files and
prints JSON. Native Rust rows use the recorded lower-middle run percentile;
the synthetic A6000 rows use NumPy's percentile implementation over retained
per-call samples. No output files are modified.
"""
from __future__ import annotations

import csv
import json
import tarfile
from pathlib import Path
from statistics import median

try:
    import numpy as np
except ImportError as exc:  # pragma: no cover
    raise SystemExit("numpy is required for the A6000 raw-sample check") from exc


ROOT = Path(__file__).resolve().parents[2]
PROCESSED = ROOT / "research/data/processed"


def rows(path: Path):
    with path.open(encoding="utf-8", newline="") as fh:
        return list(csv.DictReader(fh))


def lower_middle(values):
    ordered = sorted(values)
    return ordered[(len(ordered) - 1) // 2]


def check_native(processed_name: str, raw_dirs: dict[str, Path], key: str):
    expected = {int(r["eligible"]): float(r[key]) for r in rows(PROCESSED / processed_name)}
    observed = {}
    for eligible, directory in raw_dirs.items():
        p95 = [int(r["p95_batch_ns"]) / 1e6 for r in rows(directory / "runs.csv")]
        observed[int(eligible)] = lower_middle(p95)
    return expected, observed


def check_tar_eligibility():
    expected = {
        (int(r["eligible"]), r["mode"]): float(r["median_run_p95_ms"])
        for r in rows(PROCESSED / "eligibility_ablation_summary.csv")
    }
    archive = ROOT / "research/artifacts/qenlo-phase1-eligibility-2026-09-04.tar.gz"
    observed = {}
    with tarfile.open(archive, "r:gz") as tf:
        for member in tf.getmembers():
            if not member.name.endswith("/runs.csv") or not member.name.startswith("artifacts/"):
                continue
            stem = Path(member.name).parent.name  # e1000-cached
            if not stem.startswith("e"):
                continue
            eligible, mode = stem[1:].split("-", 1)
            with tf.extractfile(member) as fh:
                data = list(csv.DictReader(line.decode() for line in fh))
            observed[(int(eligible), mode)] = lower_middle(
                [int(r["p95_batch_ns"]) / 1e6 for r in data]
            )
    return expected, observed


def check_a6000_samples():
    expected = {
        (r["system"], int(r["eligible"])): r
        for r in rows(PROCESSED / "a6000_exact_summary.csv")
    }
    grouped: dict[tuple[str, int], list[float]] = {}
    path = ROOT / "research/data/raw/2026-09-02-a6000-cuda/heldout/raw_samples.csv"
    for r in rows(path):
        grouped.setdefault((r["system"], int(r["eligible"])), []).append(
            int(r["latency_ns_e2e"]) / 1e6
        )
    observed = {}
    for key, values in grouped.items():
        observed[key] = {
            "samples": len(values),
            "p50_ms": float(np.percentile(values, 50)),
            "p95_ms": float(np.percentile(values, 95)),
            "p99_ms": float(np.percentile(values, 99)),
        }
    return expected, observed


def compare(expected, observed, tolerance=1e-9):
    if set(expected) != set(observed):
        return False, {"missing": sorted(set(expected) - set(observed), key=str), "extra": sorted(set(observed) - set(expected), key=str)}
    errors = {}
    for key in expected:
        left, right = expected[key], observed[key]
        if isinstance(left, dict):
            for field in ("samples", "p50_ms", "p95_ms", "p99_ms"):
                a = int(left[field]) if field == "samples" else float(left[field])
                b = int(right[field]) if field == "samples" else float(right[field])
                if abs(a - b) > tolerance:
                    errors[f"{key}:{field}"] = [a, b]
        elif abs(float(left) - float(right)) > tolerance:
            errors[str(key)] = [left, right]
    return not errors, errors


def main():
    native_dirs = {
        int(p.name.split("e")[-1]): p
        for p in (ROOT / "research/data/raw/2026-09-02-native-crossover").iterdir()
        if p.is_dir() and p.name.startswith("cpu-e")
    }
    # Endpoint rows are checked separately because they use historical source
    # directories and two recorded executable revisions.
    endpoint_dirs = {
        1000: ROOT / "benchmarks/2026-08-28/real/cpu-384-onepct",
        100000: ROOT / "benchmarks/2026-08-28/real/cpu-384-all-v4",
    }
    crossover_expected, crossover_observed = check_native("native_crossover_summary.csv", native_dirs, "cpu_p95_ms")
    endpoint_expected = {int(r["eligible"]): float(r["p95_ms"]) for r in rows(PROCESSED / "windows_summary.csv") if r["system"].startswith("CPU")}
    endpoint_observed = {e: lower_middle([int(r["p95_batch_ns"]) / 1e6 for r in rows(p / "runs.csv")]) for e, p in endpoint_dirs.items()}
    ablation_expected, ablation_observed = check_tar_eligibility()
    a6_expected, a6_observed = check_a6000_samples()
    checks = {}
    for name, expected, observed in (("crossover_cpu", crossover_expected, crossover_observed), ("windows_cpu_endpoints", endpoint_expected, endpoint_observed), ("eligibility_archive", ablation_expected, ablation_observed), ("a6000_samples", a6_expected, a6_observed)):
        ok, errors = compare(expected, observed)
        checks[name] = {"ok": ok, "errors": errors}
    result = {"schema": "qenlo-historical-raw-verification-v1", "checks": checks, "all_ok": all(v["ok"] for v in checks.values())}
    print(json.dumps(result, indent=2, sort_keys=True))
    raise SystemExit(0 if result["all_ok"] else 1)


if __name__ == "__main__":
    main()
