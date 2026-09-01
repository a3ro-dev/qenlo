"""Evaluate Qenlo's immutable 1M x 768 investment gate from retained run artifacts.

The fastest recall-qualified CPU baseline wins baseline selection. The candidate
must be required exact GPU predicate execution with no fallback in any sample.
This script only evaluates completed evidence; it never launches benchmarks.
"""

import argparse
import csv
import json
import sys
from pathlib import Path

from compare_runs import compare, read_record, read_run


GATE = {
    "dimensions": 768,
    "rows": 1_000_000,
    "eligible_count": 10_000,
    "batch": 1,
    "k": 10,
    "warmup_queries": 200,
    "repetitions": 5,
    "recall_target": 0.99,
    "metadata": "synthetic-independent",
    "filter_user_id": None,
    "filter_mode": "shared",
    "corpus_range": [0, 1_000_000],
    "tuning_range": [1_000_000, 1_001_000],
    "evaluation_range": [1_001_000, 1_006_000],
    "seed": 42,
}


def validate_shape(run, name):
    mismatches = {
        key: {"expected": expected, "actual": run["workload"].get(key)}
        for key, expected in GATE.items()
        if run["workload"].get(key) != expected
    }
    if mismatches:
        raise ValueError(f"{name} is not the preregistered gate: {mismatches}")
    if not run["recall_gate"]["passed"]:
        raise ValueError(f"{name} failed the Recall@10 gate")


def validate_runtime(directory, expected_backend, require_gpu=False):
    configuration = read_record(directory / "configuration.txt")
    required = {
        "source_kind": "imported-raw-f32-le",
        "diagnostics": "basic",
        "git_worktree_dirty": "false",
    }
    for key, expected in required.items():
        if configuration.get(key) != expected:
            raise ValueError(f"{directory}: {key} must be {expected!r}")
    if require_gpu:
        for key in ("gpu_adapter", "gpu_api", "gpu_device_type"):
            if not configuration.get(key):
                raise ValueError(f"{directory}: missing {key}")
    with (directory / "samples.csv").open(newline="", encoding="utf-8") as stream:
        samples = list(csv.DictReader(stream))
    expected_samples = GATE["repetitions"] * (
        GATE["evaluation_range"][1] - GATE["evaluation_range"][0]
    )
    if len(samples) != expected_samples:
        raise ValueError(f"{directory}: expected {expected_samples} samples, got {len(samples)}")
    for row in samples:
        if row["actual_backend"] != expected_backend:
            raise ValueError(f"{directory}: unexpected actual backend {row['actual_backend']}")
        if row["fallback"].lower() != "false":
            raise ValueError(f"{directory}: fallback invalidates gate")
        if int(row["eligible_count"]) != GATE["eligible_count"]:
            raise ValueError(f"{directory}: sample eligibility changed")
        if require_gpu and any(row[field] == "" for field in (
            "upload_bytes", "readback_bytes", "max_qenlo_allocation_bytes"
        )):
            raise ValueError(f"{directory}: missing required GPU telemetry")
    return configuration


def evaluate(cpu_directories, gpu_directory):
    baselines = []
    for directory in cpu_directories:
        run = read_run(directory)
        validate_shape(run, str(directory))
        expected = "Cpu" if run["backend_requested"] == "cpu" else "Usearch"
        if run["backend_requested"] not in ("cpu", "usearch"):
            raise ValueError(f"unsupported CPU baseline {run['backend_requested']}")
        validate_runtime(directory, expected)
        baselines.append(run)
    qualifying = [run for run in baselines if run["recall_gate"]["passed"]]
    if not qualifying:
        raise ValueError("no CPU baseline qualified")
    baseline = min(qualifying, key=lambda run: run["median_run_p95_batch_ns"])

    gpu = read_run(gpu_directory)
    validate_shape(gpu, str(gpu_directory))
    if gpu["backend_requested"] != "gpu-predicate":
        raise ValueError("candidate must request exact gpu-predicate")
    gpu_configuration = validate_runtime(gpu_directory, "Wgpu", require_gpu=True)
    comparison = compare(Path(baseline["directory"]), gpu_directory, seed=20260902)
    ratio = comparison["baseline_over_candidate_median_p95_ratio"]
    passed = bool(comparison["latency_comparison_valid"] and ratio is not None and ratio >= 2.0)
    return {
        "format": "qenlo-research-gate-v1",
        "preregistered_gate": GATE,
        "status": "passed" if passed else "failed",
        "claim_allowed": passed,
        "claim": "Qenlo exact GPU predicate search achieved at least 2x lower median-of-run P95 than the fastest qualifying Qenlo CPU path on the preregistered cell" if passed else None,
        "selected_cpu_baseline": baseline,
        "all_cpu_baselines": baselines,
        "gpu_candidate": gpu,
        "gpu_hardware": {key: gpu_configuration[key] for key in ("gpu_adapter", "gpu_api", "gpu_device_type")},
        "comparison": comparison,
        "decision_rule": "pass iff both recall gates pass, all 25,000 candidate samples execute on Wgpu without fallback, required telemetry is present, and baseline/candidate median-run P95 ratio is >= 2.0",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cpu", type=Path, action="append", required=True)
    parser.add_argument("--gpu", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = evaluate(args.cpu, args.gpu)
    with args.output.open("x", encoding="utf-8") as stream:
        json.dump(result, stream, indent=2, allow_nan=False)
        stream.write("\n")
    print(f"research gate {result['status']}; saved {args.output}")
    return 0 if result["claim_allowed"] else 2


if __name__ == "__main__":
    sys.exit(main())
