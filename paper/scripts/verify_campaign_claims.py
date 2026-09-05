#!/usr/bin/env python3
"""Verify the retained Runpod small-campaign claims without rerunning benchmarks.

The verifier reads only retained files. It checks the reduced CSV/JSON matrix,
headline corrected rows, lifecycle semantics, paired selector/Chroma counts,
source archive roles, and every retained SHA256SUMS entry.
"""
from __future__ import annotations

import csv
import hashlib
import json
import math
import tarfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CAMPAIGN = ROOT / "research/artifacts/runpod-small-2026-09-05"
REPORT = CAMPAIGN / "report"


def fail(message: str) -> None:
    raise AssertionError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def number(value: object) -> float | None:
    if value is None or value == "":
        return None
    return float(value)


def assert_close(actual: object, expected: float, label: str, tolerance: float = 1e-6) -> None:
    value = number(actual)
    if value is None or not math.isclose(value, expected, rel_tol=tolerance, abs_tol=tolerance):
        fail(f"{label}: expected {expected}, got {actual}")


def key_value_file(path: Path) -> dict[str, str]:
    return dict(line.split("=", 1) for line in path.read_text(encoding="utf-8").splitlines() if "=" in line)


def load_matrix() -> tuple[list[dict[str, str]], list[dict]]:
    with (REPORT / "performance-matrix.csv").open(newline="", encoding="utf-8") as stream:
        csv_rows = list(csv.DictReader(stream))
    json_rows = json.loads((REPORT / "performance-matrix.json").read_text(encoding="utf-8"))
    if len(csv_rows) != 182 or len(json_rows) != 182:
        fail(f"matrix row count mismatch: CSV={len(csv_rows)}, JSON={len(json_rows)}")
    fields = list(csv_rows[0])
    for index, (csv_row, json_row) in enumerate(zip(csv_rows, json_rows, strict=True)):
        if set(json_row) != set(fields):
            fail(f"matrix field mismatch at row {index}")
        for field in fields:
            csv_value = csv_row[field]
            json_value = json_row[field]
            if csv_value == "":
                if json_value is not None:
                    fail(f"matrix null mismatch at row {index}, {field}")
            elif isinstance(json_value, bool):
                if csv_value != str(json_value):
                    fail(f"matrix boolean mismatch at row {index}, {field}")
            elif isinstance(json_value, (int, float)):
                if not math.isclose(float(csv_value), float(json_value), rel_tol=1e-12, abs_tol=1e-12):
                    fail(f"matrix numeric mismatch at row {index}, {field}")
            elif csv_value != str(json_value):
                fail(f"matrix text mismatch at row {index}, {field}")
    return csv_rows, json_rows


