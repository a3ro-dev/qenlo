#!/usr/bin/env python3
"""Compare a transparent Qenlo CUDA prototype with FAISS GPU exact search.

This is an implementation-development experiment, not a benchmark of Qenlo's
shipped Rust/WGPU backend. Both systems search the same prefiltered FP32 matrix.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import platform
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path

import faiss
import numpy as np
import torch


def pct(values: list[float], percentile: float) -> float:
    return float(np.percentile(np.asarray(values), percentile))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--rows", type=int, default=1_000_000)
    parser.add_argument("--dimensions", type=int, default=768)
    parser.add_argument("--queries", type=int, default=100)
    parser.add_argument("--warmups", type=int, default=50)
    parser.add_argument("--repetitions", type=int, default=5)
    parser.add_argument("--eligible", type=int, nargs="+", default=[1_000, 10_000, 100_000, 1_000_000])
    parser.add_argument("--seed", type=int, default=20260902)
    args = parser.parse_args()
    if not torch.cuda.is_available():
        raise SystemExit("CUDA unavailable")
    args.output.mkdir(parents=True, exist_ok=False)

    rng = np.random.default_rng(args.seed)
    queries = rng.standard_normal((args.queries, args.dimensions), dtype=np.float32)
    queries /= np.linalg.norm(queries, axis=1, keepdims=True)
    torch.manual_seed(args.seed + 1)
    corpus = torch.randn((args.rows, args.dimensions), device="cuda", dtype=torch.float32)
    corpus = torch.nn.functional.normalize(corpus, dim=1)
    rows: list[dict[str, object]] = []
    agreements: list[dict[str, object]] = []

    for eligible in args.eligible:
        if eligible > args.rows or eligible < 10:
            raise ValueError(f"invalid eligible count: {eligible}")
        candidates = corpus[:eligible].contiguous()
        candidates_cpu = candidates.cpu().numpy()

        resources = faiss.StandardGpuResources()
        resources.setDefaultNullStreamAllDevices()
        index = faiss.GpuIndexFlatIP(resources, args.dimensions)
        index.add(candidates_cpu)

        query_device = torch.empty(args.dimensions, device="cuda", dtype=torch.float32)
        scores = torch.empty(eligible, device="cuda", dtype=torch.float32)
        values = torch.empty(10, device="cuda", dtype=torch.float32)
        positions = torch.empty(10, device="cuda", dtype=torch.int64)

        def qenlo(q: np.ndarray) -> np.ndarray:
            query_device.copy_(torch.from_numpy(q), non_blocking=False)
            torch.mv(candidates, query_device, out=scores)
            torch.topk(scores, 10, out=(values, positions), sorted=False)
            return positions.cpu().numpy().copy()

        def faiss_search(q: np.ndarray) -> np.ndarray:
            return index.search(q[None], 10)[1][0]

        for i in range(args.warmups):
            qenlo(queries[i % args.queries])
            faiss_search(queries[i % args.queries])
        torch.cuda.synchronize()

        # Cross-implementation correctness check outside the timed region.
        for qi, q in enumerate(queries):
            q_ids = set(map(int, qenlo(q)))
            f_ids = set(map(int, faiss_search(q)))
            agreements.append({"eligible": eligible, "query": qi, "set_agreement_at_10": len(q_ids & f_ids) / 10})

        systems = (("qenlo_cuda_buffered_prototype", qenlo), ("faiss_gpu_flat_ip", faiss_search))
        for repetition in range(args.repetitions):
            order = rng.permutation(args.queries)
            # Alternate system order to avoid consistently assigning one system
            # the first (colder) or second (warmer) position in each repetition.
            ordered_systems = systems if repetition % 2 == 0 else systems[::-1]
            for system, search in ordered_systems:
                for ordinal, qi in enumerate(order):
                    start = time.perf_counter_ns()
                    search(queries[qi])
                    elapsed = time.perf_counter_ns() - start
                    rows.append({"system": system, "eligible": eligible, "repetition": repetition,
                                 "ordinal": ordinal, "query": int(qi), "latency_ns_e2e": elapsed})

        del index, resources, candidates_cpu, candidates
        torch.cuda.empty_cache()

    with (args.output / "raw_samples.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
    with (args.output / "correctness.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(agreements[0]))
        writer.writeheader()
        writer.writerows(agreements)

    summary = []
    for eligible in args.eligible:
        for system in sorted({str(row["system"]) for row in rows}):
            values_ms = [int(row["latency_ns_e2e"]) / 1e6 for row in rows
                         if row["system"] == system and row["eligible"] == eligible]
            summary.append({"system": system, "eligible": eligible, "samples": len(values_ms),
                            "p50_ms": pct(values_ms, 50), "p95_ms": pct(values_ms, 95),
                            "p99_ms": pct(values_ms, 99)})
    manifest = {
        "timestamp_utc": datetime.now(timezone.utc).isoformat(),
        "qualification": "synthetic implementation-development experiment; not shipped Qenlo",
        "workload": vars(args) | {"output": str(args.output), "batch": 1, "k": 10, "metric": "cosine"},
        "timing": "host-call wall time including H2D query, GPU work, synchronization, and D2H IDs; excludes setup",
        "correctness": {"check": "unordered top-10 agreement between two exact implementations",
                        "minimum_agreement": min(float(row["set_agreement_at_10"]) for row in agreements)},
        "environment": {"python": platform.python_version(), "torch": torch.__version__,
                        "torch_cuda": torch.version.cuda, "faiss": faiss.__version__,
                        "gpu": torch.cuda.get_device_name(0),
                        "nvidia_smi": subprocess.check_output(["nvidia-smi", "--query-gpu=name,driver_version,memory.total,power.limit", "--format=csv,noheader"], text=True).strip()},
        "summary": summary,
    }
    (args.output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    checksums = {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in args.output.iterdir()}
    (args.output / "checksums.json").write_text(json.dumps(checksums, indent=2) + "\n")
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
