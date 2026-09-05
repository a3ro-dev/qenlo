#!/usr/bin/env python3
"""Reduce retained small-collection campaign artifacts into auditable matrices."""
from __future__ import annotations

import argparse
import csv
import json
import math
import statistics
import tarfile
from pathlib import Path


COMMON_CELLS = (
    ("c1-r1000-d128-b1-f1", 1000, 128, 1, 1.0),
    ("c2-r1000-d384-b16-f001", 1000, 384, 16, 0.01),
    ("c3-r10000-d128-b16-f1", 10000, 128, 16, 1.0),
    ("c4-r10000-d384-b1-f001", 10000, 384, 1, 0.01),
    ("c5-r100000-d128-b1-f001", 100000, 128, 1, 0.01),
    ("c6-r100000-d384-b16-f1", 100000, 384, 16, 1.0),
)


def properties(path: Path) -> dict[str, str]:
    return dict(
        line.split("=", 1)
        for line in path.read_text(encoding="utf-8").splitlines()
        if "=" in line
    )


def lower_median(values: list[float]) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    return ordered[(len(ordered) - 1) // 2]


def numeric(row: dict[str, str], *names: str) -> float | None:
    for name in names:
        value = row.get(name)
        if value not in (None, "", "not-applicable"):
            return float(value)
    return None


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as stream:
        return list(csv.DictReader(stream, delimiter="\t" if path.suffix == ".tsv" else ","))


def peak_rss(time_path: Path) -> int | None:
    if not time_path.exists():
        return None
    for line in time_path.read_text(encoding="utf-8").splitlines():
        if "Maximum resident set size (kbytes)" in line:
            return int(line.rsplit(":", 1)[1].strip()) * 1024
    return None


def safe_extract(archive: Path, destination: Path) -> None:
    with tarfile.open(archive, "r:gz") as tar:
        root = destination.resolve()
        for member in tar.getmembers():
            target = (destination / member.name).resolve()
            if not target.is_relative_to(root) or member.issym() or member.islnk():
                raise ValueError(f"unsafe archive member: {member.name}")
        tar.extractall(destination, filter="data")


def result_template(record: dict, cell: str, engine: str) -> dict:
    configuration = record["configuration"]
    return {
        "configuration": configuration["name"],
        "gpu": configuration["gpuId"],
        "data_center": configuration["dataCenter"],
        "cloud": configuration["cloud"],
        "campaign_mode": record.get("mode"),
        "workload": cell,
        "engine": engine,
        "source_bundle_sha256": None,
        "git_revision": None,
        "rows": None,
        "dimensions": None,
        "batch": None,
        "k": None,
        "eligible_fraction": None,
        "timing_scope": None,
        "filter_scope": None,
        "p50_completed_ns": None,
        "p95_completed_ns": None,
        "p99_completed_ns": None,
        "p95_min_ns": None,
        "p95_max_ns": None,
        "throughput_qps": None,
        "recall_at_k": None,
        "collection_build_ns": None,
        "binary_build_ns": None,
        "peak_process_rss_bytes": None,
        "qenlo_allocation_bytes": None,
        "upload_bytes_per_call": None,
        "readback_bytes_per_call": None,
        "device_resident_ns": None,
        "device_scoring_ns": None,
        "device_selection_ns": None,
        "upload_enqueue_ns": None,
        "readback_completion_ns": None,
        "backend_execution_ns": None,
        "eligibility_materialization_ns": None,
        "eligibility_transfer_bytes": None,
        "mutation_p50_ns": None,
        "mutation_p95_ns": None,
        "actual_backend": None,
        "rebuilt_fraction": None,
        "deletion_correct": None,
        "sample_count": 0,
        "qualified": False,
        "status": "unknown",
        "failure_detail": None,
    }


def native_row(record: dict, cell: str, engine: str, path: Path, time_path: Path) -> dict:
    result = result_template(record, cell, engine)
    config = properties(path / "configuration.txt")
    summary = properties(path / "summary.txt")
    runs = read_csv(path / "runs.csv")
    samples = read_csv(path / "samples.csv")
    p50s = [numeric(row, "p50_batch_ns", "p50_completed_ns") for row in runs]
    p95s = [numeric(row, "p95_batch_ns", "p95_completed_ns") for row in runs]
    p99s = [numeric(row, "p99_batch_ns", "p99_completed_ns") for row in runs]
    qps = [numeric(row, "qps") for row in runs]
    recalls = [numeric(row, "recall_at_k", "recall_at_10") for row in runs]
    allocations = [numeric(row, "max_qenlo_allocation_bytes") for row in samples]
    uploads = [numeric(row, "upload_bytes") for row in samples]
    readbacks = [numeric(row, "readback_bytes") for row in samples]
    phase_names = (
        "device_scoring_ns", "device_selection_ns", "upload_enqueue_ns",
        "readback_completion_ns", "backend_execution_ns", "row_materialization_ns",
        "eligibility_transfer_bytes",
    )
    phases = {
        name: lower_median([value for row in samples if (value := numeric(row, name)) is not None])
        for name in phase_names
    }
    result.update(
        source_bundle_sha256=config.get("source_bundle_sha256")
        or record.get("sourceDigests", {}).get("Baseline" if engine.startswith("baseline") else "Current"),
        git_revision=config.get("git_revision"),
        rows=int(config["rows"]),
        dimensions=int(config["dimensions"]),
        batch=int(config["batch"]),
        k=int(config.get("k", 10)),
        eligible_fraction=float(config["eligible_fraction_actual"]),
        timing_scope=config["query_latency"],
        filter_scope=config.get("fraction_scope"),
        p50_completed_ns=lower_median([value for value in p50s if value is not None]),
        p95_completed_ns=lower_median([value for value in p95s if value is not None]),
        p99_completed_ns=lower_median([value for value in p99s if value is not None]),
        p95_min_ns=min(value for value in p95s if value is not None),
        p95_max_ns=max(value for value in p95s if value is not None),
        throughput_qps=lower_median([value for value in qps if value is not None]),
        recall_at_k=statistics.fmean(value for value in recalls if value is not None),
        collection_build_ns=int(summary["build_ns"]),
        peak_process_rss_bytes=peak_rss(time_path),
        qenlo_allocation_bytes=max((value for value in allocations if value is not None), default=None),
        upload_bytes_per_call=lower_median([value for value in uploads if value is not None]),
        readback_bytes_per_call=lower_median([value for value in readbacks if value is not None]),
        device_scoring_ns=phases["device_scoring_ns"],
        device_selection_ns=phases["device_selection_ns"],
        upload_enqueue_ns=phases["upload_enqueue_ns"],
        readback_completion_ns=phases["readback_completion_ns"],
        backend_execution_ns=phases["backend_execution_ns"],
        eligibility_materialization_ns=phases["row_materialization_ns"],
        eligibility_transfer_bytes=phases["eligibility_transfer_bytes"],
        actual_backend=next((row.get("actual_backend") for row in samples if row.get("actual_backend")), None),
        sample_count=len(samples),
        qualified=summary.get("recall_target_passed") == "true",
        status="completed",
    )
    return result


def replay_row(record: dict, cell: str, engine: str, path: Path) -> dict:
    result = result_template(record, cell, engine)
    summary = json.loads((path / "summary.json").read_text(encoding="utf-8"))
    runs = read_csv(path / "runs.csv")
    if engine.startswith("chroma"):
        config = json.loads((path / "configuration.json").read_text(encoding="utf-8"))
        p50s = [numeric(row, "p50_batch_ns") for row in runs]
        p95s = [numeric(row, "p95_batch_ns") for row in runs]
        p99s = [numeric(row, "p99_batch_ns") for row in runs]
        recalls = [numeric(row, "recall_at_10") for row in runs]
        result.update(
            source_bundle_sha256=record.get("sourceDigests", {}).get("Current"),
            git_revision=config.get("git_revision"),
            rows=int(config["rows"]), dimensions=int(config["dimensions"]),
            batch=int(config["batch"]), k=10,
            eligible_fraction=float(config["eligible_fraction_actual"]),
            timing_scope=config["query_latency"], filter_scope=config.get("fraction_scope"),
            collection_build_ns=summary["build_ns"],
            qualified=bool(summary["recall_target_passed"]),
        )
    else:
        p50s = [numeric(row, "p50_completed_ns") for row in runs]
        p95s = [numeric(row, "p95_completed_ns") for row in runs]
        p99s = [numeric(row, "p99_completed_ns") for row in runs]
        recalls = [numeric(row, "recall_at_k") for row in runs]
        device = read_csv(path / "samples.csv")
        device_times = [numeric(row, "device_resident_ns") for row in device]
        result.update(
            source_bundle_sha256=record.get("sourceDigests", {}).get("Current"),
            rows=summary["rows"], dimensions=summary["dimension"], batch=summary["batch"],
            k=summary["k"], eligible_fraction=summary["eligible_rows"] / summary["rows"],
            timing_scope=summary["timing_scope"], filter_scope=summary["filter_scope"],
            collection_build_ns=summary["build_and_filter_prepare_ns"],
            peak_process_rss_bytes=(summary["peak_process_rss_kib"] * 1024
                                    if summary["peak_process_rss_kib"] is not None else None),
            qenlo_allocation_bytes=summary["allocation_bytes"],
            device_resident_ns=lower_median([value for value in device_times if value is not None]),
            qualified=bool(summary["qualified"]),
        )
    qps = [numeric(row, "qps") for row in runs]
    result.update(
        p50_completed_ns=lower_median([value for value in p50s if value is not None]),
        p95_completed_ns=lower_median([value for value in p95s if value is not None]),
        p99_completed_ns=lower_median([value for value in p99s if value is not None]),
        p95_min_ns=min(value for value in p95s if value is not None),
        p95_max_ns=max(value for value in p95s if value is not None),
        throughput_qps=lower_median([value for value in qps if value is not None]),
        recall_at_k=statistics.fmean(value for value in recalls if value is not None),
        sample_count=len(runs), status="completed",
    )
    return result


def lifecycle_rows(record: dict, artifact_root: Path) -> list[dict]:
    path = artifact_root / "lifecycle-current-gpu"
    if not (path / "summary.txt").exists() or not (path / "lifecycle.csv").exists():
        return []
    summary = properties(path / "summary.txt")
    samples = read_csv(path / "lifecycle.csv")
    rows: list[dict] = []
    for phase in sorted({sample["phase"] for sample in samples} - {"reopen"}):
        selected = [sample for sample in samples if sample["phase"] == phase]
        searches = [float(sample["first_search_ns"]) for sample in selected]
        mutations = [float(sample["mutation_ns"]) for sample in selected]
        row = result_template(record, f"lifecycle:{phase}", "current-gpu")
        deletion = [sample["deleted_id_absent"] == "true" for sample in selected if phase.startswith("delete")]
        rebuilt = [sample["rebuilt"] == "true" for sample in selected]
        actual = sorted({sample["actual_backend"] for sample in selected})
        row.update(
            source_bundle_sha256=summary.get("source_bundle_sha256"),
            git_revision=summary.get("git_revision"),
            rows=int(summary["rows"]), dimensions=int(summary["dimensions"]), batch=1, k=10,
            eligible_fraction=1.0, timing_scope=summary["timing_scope"],
            filter_scope=summary["filter_scope"],
            p50_completed_ns=float(summary[f"{phase}_p50_first_search_ns"]),
            p95_completed_ns=float(summary[f"{phase}_p95_first_search_ns"]),
            p99_completed_ns=max(searches), p95_min_ns=min(searches), p95_max_ns=max(searches),
            mutation_p50_ns=statistics.median_low(mutations), mutation_p95_ns=max(mutations),
            upload_bytes_per_call=lower_median([float(sample["upload_bytes"]) for sample in selected]),
            qenlo_allocation_bytes=max(float(sample["allocation_bytes"]) for sample in selected),
            actual_backend=",".join(actual), rebuilt_fraction=sum(rebuilt) / len(rebuilt),
            deletion_correct=(all(deletion) if deletion else None), sample_count=len(selected),
            qualified=(actual == ["Wgpu"] and not any(rebuilt) and (not deletion or all(deletion))),
            status="completed",
        )
        rows.append(row)
    reopen = result_template(record, "lifecycle:reopen", "current-gpu")
    reopen.update(
        source_bundle_sha256=summary.get("source_bundle_sha256"), git_revision=summary.get("git_revision"),
        rows=int(summary["rows"]), dimensions=int(summary["dimensions"]), batch=1, k=10,
        eligible_fraction=1.0, timing_scope="completed reopen plus first completed search",
        filter_scope=summary["filter_scope"], p50_completed_ns=float(summary["reopen_first_search_ns"]),
        p95_completed_ns=float(summary["reopen_first_search_ns"]),
        p99_completed_ns=float(summary["reopen_first_search_ns"]),
        collection_build_ns=int(summary["reopen_ns"]), actual_backend="Wgpu", rebuilt_fraction=1.0,
        sample_count=1, qualified=True, status="completed",
    )
    rows.append(reopen)
    return rows


def failed_engine_row(record: dict, artifact_root: Path, status: dict[str, str]) -> dict:
    cell, engine = status["cell"], status["engine"]
    row = result_template(record, cell, engine)
    engine_path = artifact_root / "runs" / cell / engine
    config_path = engine_path / "configuration.json"
    config: dict = {}
    if config_path.exists():
        config = json.loads(config_path.read_text(encoding="utf-8"))
    elif (engine_path / "configuration.txt").exists():
        config = properties(engine_path / "configuration.txt")
    else:
        sibling = artifact_root / "runs" / cell / "current-cpu" / "configuration.txt"
        if sibling.exists():
            config = properties(sibling)
    if config:
        row.update(
            source_bundle_sha256=config.get("source_bundle_sha256") or record.get("sourceDigests", {}).get("Current"),
            git_revision=config.get("git_revision"), rows=int(config["rows"]),
            dimensions=int(config["dimensions"]), batch=int(config["batch"]),
            k=int(config.get("k", 10)), eligible_fraction=float(config["eligible_fraction_actual"]),
            timing_scope=config.get("query_latency"), filter_scope=config.get("fraction_scope"),
        )
    detail_path = engine_path / "tuning-failure.json"
    detail = detail_path.read_text(encoding="utf-8").strip() if detail_path.exists() else f"exit_code={status['exit_code']}"
    row.update(status="failed", failure_detail=detail)
    return row


def attach_binary_build(rows: list[dict], artifact_root: Path) -> None:
    path = artifact_root / "build-times.txt"
    if not path.exists():
        return
    times = properties(path)
    for row in rows:
        key = "baseline_build_ns" if row["engine"].startswith("baseline") else "current_build_ns"
        if key in times:
            row["binary_build_ns"] = int(times[key])


def failure_rows(record: dict, reason: str) -> list[dict]:
    mode = record.get("mode") or "common"
    cells = COMMON_CELLS if mode != "pilot" else (("p1-r1000-d128-b1-f1", 1000, 128, 1, 1.0),)
    rows = []
    for cell, count, dimensions, batch, fraction in cells:
        row = result_template(record, cell, "current-gpu")
        row.update(rows=count, dimensions=dimensions, batch=batch, k=10,
                   eligible_fraction=fraction, status=reason)
        rows.append(row)
    return rows


def collect(campaign: Path) -> list[dict]:
    configurations = json.loads((campaign.parent.parent / "runpod" / "small-collection-configurations.json").read_text())
    known = {item["name"]: item for item in configurations}
    rows: list[dict] = []
    for mode in ("pilot", "common", "reference", "deep", "deep768"):
        parent = campaign / mode
        if not parent.exists():
            continue
        for directory in sorted(path for path in parent.iterdir() if path.is_dir()):
            complete = directory / "complete.json"
            outcome = directory / "outcome.json"
            if outcome.exists() and not complete.exists():
                raw = json.loads(outcome.read_text(encoding="utf-8"))
                configuration = raw.get("configuration") or known[directory.name]
                rows.extend(failure_rows({"configuration": configuration, "mode": mode}, raw["status"]))
                continue
            if not complete.exists():
                continue
            record = json.loads(complete.read_text(encoding="utf-8"))
            extracted = directory / "extracted"
            artifact_root = extracted / "artifacts"
            if not artifact_root.exists():
                extracted.mkdir(exist_ok=True)
                safe_extract(directory / "artifacts.tar.gz", extracted)
            host_rows: list[dict] = []
            runs_root = artifact_root / "runs"
            if runs_root.exists():
                for cell_dir in sorted(path for path in runs_root.iterdir() if path.is_dir()):
                    for engine_dir in sorted(path for path in cell_dir.iterdir() if path.is_dir()):
                        engine = engine_dir.name
                        if (engine_dir / "configuration.txt").exists() and (engine_dir / "summary.txt").exists():
                            host_rows.append(native_row(record, cell_dir.name, engine, engine_dir,
                                                        cell_dir / f"{engine}.time"))
                        elif (engine_dir / "summary.json").exists() and (engine_dir / "runs.csv").exists():
                            host_rows.append(replay_row(record, cell_dir.name, engine, engine_dir))
            attach_binary_build(host_rows, artifact_root)
            completed_keys = {(row["workload"], row["engine"]) for row in host_rows}
            status_path = artifact_root / "status.tsv"
            if status_path.exists():
                for status in read_csv(status_path):
                    key = (status["cell"], status["engine"])
                    if status["status"] != "completed" and key not in completed_keys:
                        row = failed_engine_row(record, artifact_root, status)
                        if record.get("validWorkload") is False:
                            row["status"] = "invalid_harness"
                            row["failure_detail"] = record.get("invalidReason", "invalid workload")
                        host_rows.append(row)
            host_rows.extend(lifecycle_rows(record, artifact_root))
            rows.extend(host_rows)
    return rows


def write_report(rows: list[dict], output: Path) -> None:
    fields = list(rows[0])
    with (output / "performance-matrix.csv").open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fields)
        writer.writeheader()
        writer.writerows(rows)
    (output / "performance-matrix.json").write_text(json.dumps(rows, indent=2) + "\n", encoding="utf-8")
    comparisons = []
    index = {(row["configuration"], row["workload"], row["engine"]): row for row in rows}
    for key, improved in index.items():
        configuration, workload, engine = key
        if engine != "current-gpu" or improved["status"] != "completed" or not improved["qualified"]:
            continue
        for competitor in ("baseline-gpu", "chroma", "chroma-ef512"):
            other = index.get((configuration, workload, competitor))
            if not other or other["status"] != "completed" or not other["qualified"]:
                continue
            ratio = other["p95_completed_ns"] / improved["p95_completed_ns"]
            comparisons.append((configuration, workload, competitor, ratio))
    lines = [
        "# Qenlo small-collection performance matrix",
        "",
        f"Rows retained: {len(rows)}. Completed: {sum(row['status'] == 'completed' for row in rows)}. ",
        "P50/P95/P99 are lower medians of per-run completed-call percentiles. Failed and unavailable configurations remain in the CSV and JSON.",
        "",
        "## Qualified paired P95 comparisons",
        "",
        "| Configuration | Workload | Comparator | Comparator / improved GPU | Outcome |",
        "|---|---|---:|---:|---|",
    ]
    for configuration, workload, competitor, ratio in comparisons:
        outcome = "improved GPU faster" if ratio > 1 else "improved GPU slower"
        lines.append(f"| {configuration} | {workload} | {competitor} | {ratio:.3f}x | {outcome} |")
    if not comparisons:
        lines.append("| — | — | — | — | No comparable completed pairs |")
    lines.extend(("", "This report does not infer mobile performance from cloud NVIDIA hosts."))
    (output / "performance-report.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--campaign", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    rows = collect(args.campaign.resolve())
    if not rows:
        raise ValueError("campaign contains no completed or failed configuration records")
    args.output.mkdir(parents=True, exist_ok=True)
    write_report(rows, args.output)
    print(f"wrote {len(rows)} rows to {args.output}")


if __name__ == "__main__":
    main()
