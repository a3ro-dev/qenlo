#!/usr/bin/env python3
"""Reduce retained CPU/GPU measurements into an auditable routing hypothesis."""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from pathlib import Path


DEFAULT_MATRIX = Path("research/artifacts/runpod-small-2026-09-05/report/performance-matrix.csv")
DEFAULT_CROSSOVER = Path("research/data/processed/native_crossover_summary.csv")
DEFAULT_OUTPUT = Path("research/data/processed/alpha5-routing")
DEFAULT_HELDOUT = Path("research/data/processed/alpha5-router-heldout/summary.json")
OLD_MIN_ELIGIBLE_ROWS = 4_096
DEFAULT_WORK_THRESHOLD = 1_000_000


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8-sig") as stream:
        return list(csv.DictReader(stream))


def truth(value: str) -> bool:
    return value.strip().lower() == "true"


def regret(chosen_ns: float, best_ns: float) -> float:
    return chosen_ns / best_ns - 1.0


def route_rows(
    matrix: Path, crossover: Path, work_threshold: int
) -> list[dict[str, object]]:
    raw = [
        row
        for row in read_csv(matrix)
        if row["status"] == "completed" and truth(row["qualified"])
    ]
    grouped: dict[tuple[str, str, str], dict[str, dict[str, str]]] = {}
    for row in raw:
        family = row["engine"].removesuffix("-cpu").removesuffix("-gpu")
        if family not in {"baseline", "current"}:
            continue
        grouped.setdefault(
            (row["configuration"], row["workload"], family), {}
        )[row["engine"].rsplit("-", 1)[1]] = row

    pairs: list[dict[str, object]] = []
    for (configuration, workload, family), engines in sorted(grouped.items()):
        if set(engines) != {"cpu", "gpu"}:
            continue
        cpu, gpu = engines["cpu"], engines["gpu"]
        eligible_rows = round(float(cpu["rows"]) * float(cpu["eligible_fraction"]))
        dimension, batch = int(float(cpu["dimensions"])), int(float(cpu["batch"]))
        cpu_ns, gpu_ns = float(cpu["p95_completed_ns"]), float(gpu["p95_completed_ns"])
        pairs.append(
            make_pair(
                cohort="small-collection-campaign",
                configuration=configuration,
                workload=workload,
                source_family=family,
                eligible_rows=eligible_rows,
                dimension=dimension,
                batch=batch,
                cpu_ns=cpu_ns,
                gpu_ns=gpu_ns,
                work_threshold=work_threshold,
            )
        )

    for row in read_csv(crossover):
        eligible_rows = int(row["eligible"])
        pairs.append(
            make_pair(
                cohort="windows-native-crossover",
                configuration="RTX 4050 / DX12",
                workload=f"eligible-{eligible_rows}",
                source_family="historical",
                eligible_rows=eligible_rows,
                dimension=384,
                batch=1,
                cpu_ns=float(row["cpu_p95_ms"]) * 1_000_000,
                gpu_ns=float(row["gpu_rows_p95_ms"]) * 1_000_000,
                work_threshold=work_threshold,
            )
        )
    return pairs


def make_pair(
    *,
    cohort: str,
    configuration: str,
    workload: str,
    source_family: str,
    eligible_rows: int,
    dimension: int,
    batch: int,
    cpu_ns: float,
    gpu_ns: float,
    work_threshold: int,
) -> dict[str, object]:
    work = eligible_rows * dimension * batch
    best = min(cpu_ns, gpu_ns)
    old_backend = "cpu" if eligible_rows < OLD_MIN_ELIGIBLE_ROWS and batch < 8 else "gpu"
    candidate_backend = "cpu" if work < work_threshold else "gpu"
    return {
        "cohort": cohort,
        "configuration": configuration,
        "workload": workload,
        "source_family": source_family,
        "eligible_rows": eligible_rows,
        "dimension": dimension,
        "batch": batch,
        "work_units": work,
        "cpu_p95_ns": cpu_ns,
        "gpu_p95_ns": gpu_ns,
        "optimal_backend": "cpu" if cpu_ns <= gpu_ns else "gpu",
        "old_backend": old_backend,
        "old_regret": regret(cpu_ns if old_backend == "cpu" else gpu_ns, best),
        "candidate_backend": candidate_backend,
        "candidate_regret": regret(
            cpu_ns if candidate_backend == "cpu" else gpu_ns, best
        ),
    }