def verify_matrix(rows: list[dict[str, str]]) -> None:
    counts: dict[str, int] = {}
    for row in rows:
        counts[row["status"]] = counts.get(row["status"], 0) + 1
    expected = {"completed": 131, "failed_or_unavailable": 42, "failed": 7, "invalid_harness": 2}
    if counts != expected:
        fail(f"status counts mismatch: {counts}")
    if sum(row["qualified"] == "True" for row in rows) != 130:
        fail("qualified row count mismatch")

    fixed = [
        row for row in rows
        if row["configuration"] == "rtx4090-eu-ro-secure-deep768-admission-fix"
        and row["status"] == "completed"
    ]
    if len(fixed) != 12 or any(float(row["recall_at_k"]) != 1.0 for row in fixed):
        fail("corrected archive does not contain 12 recall-1 completed rows")
    headline = {
        ("d4-r100000-d768-b1-k1-f1", "current-cpu"): 9408908,
        ("d4-r100000-d768-b1-k1-f1", "current-gpu"): 897146,
        ("d4-r100000-d768-b1-k1-f1", "faiss-flat"): 14800348,
        ("d4-r100000-d768-b1-k1-f1", "numpy"): 15342732,
        ("d4-r100000-d768-b1-k1-f1", "torch-cpu"): 18694852,
        ("d4-r100000-d768-b1-k1-f1", "torch-cuda"): 670757,
        ("d5-r100000-d768-b8-k64-f01", "current-cpu"): 28598793,
        ("d5-r100000-d768-b8-k64-f01", "current-gpu"): 896204,
        ("d5-r100000-d768-b8-k64-f01", "faiss-flat"): 3055385,
        ("d5-r100000-d768-b8-k64-f01", "numpy"): 6202396,
        ("d5-r100000-d768-b8-k64-f01", "torch-cpu"): 2936026,
        ("d5-r100000-d768-b8-k64-f01", "torch-cuda"): 384076,
    }
    index = {(row["workload"], row["engine"]): row for row in fixed}
    for key, p95 in headline.items():
        if key not in index:
            fail(f"missing corrected headline row {key}")
        assert_close(index[key]["p95_completed_ns"], p95, f"{key} P95")

    completed_unqualified = [row for row in rows if row["status"] == "completed" and row["qualified"] == "False"]
    if len(completed_unqualified) != 1:
        fail(f"expected one completed-unqualified row, got {len(completed_unqualified)}")
    row = completed_unqualified[0]
    if (row["engine"], row["workload"]) != ("usearch", "d2-r25000-d384-b64-k10-f1"):
        fail("unexpected completed-unqualified row")
    assert_close(row["recall_at_k"], 0.3453125, "USearch recall")
    assert_close(row["p95_completed_ns"], 83313304, "USearch P95")


def verify_lifecycle() -> None:
    path = (CAMPAIGN / "reference/rtx4090-eu-ro-secure-reference-retry/extracted/artifacts/lifecycle-current-gpu")
    summary = key_value_file(path / "summary.txt")
    expected_summary = {
        "add-one_p95_first_search_ns": 598309,
        "delete-one_p95_first_search_ns": 631140,
        "add-batch_p95_first_search_ns": 740094,
        "delete-batch_p95_first_search_ns": 818721,
        "reopen_ns": 52168927,
        "reopen_first_search_ns": 101295702,
    }
    for key, value in expected_summary.items():
        assert_close(summary[key], value, f"lifecycle {key}")
    with (path / "lifecycle.csv").open(newline="", encoding="utf-8") as stream:
        samples = list(csv.DictReader(stream))
    if len(samples) != 13:
        fail(f"lifecycle sample count mismatch: {len(samples)}")
    mutation = [row for row in samples if row["phase"] != "reopen"]
    if any(row["rebuilt"] != "false" for row in mutation):
        fail("mutation row rebuilt flag is not uniformly false")
    if any(row["phase"].startswith("delete") and row["deleted_id_absent"] != "true" for row in mutation):
        fail("immediate deletion check failed")
    reopen = [row for row in samples if row["phase"] == "reopen"]
    if len(reopen) != 1 or reopen[0]["rebuilt"] != "true":
        fail("reopen row does not record one resident-state rebuild")


def verify_pairs(rows: list[dict[str, str]]) -> None:
    index = {(row["configuration"], row["workload"], row["engine"]): row for row in rows}
    selector = []
    for (configuration, workload, engine), candidate in index.items():
        if engine != "current-gpu" or candidate["status"] != "completed" or candidate["qualified"] != "True":
            continue
        baseline = index.get((configuration, workload, "baseline-gpu"))
        if baseline and baseline["status"] == "completed" and baseline["qualified"] == "True":
            ratio = float(baseline["p95_completed_ns"]) / float(candidate["p95_completed_ns"])
            selector.append(ratio)
    if len(selector) != 12 or sum(value > 1 for value in selector) != 5 or sum(value < 1 for value in selector) != 7:
        fail(f"selector pair accounting mismatch: n={len(selector)} wins={sum(value > 1 for value in selector)}")

    chroma = []
    for (configuration, workload, engine), candidate in index.items():
        if engine != "current-gpu" or candidate["status"] != "completed" or candidate["qualified"] != "True":
            continue
        for comparator in ("chroma", "chroma-ef512"):
            row = index.get((configuration, workload, comparator))
            if row and row["status"] == "completed" and row["qualified"] == "True":
                chroma.append(float(row["p95_completed_ns"]) / float(candidate["p95_completed_ns"]))
    if len(chroma) != 7 or min(chroma) < 3.699 or max(chroma) < 3233:
        fail(f"Chroma pair accounting mismatch: n={len(chroma)} range={min(chroma) if chroma else None}..{max(chroma) if chroma else None}")


