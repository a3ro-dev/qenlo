#!/usr/bin/env python3
"""Validate and summarize the held-out alpha.5 router gate."""

from __future__ import annotations

import argparse
import csv
import json
import statistics
import tarfile
import tempfile
from pathlib import Path


def properties(path: Path) -> dict[str, str]:
    return dict(
        line.split("=", 1)
        for line in path.read_text(encoding="utf-8").splitlines()
        if "=" in line
    )


def rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8-sig") as stream:
        return list(csv.DictReader(stream))


def lower_median(values: list[int]) -> int:
    ordered = sorted(values)
    return ordered[(len(ordered) - 1) // 2]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gate", type=Path, default=Path("research/data/raw/alpha5-router-heldout-rtx4050.tar.gz"))
    parser.add_argument("--output", type=Path, default=Path("research/data/processed/alpha5-router-heldout"))
    args = parser.parse_args()
    temporary = None
    gate = args.gate
    if gate.is_file():
        temporary = tempfile.TemporaryDirectory()
        with tarfile.open(gate, "r:gz") as archive:
            archive.extractall(temporary.name, filter="data")
        manifests = list(Path(temporary.name).rglob("manifest.json"))
        if len(manifests) != 1:
            raise ValueError(f"expected one manifest in {gate}, found {len(manifests)}")
        gate = manifests[0].parent
    manifest = json.loads((gate / "manifest.json").read_text(encoding="utf-8"))
    results = []
    for workload in manifest["workloads"]:
        name = workload["name"]
        summaries = {engine: properties(gate / name / engine / "summary.txt") for engine in ("cpu", "gpu-rows", "automatic")}
        run_data = {engine: rows(gate / name / engine / "runs.csv") for engine in summaries}
        sample_data = rows(gate / name / "automatic" / "samples.csv")
        for engine, summary in summaries.items():
            if summary.get("status") != "completed" or summary.get("recall_target_passed") != "true":
                raise ValueError(f"{name}/{engine} failed completion or recall")
            if len(run_data[engine]) != 5:
                raise ValueError(f"{name}/{engine} does not contain five runs")
        cpu_ns = lower_median([int(row["p95_batch_ns"]) for row in run_data["cpu"]])
        gpu_ns = lower_median([int(row["p95_batch_ns"]) for row in run_data["gpu-rows"]])
        auto_ns = lower_median([int(row["p95_batch_ns"]) for row in run_data["automatic"]])
        backends = sorted({row["actual_backend"] for row in sample_data})
        if len(backends) != 1:
            raise ValueError(f"{name} automatic route was not stable: {backends}")
        actual = "cpu" if backends == ["Cpu"] else "gpu"
        optimal = "cpu" if cpu_ns <= gpu_ns else "gpu"
        best_ns = min(cpu_ns, gpu_ns)
        results.append(workload | {
            "cpu_p95_ns": cpu_ns, "gpu_p95_ns": gpu_ns, "automatic_p95_ns": auto_ns,
            "optimal_backend": optimal, "automatic_backend": actual,
            "routing_regret": (cpu_ns if actual == "cpu" else gpu_ns) / best_ns - 1,
            "automatic_over_best": auto_ns / best_ns - 1,
            "recall_passed": True,
        })
    regrets = [row["routing_regret"] for row in results]
    wrong = sum(row["optimal_backend"] != row["automatic_backend"] for row in results)
    summary = {
        "schema": "qenlo-alpha5-router-heldout-result-v1",
        "workloads": len(results), "wrong_routes": wrong,
        "median_routing_regret": statistics.median(regrets),
        "max_routing_regret": max(regrets),
        "recall_and_filter_correct": all(row["recall_passed"] for row in results),
    }
    summary["release_gate_passed"] = (
        summary["recall_and_filter_correct"]
        and summary["median_routing_regret"] <= 0.10
        and summary["max_routing_regret"] <= 0.25
    )
    args.output.mkdir(parents=True, exist_ok=True)
    with (args.output / "results.csv").open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(results[0]), lineterminator="\n")
        writer.writeheader(); writer.writerows(results)
    with (args.output / "summary.json").open("w", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(summary, indent=2) + "\n")
    verdict = "accepted" if summary["release_gate_passed"] else "rejected"
    report = f"""# Alpha.5 held-out routing gate

Status: **{verdict}**.

The preregistered RTX 4050 / DX12 suite measured {summary['workloads']} workloads after the 1,000,000-work-unit rule was selected from older evidence. Recall and filter correctness passed: **{str(summary['recall_and_filter_correct']).lower()}**.

| Result | Value | Release limit |
| --- | ---: | ---: |
| Wrong routes | {summary['wrong_routes']} / {summary['workloads']} | descriptive |
| Median routing regret | {summary['median_routing_regret']:.1%} | <= 10% |
| Maximum routing regret | {summary['max_routing_regret']:.1%} | <= 25% |

The candidate is not shipped. The held-out data shows that a single `eligible_rows * dimension * batch` threshold does not generalize across query shapes and `k`; Qenlo retains the alpha.4 static fallback and its per-device tuning-profile override. Raw summaries, samples, run order, diagnostics, and the deterministic dataset recipe are retained in `research/data/raw/alpha5-router-heldout-rtx4050.tar.gz`.
"""
    with (args.output / "report.md").open("w", encoding="utf-8", newline="\n") as stream:
        stream.write(report)
    if temporary is not None:
        temporary.cleanup()
    print(json.dumps(summary, indent=2))
    return 0 if summary["release_gate_passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