def summarize(
    rows: list[dict[str, object]],
    threshold: int,
    heldout: dict[str, object] | None = None,
) -> dict[str, object]:
    old = [float(row["old_regret"]) for row in rows]
    candidate = [float(row["candidate_regret"]) for row in rows]
    release_passed = bool(heldout and heldout.get("release_gate_passed"))
    release_reason = (
        "held-out alpha.5 confirmation passed"
        if release_passed
        else "held-out alpha.5 confirmation rejected the candidate"
        if heldout
        else "held-out alpha.5 confirmation has not been recorded"
    )
    return {
        "schema": "qenlo-routing-analysis-v1",
        "status": (
            "candidate accepted by held-out evidence"
            if release_passed
            else "candidate rejected by held-out evidence"
            if heldout
            else "hypothesis-forming; requires held-out confirmation"
        ),
        "work_definition": "eligible_rows * dimension * batch",
        "candidate_work_threshold": threshold,
        "pair_count": len(rows),
        "old": {
            "wrong_routes": sum(value > 0 for value in old),
            "median_regret": statistics.median(old),
            "max_regret": max(old),
        },
        "candidate": {
            "wrong_routes": sum(value > 0 for value in candidate),
            "median_regret": statistics.median(candidate),
            "max_regret": max(candidate),
        },
        "gate": {
            "median_regret_at_most": 0.10,
            "max_regret_at_most": 0.25,
            "requires_improvement_over_old": True,
            "development_evidence_passed": (
                statistics.median(candidate) <= 0.10
                and max(candidate) <= 0.25
                and statistics.median(candidate) <= statistics.median(old)
                and max(candidate) < max(old)
            ),
            "release_gate_passed": release_passed,
            "release_gate_reason": release_reason,
        },
    }


def write_outputs(output: Path, rows: list[dict[str, object]], summary: dict[str, object]) -> None:
    output.mkdir(parents=True, exist_ok=True)
    with (output / "routing-pairs.csv").open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]), lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    with (output / "summary.json").open("w", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(summary, indent=2) + "\n")
    old, candidate = summary["old"], summary["candidate"]
    release_status = "accepted" if summary['gate']['release_gate_passed'] else "rejected"
    report = f"""# Alpha.5 routing evidence

Status: {summary['status']}.

The analyzed {summary['pair_count']} compatible CPU/GPU pairs compare the alpha.4 fallback with a static `{summary['work_definition']}` threshold of {summary['candidate_work_threshold']:,} work units.

| Policy | Wrong routes | Median regret | Maximum regret |
| --- | ---: | ---: | ---: |
| Alpha.4 | {old['wrong_routes']} | {old['median_regret']:.1%} | {old['max_regret']:.1%} |
| Candidate | {candidate['wrong_routes']} | {candidate['median_regret']:.1%} | {candidate['max_regret']:.1%} |

The candidate passes the development-data gate: **{str(summary['gate']['development_evidence_passed']).lower()}**. The held-out release decision is **{release_status}**: {summary['gate']['release_gate_reason']}. See `../alpha5-router-heldout/report.md`. The candidate is not part of alpha.5 runtime behavior.
"""
    with (output / "report.md").open("w", encoding="utf-8", newline="\n") as stream:
        stream.write(report)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", type=Path, default=DEFAULT_MATRIX)
    parser.add_argument("--crossover", type=Path, default=DEFAULT_CROSSOVER)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--heldout", type=Path, default=DEFAULT_HELDOUT)
    parser.add_argument("--work-threshold", type=int, default=DEFAULT_WORK_THRESHOLD)
    args = parser.parse_args()
    if args.work_threshold <= 0:
        parser.error("--work-threshold must be positive")
    rows = route_rows(args.matrix, args.crossover, args.work_threshold)
    if not rows:
        raise SystemExit("no compatible CPU/GPU pairs found")
    heldout = json.loads(args.heldout.read_text(encoding="utf-8")) if args.heldout.exists() else None
    summary = summarize(rows, args.work_threshold, heldout)
    write_outputs(args.output, rows, summary)
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
