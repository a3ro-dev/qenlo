#!/usr/bin/env python3
"""Create a bounded inventory of retained research evidence.

The inventory deliberately excludes derived paper outputs and never reads
large vector/database payloads. Text and machine-readable records are hashed;
large archives and payloads carry an existing manifest/sidecar reference when
one is available and are marked ``hash_skipped`` otherwise.
"""
from __future__ import annotations

import hashlib
import json
import re
from collections import defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "paper/audit/archive-inventory.json"
ROOTS = [ROOT / "research", ROOT / "benchmarks", ROOT / "docs/reports", ROOT / "benchmark-results"]
# The retained tree is about 1 GB. Hash individual payloads up to 256 MiB;
# this covers the retained qnb/source archives while keeping the index bounded.
MAX_HASH_BYTES = 256 * 1024 * 1024
SHA_RE = re.compile(r"(?i)(?<![0-9a-f])[0-9a-f]{64}(?![0-9a-f])")

TEXT_SUFFIXES = {
    ".md", ".txt", ".csv", ".json", ".jsonl", ".toml", ".yaml", ".yml", ".xml",
    ".svg", ".sha256", ".patch", ".log", ".out", ".err", ".stderr", ".stdout",
    ".command", ".cfg", ".ini", ".html", ".tex", ".bib",
}
ARCHIVE_SUFFIXES = {".gz", ".tgz", ".zip", ".xz", ".bz2", ".zst"}
DATASET_SUFFIXES = {".qnb", ".f32", ".h5", ".hdf5", ".npz", ".bin", ".sqlite3", ".db"}
IMAGE_SUFFIXES = {".png", ".jpg", ".jpeg", ".webp", ".pdf"}

KNOWN_ARCHIVE_HASHES = {
    "research/artifacts/phase0-baseline-head-minimal.tar.gz": "989819886531103140642a7bf85ce2a6236c7bec61a16e8fda862263d5c49581",
    "research/artifacts/qenlo-phase0-runpod-2026-09-03.tar.gz": "bdb93dfdda104eb1fe0ccfc0b52c53bb6202ad189460ea67faab04d848429c74",
    "research/artifacts/qenlo-phase0-runpod-complete-2026-09-04.tar.gz": "7292fd1cd0e7669c9febeaa2d49baedee747811ed65f6bc704ecbcb6e61f51ea",
    "research/artifacts/qenlo-phase1-eligibility-2026-09-04.tar.gz": "26e0ccc057c59c0fb4687f8d85a923e88b7bb25257e4e551e22cffe13c1b821f",
    "research/artifacts/qenlo-phase2-a40-2026-09-04-final.tar.gz": "ac3afda446f4943eab128cff458402f7559be53b96c3cf8ce44fcbbd7f8be4df",
    "benchmarks/runpod/2026-09-02-bb51987/qenlo-runpod-artifacts.tar.gz": "d46b3a098f4d92b8bd21ae329e41a3be8b9b9117f3737c4600f7849c682bf9a3",
}

