#!/usr/bin/env python3
"""Run the held-out local CPU/GPU/automatic routing gate for alpha.5."""

from __future__ import annotations

import argparse
import json
import os
import platform
import subprocess
from pathlib import Path


WORKLOADS = (
    # Boundary sweep. These values were chosen before measuring this suite.
    ("d128-b8-e488-k1", 10_000, 128, 8, 488, 1),
    ("d128-b8-e733-k10", 10_000, 128, 8, 733, 10),
    ("d128-b8-e977-k64", 10_000, 128, 8, 977, 64),
    ("d128-b8-e1221-k10", 10_000, 128, 8, 1_221, 10),
    ("d128-b8-e1465-k1", 10_000, 128, 8, 1_465, 1),
    ("d384-b1-e1302-k1", 10_000, 384, 1, 1_302, 1),
    ("d384-b1-e1953-k10", 10_000, 384, 1, 1_953, 10),
    ("d384-b1-e2604-k64", 10_000, 384, 1, 2_604, 64),
    ("d384-b1-e3255-k10", 10_000, 384, 1, 3_255, 10),
    ("d384-b1-e3906-k1", 10_000, 384, 1, 3_906, 1),
    ("d768-b16-e41-k1", 10_000, 768, 16, 41, 1),
    ("d768-b16-e61-k10", 10_000, 768, 16, 61, 10),
    ("d768-b16-e81-k64", 10_000, 768, 16, 81, 64),
    ("d768-b16-e102-k10", 10_000, 768, 16, 102, 10),
    ("d768-b16-e122-k1", 10_000, 768, 16, 122, 1),
    # One all-row cell at the candidate boundary.
    ("all-r1000-d128-b8-k10", 1_000, 128, 8, 1_000, 10),
)
ENGINES = ("cpu", "gpu-rows", "automatic")


def run(command: list[str], env: dict[str, str]) -> None:
    print(" ".join(command), flush=True)
    subprocess.run(command, check=True, env=env)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output", type=Path,
        default=Path("research/data/raw/alpha5-router-heldout-rtx4050"),
    )
    parser.add_argument("--skip-build", action="store_true")
    args = parser.parse_args()
    source = Path("crates/qenlo/src/lib.rs").read_text(encoding="utf-8")
    if "AUTOMATIC_GPU_MIN_WORK" not in source:
        raise SystemExit(
            "this historical runner requires the rejected alpha.5 candidate source; "
            "the release intentionally reverted it (see research/data/processed/alpha5-routing)"
        )
    args.output.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env.setdefault("WGPU_BACKEND", "dx12")
    if not args.skip_build:
        run(["cargo", "build", "--locked", "--release", "-p", "qenlo-bench", "--features", "gpu-wgpu"], env)
    binary = Path("target/release/qenlo-bench.exe")
    if not binary.exists():
        binary = Path("target/release/qenlo-bench")
    if not binary.exists():
        raise SystemExit("release qenlo-bench binary not found")

    datasets: dict[tuple[int, int], Path] = {}
    for _, rows, dimension, _, _, _ in WORKLOADS:
        key = rows, dimension
        if key in datasets:
            continue
        dataset = args.output / f"dataset-r{rows}-d{dimension}.qnb"
        datasets[key] = dataset
        if not dataset.exists():
            run([
                str(binary), "prepare", "--dataset", str(dataset), "--rows", str(rows),
                "--dimensions", str(dimension), "--tuning", "128", "--evaluation", "640",
                "--seed", "5505",
            ], env)

    manifest = {
        "schema": "qenlo-alpha5-router-heldout-v1",
        "status": "incomplete-until-analyzed",
        "backend": env["WGPU_BACKEND"],
        "host": platform.platform(),
        "candidate": {
            "rule": "GPU when eligible_rows * dimension * batch >= threshold",
            "work_threshold": 1_000_000,
            "selection_status": "pre-registered before this suite",
        },
        "dataset": {"seed": 5505, "tuning_queries": 128, "evaluation_queries": 640},
        "measurement": {"warmups": 64, "repetitions": 5, "recall_target": 0.99},
        "workloads": [
            {"name": n, "rows": r, "dimension": d, "batch": b, "eligible_rows": e,
             "k": k, "work_units": e * d * b}
            for n, r, d, b, e, k in WORKLOADS
        ],
    }
    with (args.output / "manifest.json").open("w", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(manifest, indent=2) + "\n")

    for index, (name, rows, dimension, batch, eligible_rows, k) in enumerate(WORKLOADS):
        dataset = datasets[(rows, dimension)]
        order = ("cpu",) + (("gpu-rows", "automatic") if index % 2 == 0 else ("automatic", "gpu-rows"))
        for engine in order:
            output = args.output / name / engine
            if (output / "summary.txt").exists():
                continue
            output.parent.mkdir(parents=True, exist_ok=True)
            command = [
                str(binary), "run", "--dataset", str(dataset), "--output", str(output),
                "--dimensions", str(dimension), "--backend", engine,
                "--distribution", "independent", "--eligible-count", str(eligible_rows),
                "--batch", str(batch), "--k", str(k), "--warmups", "64",
                "--repetitions", "5", "--recall-target", "0.99", "--order-seed", str(8_000 + index),
                "--gpu-row-preparation", "one-pass", "--diagnostics", "detailed",
            ]
            cpu = args.output / name / "cpu"
            if engine != "cpu":
                command += ["--oracle-reference", str(cpu)]
            run(command, env)
    manifest["status"] = "completed"
    with (args.output / "manifest.json").open("w", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(manifest, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