def verify_sources() -> None:
    manifest = json.loads((CAMPAIGN / "source-manifest.json").read_text(encoding="utf-8"))
    before = json.loads((CAMPAIGN / "source-manifest-before-admission-fix.json").read_text(encoding="utf-8"))
    baseline = CAMPAIGN / "baseline-source.tar.gz"
    current = CAMPAIGN / "current-source.tar.gz"
    if sha256(baseline) != manifest["baselineSourceBundleSha256"]:
        fail("baseline source archive hash mismatch")
    if sha256(current) != manifest["currentSourceBundleSha256"]:
        fail("current source archive hash mismatch")
    if manifest["currentSourceBundleSha256"] == before["currentSourceBundleSha256"]:
        fail("corrected and pre-fix current source hashes are not distinct")
    with tarfile.open(baseline, "r:gz") as archive:
        baseline_wgsl = archive.extractfile("crates/qenlo/src/gpu_exact.wgsl").read()
    with tarfile.open(current, "r:gz") as archive:
        current_wgsl = archive.extractfile("crates/qenlo/src/gpu_exact.wgsl").read()
    worktree_wgsl = (ROOT / "crates/qenlo/src/gpu_exact.wgsl").read_bytes()
    if baseline_wgsl != worktree_wgsl:
        fail("durable worktree WGSL differs from baseline archive WGSL")
    if baseline_wgsl == current_wgsl:
        fail("candidate and baseline WGSL unexpectedly identical")
    if b"Each lane scans" not in baseline_wgsl or b"Keep lane minima" not in current_wgsl:
        fail("selector source markers are missing")


def verify_checksums() -> tuple[int, int, int]:
    manifests = list(CAMPAIGN.rglob("SHA256SUMS"))
    checked = skipped = failures = 0
    for manifest in manifests:
        for line in manifest.read_text(encoding="utf-8").splitlines():
            parts = line.split(None, 1)
            if len(parts) != 2:
                continue
            expected, recorded = parts
            if recorded.endswith("/SHA256SUMS") and expected == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855":
                skipped += 1
                continue
            marker = "/artifacts/"
            if marker not in recorded:
                fail(f"unmapped checksum path: {recorded}")
            local = manifest.parent / recorded.split(marker, 1)[1].replace("/", "\\")
            checked += 1
            if not local.exists() or sha256(local).lower() != expected.lower():
                failures += 1
    if failures:
        fail(f"checksum failures: {failures}")
    return len(manifests), checked, skipped


def main() -> None:
    rows, _ = load_matrix()
    verify_matrix(rows)
    verify_lifecycle()
    verify_pairs(rows)
    verify_sources()
    manifest_count, checksum_count, skipped = verify_checksums()
    summary = json.loads((CAMPAIGN / "campaign-summary.json").read_text(encoding="utf-8"))
    if summary["finalKnownSpendUsd"] != 0.9400778694252952 or summary["campaignPodsRemaining"] != 0:
        fail("campaign operational summary mismatch")
    print(json.dumps({
        "status": "ok",
        "matrix_rows": len(rows),
        "checksum_manifests": manifest_count,
        "checksum_entries_verified": checksum_count,
        "checksum_self_entries_skipped": skipped,
        "result_archives": len(list(CAMPAIGN.rglob("artifacts.tar.gz"))),
        "selector_pairs": {"wins": 5, "losses": 7},
        "chroma_pairs": 7,
        "completed_unqualified": "usearch:d2-r25000-d384-b64-k10-f1"
    }, indent=2))


if __name__ == "__main__":
    main()
