"""Verify archived tuning evidence using only the Python standard library."""

import csv
import hashlib
import math
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent


def fields(path):
    return dict(line.split("=", 1) for line in path.read_text().splitlines() if "=" in line)


def rows(path):
    with path.open(newline="") as source:
        return list(csv.DictReader(source))


def verify():
    entries = [line.split("  ", 1) for line in (ROOT / "SHA256SUMS").read_text().splitlines()]
    assert len(entries) == len({name for _, name in entries}), "duplicate manifest entry"
    for expected, name in entries:
        actual = hashlib.sha256((ROOT / name).read_bytes()).hexdigest()
        assert actual == expected, f"SHA256 mismatch: {name}"
    covered = {name for _, name in entries}
    expected_files = {
        path.relative_to(ROOT).as_posix() for path in ROOT.rglob("*")
        if path.is_file() and path.name != "SHA256SUMS" and "__pycache__" not in path.parts
    }
    assert covered == expected_files, "manifest does not cover every retained file"

    repository = ROOT.parents[2]
    provenance = fields(ROOT / "source-provenance.txt")
    if (repository / ".git").exists():
        for label in ["original", "optimized"]:
            revision = provenance[f"{label}_revision"]
            for name in ["crates/qenlo/src/gpu.rs", "crates/qenlo/src/gpu_exact.wgsl", "Cargo.lock"]:
                blob = subprocess.check_output(["git", "show", f"{revision}:{name}"], cwd=repository)
                assert hashlib.sha256(blob).hexdigest() == provenance[f"{label}_git_blob_sha256_{name}"]
    dataset = repository / "target/gpu-synthetic-100k384.qnb"
    if dataset.exists():
        with dataset.open("rb") as source:
            assert hashlib.file_digest(source, "sha256").hexdigest() == provenance["dataset_sha256"]

    directories = sorted(ROOT.glob("gpu-*/configuration.txt"))
    assert len(directories) == 8
    total_samples = 0
    for configuration in directories:
        directory = configuration.parent
        config = fields(configuration)
        summary = fields(directory / "summary.txt")
        samples = rows(directory / "samples.csv")
        runs = rows(directory / "runs.csv")
        assert config["dataset_crc32"] == "8b058757"
        assert config["rows"] == "100000" and config["dimensions"] == "384"
        assert config["batch"] == "1" and config["diagnostics"] == "basic"
        assert summary["status"] == "completed"
        assert summary["filter_violations"] == "0"
        assert float(summary["evaluation_recall_at_10"]) == 1
        assert len(runs) == int(config["repetitions"])
        p95s = []
        for run in runs:
            run_samples = [sample for sample in samples if sample["run"] == run["run"]]
            assert len(run_samples) == 20
            assert {int(sample["query_indices"]) for sample in run_samples} == set(range(20))
            latencies = sorted(int(sample["batch_latency_ns"]) for sample in run_samples)
            for percentile, column in [(0.50, "p50_batch_ns"), (0.95, "p95_batch_ns"), (0.99, "p99_batch_ns")]:
                assert latencies[math.ceil(percentile * len(latencies)) - 1] == int(run[column])
            p95s.append(int(run["p95_batch_ns"]))
            for sample in run_samples:
                assert float(sample["recall_at_10"]) == 1
                assert sample["fallback"] == "false"
                assert sample["actual_backend"] == ("Cpu" if config["backend"] == "cpu" else "Wgpu")
        assert int(summary["median_run_p95_batch_ns"]) == sorted(p95s)[(len(p95s) - 1) // 2]
        total_samples += len(samples)
    for backend in ["dx12", "vulkan"]:
        transcript = (ROOT / f"tests-{backend}.txt").read_text()
        assert "14 passed; 0 failed" in transcript and "SKIP GPU" not in transcript
        assert "NVIDIA GeForce RTX 4050 Laptop GPU" in transcript
    print(f"Verified {len(entries)} SHA256 hashes, 8 workload cells, {total_samples} timed queries, and both hardware test transcripts.")


if __name__ == "__main__":
    verify()