COHORT_DECISIONS = {
    "small_collection_campaign": {"decision": "use", "role": "primary current campaign", "reason": "Corrected 1K--100K campaign with retained matrix, failures, resource/lifecycle evidence, and source manifests."},
    "windows_agnews_endpoints": {"decision": "use", "role": "historical contextual cohort", "reason": "Retained AG News 100K x 384 completed-call endpoints; revisions and representation boundaries remain explicit."},
    "windows_native_crossover": {"decision": "use", "role": "historical primary crossover", "reason": "Same recorded revision across the six CPU/compact-row pairs; bounded post-hoc reversal and static-policy regret."},
    "eligibility_ablation": {"decision": "use", "role": "historical preparation ablation", "reason": "Immutable RTX 4090/Vulkan archive; one-pass/cached preparation comparison with FP64 checks."},
    "android_device_lab": {"decision": "use", "role": "platform context", "reason": "Physical Mali-G615 aggregate evidence; no raw calls, power, or thermal semantics retained."},
    "intel_arc_device_lab": {"decision": "use", "role": "platform context", "reason": "Physical Intel Arc/Vulkan aggregate evidence; JSON suite labels and quick/full discrepancy preserved."},
    "a6000_strict_gate": {"decision": "use", "role": "negative external boundary", "reason": "Independent FP64 oracle and qualified rows, but research CUDA prototype loses to FAISS and fails the 2x gate."},
    "a6000_synthetic": {"decision": "context_only", "role": "exploratory implementation result", "reason": "PyTorch/CUDA prototype with pre-materialized eligibility and cross-implementation agreement, no independent FP64 oracle."},
    "a6000_provider_approximate": {"decision": "exclude_headline", "role": "historical failed-gate context", "reason": "Provider <= filter semantics, prefiltered matrix, supplied ground truth, and prototype implementation."},
    "linux_external_replay": {"decision": "use", "role": "scoped external context", "reason": "Seven-library 100K x 384 replay; API, persistence, filtering, and ANN boundaries differ and Qenlo GPU failed closed."},
    "gpu_tuning": {"decision": "context_only", "role": "implementation/optimization provenance", "reason": "Source-provenance and tuning evidence explain revisions; no universal performance claim."},
    "phase_archives": {"decision": "use", "role": "historical provenance and failure records", "reason": "Phase 0/1/2 archives preserve source hashes, raw runs, failures, and methodology limits."},
    "historical_processed": {"decision": "use", "role": "derived reductions", "reason": "Recomputed from retained raw evidence; semantic row equality verified by reduction log."},
    "benchmark_results_mirrors": {"decision": "use", "role": "mirror/archive integrity", "reason": "Extracted and replay archives preserve external summaries, failures, and sidecar checksums; exact duplicates are marked."},
    "benchmarks": {"decision": "use", "role": "raw verification and provenance", "reason": "Retained benchmark configurations, per-call samples, platform checks, and source provenance; numerical claims remain limited to ledger-qualified records."},
    "research": {"decision": "use", "role": "methodology and archive metadata", "reason": "Research plans, logs, scripts, manifests, and integrity records define methods and provenance; raw or machine-readable records take precedence over prose summaries."},
    "historical_reports": {"decision": "use", "role": "scope and negative-result documentation", "reason": "Reports retain failures, qualification gates, and interpretation limits; headline values are traced to machine-readable artifacts where available."},
}


