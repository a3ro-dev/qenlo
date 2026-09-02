"""Independently verify retained Qenlo campaign artifacts without rerunning a benchmark."""

import argparse
import csv
import hashlib
import json
import math
from pathlib import Path


def sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def nearest_rank(values: list[int], fraction: float) -> int:
    return sorted(values)[max(0, math.ceil(len(values) * fraction) - 1)]


def verify_qenlo_run(directory: Path) -> dict:
    with (directory / "samples.csv").open(newline="", encoding="utf-8") as stream:
        samples = list(csv.DictReader(stream))
    with (directory / "runs.csv").open(newline="", encoding="utf-8") as stream:
        runs = list(csv.DictReader(stream))
    by_run: dict[str, list[dict]] = {}
    for sample in samples:
        by_run.setdefault(sample["run"], []).append(sample)
    errors = []
    if len(runs) != len(by_run):
        errors.append(f"run count {len(runs)} disagrees with sample groups {len(by_run)}")
    for run in runs:
        rows = by_run.get(run["run"], [])
        latencies = [int(row["batch_latency_ns"]) for row in rows]
        if not latencies:
            errors.append(f"run {run['run']} has no samples")
            continue
        for field, fraction in (("p50_batch_ns", 0.5), ("p95_batch_ns", 0.95), ("p99_batch_ns", 0.99)):
            if int(run[field]) != nearest_rank(latencies, fraction):
                errors.append(f"run {run['run']} {field} does not match samples")
        if len(rows) != int(run["batches"]):
            errors.append(f"run {run['run']} batch count does not match samples")
        if any(row["fallback"].lower() != "false" for row in rows):
            errors.append(f"run {run['run']} includes a fallback")
    return {"directory": str(directory), "samples": len(samples), "runs": len(runs), "errors": errors}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--archive-sha256", type=Path, required=True)
    parser.add_argument("--extracted", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    expected = args.archive_sha256.read_text(encoding="utf-8").split()[0].lower()
    actual = sha256(args.archive)
    archive_ok = expected == actual
    root = args.extracted / "artifacts"
    checks = [verify_qenlo_run(root / name) for name in ("qenlo-cpu", "qenlo-usearch")]
    gpu_exit = (root / "logs" / "qenlo-gpu.exit-code").read_text(encoding="utf-8").strip()
    report = {
        "format": "qenlo-independent-artifact-verification-v1",
        "archive": str(args.archive),
        "archive_sha256_expected": expected,
        "archive_sha256_actual": actual,
        "archive_checksum_passed": archive_ok,
        "qenlo_run_checks": checks,
        "retained_gpu_exit_code": gpu_exit,
        "retained_gpu_status": "failed" if gpu_exit != "0" else "completed",
        "passed": archive_ok and all(not check["errors"] for check in checks),
        "scope": "archive integrity and recomputed nearest-rank statistics only; no benchmark was executed",
    }
    if args.output.exists():
        raise FileExistsError(args.output)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
