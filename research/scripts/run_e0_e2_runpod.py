"""Run the preregistered E0 noise floor and E2 representation ablation on Runpod.

The runner is resumable, records every command and clock sample, randomizes engine
order within blocks, and refuses to overwrite incomplete or completed runs.
"""
from __future__ import annotations

import hashlib
import json
import os
import random
import subprocess
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
BINARY = ROOT / "target/release/qenlo-bench"
DATASET = ROOT / "data/ag-news/ag-news-100k-384.qnb"
OUT = Path(os.environ.get("QENLO_RUNPOD_OUT", "/workspace/qenlo-campaign/e0-e2"))
DEADLINE_SECONDS = int(os.environ.get("QENLO_DEADLINE_SECONDS", str(6 * 60 * 60)))


def sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def clocks() -> str:
    result = subprocess.run(
        [
            "nvidia-smi",
            "--query-gpu=timestamp,name,uuid,clocks.sm,temperature.gpu,power.draw",
            "--format=csv,noheader",
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def completed_run_issue(destination: Path, engine: str) -> str | None:
    """Return why a summary-bearing destination is unsafe to resume past."""
    summary = destination / "summary.txt"
    if not summary.is_file():
        return "summary missing"
    record_path = destination.parent / f"{engine}.run.json"
    if not record_path.is_file():
        return "run record missing"
    try:
        record = json.loads(record_path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        return f"run record unreadable: {error}"
    if record.get("returncode") != 0:
        return f"process exit code is {record.get('returncode')!r}, not 0"
    try:
        values = dict(
            line.split("=", 1) for line in summary.read_text().splitlines() if "=" in line
        )
    except OSError as error:
        return f"summary unreadable: {error}"
    if values.get("status") != "completed":
        return f"summary status is {values.get('status')!r}"
    if values.get("recall_target_passed") != "true":
        return f"recall gate is {values.get('recall_target_passed')!r}"
    return None


def run_cell(*, experiment: str, block: int, engine: str, eligible: int, batch: int) -> None:
    destination = OUT / experiment / f"b{batch}-e{eligible}" / f"block-{block:02d}" / engine
    summary = destination / "summary.txt"
    if summary.is_file():
        if issue := completed_run_issue(destination, engine):
            raise SystemExit(f"Invalid completed destination retained: {destination}: {issue}")
        return
    if destination.exists():
        raise SystemExit(f"Incomplete destination retained: {destination}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    command = [
        str(BINARY),
        "run",
        "--dataset",
        str(DATASET),
        "--output",
        str(destination),
        "--dimensions",
        "384",
        "--backend",
        engine,
        "--distribution",
        "independent",
        "--eligible-count",
        str(eligible),
        "--batch",
        str(batch),
        "--k",
        "10",
        "--warmups",
        "200",
        "--repetitions",
        "3",
        "--recall-target",
        "0.99",
        "--order-seed",
        str(9001 + block),
        "--diagnostics",
        "detailed",
        "--vector-budget-mib",
        "1024",
        "--gpu-budget-mib",
        "2048",
    ]
    before = clocks()
    started = time.time()
    completed = subprocess.run(command, cwd=ROOT, env={**os.environ, "WGPU_BACKEND": "vulkan"})
    after = clocks()
    record = {
        "command": command,
        "started_unix": started,
        "elapsed_seconds": time.time() - started,
        "returncode": completed.returncode,
        "clocks_before": before,
        "clocks_after": after,
    }
    (destination.parent / f"{engine}.run.json").write_text(json.dumps(record, indent=2) + "\n")
    if completed.returncode != 0 or not summary.is_file():
        raise SystemExit(f"Run failed: {destination}")
    values = dict(line.split("=", 1) for line in summary.read_text().splitlines() if "=" in line)
    if values.get("status") != "completed" or values.get("recall_target_passed") != "true":
        raise SystemExit(f"Correctness gate failed: {destination}")


def main() -> None:
    if not BINARY.is_file() or not DATASET.is_file():
        raise SystemExit("Missing benchmark binary or dataset")
    OUT.mkdir(parents=True, exist_ok=True)
    manifest = {
        "source_revision": "e5d85adbb638caa05a99fb8d1ef0a96147091cf0",
        "binary_sha256": sha256(BINARY),
        "dataset_sha256": sha256(DATASET),
        "runner_sha256": sha256(Path(__file__)),
        "deadline_seconds": DEADLINE_SECONDS,
        "wgpu_backend": "vulkan",
        "cooldown_seconds": 30,
    }
    manifest_path = OUT / "manifest.json"
    if manifest_path.exists() and json.loads(manifest_path.read_text()) != manifest:
        raise SystemExit("Manifest mismatch; refusing mixed resume")
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
    deadline = time.monotonic() + DEADLINE_SECONDS

    # E0: repeated A/A observations for CPU and gpu-rows at the product-relevant sparse cell.
    for block in range(10):
        engines = ["cpu", "gpu-rows"]
        random.Random(7000 + block).shuffle(engines)
        for engine in engines:
            if time.monotonic() >= deadline:
                raise SystemExit("Six-hour experiment deadline reached")
            run_cell(experiment="e0", block=block, engine=engine, eligible=3000, batch=1)
            time.sleep(30)

    # E2: representation ablation over eligible fraction and batch size.
    for batch in (1, 16):
        for eligible in (100, 1000, 3000, 10000, 30000, 100000):
            for block in range(5):
                engines = ["gpu-rows", "gpu-predicate", "gpu-mask", "cpu"]
                random.Random(7200 + batch * 100 + eligible + block).shuffle(engines)
                for engine in engines:
                    if time.monotonic() >= deadline:
                        raise SystemExit("Six-hour experiment deadline reached")
                    run_cell(
                        experiment="e2",
                        block=block,
                        engine=engine,
                        eligible=eligible,
                        batch=batch,
                    )
                    time.sleep(30)
    (OUT / "COMPLETE").write_text(f"completed_unix={time.time()}\n")


if __name__ == "__main__":
    main()