def rel(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def classify(path: Path) -> str:
    suffix = path.suffix.lower()
    if suffix in TEXT_SUFFIXES:
        return "text_or_machine_readable"
    if path.name.lower().endswith(tuple(ARCHIVE_SUFFIXES)) or path.name.lower().endswith(".tar.gz"):
        return "binary_archive"
    if suffix in DATASET_SUFFIXES:
        return "binary_dataset_or_database"
    if suffix in IMAGE_SUFFIXES:
        return "binary_figure_or_document"
    return "binary_or_other"


def cohort(path: Path) -> str:
    p = rel(path).lower()
    if "runpod-small-2026-09-05" in p or "small-collection-configurations" in p:
        return "small_collection_campaign"
    if "native-crossover" in p:
        return "windows_native_crossover"
    if "phase1-eligibility" in p or "eligibility_ablation" in p or "phase1-" in p:
        return "eligibility_ablation"
    if "2026-09-02-android" in p or "device-lab/android" in p or "android_device_lab" in p:
        return "android_device_lab"
    if "intel-arc" in p:
        return "intel_arc_device_lab"
    if "strict-research-gate" in p or "a6000-strict" in p:
        return "a6000_strict_gate"
    if "sota-a6000" in p or "topk-a6000" in p:
        return "a6000_provider_approximate"
    if "a6000-cuda" in p or "a6000-cuda-prototype" in p:
        return "a6000_synthetic"
    if "2026-09-02-runpod-archive" in p or "runpod/2026-09-02-bb51987" in p:
        return "linux_external_replay"
    if "2026-08-28/real" in p:
        return "windows_agnews_endpoints"
    if "gpu-tuning" in p:
        return "gpu_tuning"
    if ("research/artifacts/phase0-" in p
            or "research/artifacts/qenlo-phase0-" in p
            or "research/artifacts/phase2-" in p
            or "research/artifacts/qenlo-phase2-" in p):
        return "phase_archives"
    if "research/data/processed" in p:
        return "historical_processed"
    if p.startswith("benchmark-results/"):
        return "benchmark_results_mirrors"
    if p.startswith("docs/reports/"):
        return "historical_reports"
    return p.split("/")[0]


def nearby_manifest_refs(path: Path) -> list[dict[str, str]]:
    refs = []
    candidates = [path.parent / n for n in ("manifest.json", "checksums.json", "verification.json", "CAMPAIGN_MANIFEST.json", "SHA256SUMS", "README.md", "PROVENANCE.md")]
    candidates += list(path.parent.glob("*.sha256"))
    candidates += list(path.parent.glob("*manifest*.json"))
    for candidate in candidates:
        if not candidate.exists() or candidate == path or candidate.stat().st_size > 2 * 1024 * 1024:
            continue
        try:
            text = candidate.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        matches = sorted(set(SHA_RE.findall(text)))
        if matches:
            refs.append({"path": rel(candidate), "sha256_values": matches[:20]})
    return refs


def digest(path: Path, size: int) -> tuple[str | None, str]:
    known = KNOWN_ARCHIVE_HASHES.get(rel(path))
    if size > MAX_HASH_BYTES:
        if known:
            return known, "existing_manifest_reference"
        return None, "skipped_large_payload"
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(chunk)
    actual = h.hexdigest()
    if known:
        return actual, "computed_sha256_matches_manifest" if actual == known else "computed_sha256_manifest_mismatch"
    return actual, "computed_sha256"


def main() -> None:
    files = []
    seen = set()
    for root in ROOTS:
        for path in root.rglob("*"):
            if not path.is_file():
                continue
            key = path.resolve()
            if key in seen:
                continue
            seen.add(key)
            size = path.stat().st_size
            sha, status = digest(path, size)
            files.append({
                "path": rel(path),
                "size_bytes": size,
                "type": classify(path),
                "cohort": cohort(path),
                "sha256": sha,
                "hash_status": status,
                "manifest_references": nearby_manifest_refs(path),
            })
    files.sort(key=lambda x: x["path"])
    by_hash = defaultdict(list)
    for item in files:
        if item["sha256"]:
            by_hash[item["sha256"]].append(item["path"])
    for item in files:
        paths = by_hash.get(item["sha256"], [])
        if len(paths) > 1:
            item["duplicate_of"] = paths[0] if paths[0] != item["path"] else None
            item["duplicate_group_size"] = len(paths)
        else:
            item["duplicate_of"] = None
    counts = defaultdict(int)
    bytes_by_cohort = defaultdict(int)
    for item in files:
        counts[item["cohort"]] += 1
        bytes_by_cohort[item["cohort"]] += item["size_bytes"]
    payload = {
        "schema": "qenlo-evidence-archive-inventory-v1",
        "generated_by": "paper/scripts/inventory_evidence.py",
        "scope": ["research/**", "benchmarks/**", "docs/reports/**", "benchmark-results/**"],
        "excluded_paths": ["paper/** (derived manuscript, figures, reductions, and audit outputs)"],
        "hash_policy": {"max_computed_bytes": MAX_HASH_BYTES, "large_payload_behavior": "use existing manifest/sidecar reference where known; otherwise hash=null and hash_status=skipped_large_payload"},
        "summary": {"file_count": len(files), "bytes": sum(x["size_bytes"] for x in files), "cohort_file_counts": dict(sorted(counts.items())), "cohort_bytes": dict(sorted(bytes_by_cohort.items()))},
        "cohort_decisions": COHORT_DECISIONS,
        "files": files,
    }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(payload["summary"], indent=2))


if __name__ == "__main__":
    main()
