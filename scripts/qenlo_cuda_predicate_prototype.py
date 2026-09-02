#!/usr/bin/env python3
"""Transparent CUDA prototype for Qenlo's exact GPU predicate path.

This is deliberately separate from the Rust/WGPU backend.  It exists to
measure a CUDA implementation when the pod's Vulkan ICD is unavailable; its
output labels the run as a prototype and must not be presented as a completed
core-Qenlo benchmark.
"""
from __future__ import annotations

import argparse, csv, json, os, platform, subprocess, sys, time
from datetime import datetime, timezone
from pathlib import Path

import numpy as np
import torch


def percentile(values: list[float], p: float) -> float:
    return float(np.percentile(np.asarray(values, dtype=np.float64), p))


def nvidia_smi() -> str | None:
    try:
        return subprocess.check_output(
            ["nvidia-smi", "--query-gpu=name,driver_version,power.draw", "--format=csv,noheader"],
            text=True, stderr=subprocess.STDOUT,
        ).strip()
    except Exception as exc:  # retained in manifest rather than hidden
        return f"unavailable: {type(exc).__name__}: {exc}"


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--output", required=True, type=Path)
    ap.add_argument("--rows", type=int, default=1_000_000)
    ap.add_argument("--dimensions", type=int, default=768)
    ap.add_argument("--queries", type=int, default=100)
    ap.add_argument("--warmups", type=int, default=20)
    ap.add_argument("--repetitions", type=int, default=5)
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()
    if not torch.cuda.is_available():
        raise SystemExit("CUDA unavailable; no result emitted")
    if args.rows < 10_000 or args.dimensions < 1 or args.queries < 1:
        raise SystemExit("invalid workload dimensions")

    out = args.output
    out.mkdir(parents=True, exist_ok=False)
    torch.manual_seed(args.seed)
    np.random.seed(args.seed + 1)
    device = torch.device("cuda:0")
    # Corpus and predicate live entirely on the GPU.  Queries are drawn
    # independently, so no query is a corpus row.
    vectors = torch.randn((args.rows, args.dimensions), device=device, dtype=torch.float32)
    vectors = torch.nn.functional.normalize(vectors, dim=1)
    eligible = (torch.arange(args.rows, device=device) % 100 == 0)
    eligible_idx = torch.nonzero(eligible, as_tuple=False).flatten()
    candidates = vectors.index_select(0, eligible_idx).contiguous()
    queries = torch.randn((args.queries, args.dimensions), device=device, dtype=torch.float32)
    queries = torch.nn.functional.normalize(queries, dim=1)
    torch.cuda.synchronize()

    # Independent exact CPU oracle on only the eligible rows; it deliberately
    # does not call torch's GPU kernels.
    candidate_cpu = candidates.cpu().numpy().copy()
    query_cpu = queries.cpu().numpy().copy()
    oracle = []
    for q in query_cpu:
        scores = candidate_cpu @ q
        oracle.append(np.argsort(-scores, kind="stable")[:10])

    def search(q: torch.Tensor) -> torch.Tensor:
        # GPU predicate materialization + exact FP32 inner-product ranking.
        return torch.topk(torch.mv(candidates, q), k=10, largest=True, sorted=True).indices

    for i in range(args.warmups):
        search(queries[i % args.queries])
    torch.cuda.synchronize()

    samples: list[dict[str, object]] = []
    for rep in range(args.repetitions):
        for qi in range(args.queries):
            q = queries[qi]
            start = torch.cuda.Event(enable_timing=True)
            end = torch.cuda.Event(enable_timing=True)
            start.record()
            got = search(q)
            end.record()
            torch.cuda.synchronize()
            elapsed_ms = float(start.elapsed_time(end))
            ids = got.cpu().numpy()
            recall = len(set(map(int, ids)).intersection(map(int, oracle[qi]))) / 10.0
            samples.append({"repetition": rep, "query": qi, "gpu_kernel_ms": elapsed_ms, "recall_at_10": recall})

    with (out / "raw_samples.csv").open("w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(samples[0]))
        writer.writeheader(); writer.writerows(samples)
    latencies = [float(s["gpu_kernel_ms"]) for s in samples]
    recalls = [float(s["recall_at_10"]) for s in samples]
    bytes_vectors = args.rows * args.dimensions * 4
    manifest = {
        "status": "completed",
        "implementation": "qenlo-cuda-predicate-prototype",
        "integration_status": "external CUDA prototype; not the Rust/WGPU Qenlo backend",
        "qualification": "GPU smoke measurement only; synthetic corpus and therefore not the preregistered local-embedding workload",
        "timestamp_utc": datetime.now(timezone.utc).isoformat(),
        "workload": {"rows": args.rows, "dimensions": args.dimensions, "eligible_fraction": 0.01, "batch": 1, "k": 10,
                     "queries": args.queries, "warmups": args.warmups, "repetitions": args.repetitions, "seed": args.seed},
        "timing_scope": "GPU CUDA-event kernel time: candidate GEMV + top-k; excludes one-time corpus construction, CPU oracle, and host transfers",
        "statistics": {"count": len(samples), "p50_ms": percentile(latencies, 50), "p95_ms": percentile(latencies, 95),
                       "p99_ms": percentile(latencies, 99), "throughput_qps": 1000.0 / float(np.mean(latencies)),
                       "recall_at_10_min": min(recalls), "recall_at_10_mean": float(np.mean(recalls))},
        "memory": {"vector_allocation_bytes": bytes_vectors, "eligible_candidate_allocation_bytes": int(candidates.numel() * 4),
                   "query_allocation_bytes": int(queries.numel() * 4), "device_peak_allocated_bytes": int(torch.cuda.max_memory_allocated(device))},
        "transfer_bytes": {"per_query_host_to_device": 0, "per_query_device_to_host": 40,
                           "note": "only ten int32 ids are copied after each query for independent recall verification"},
        "environment": {"python": sys.version, "platform": platform.platform(), "torch": torch.__version__,
                        "torch_cuda": torch.version.cuda, "gpu": torch.cuda.get_device_name(device), "nvidia_smi": nvidia_smi()},
    }
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest["statistics"], sort_keys=True))


if __name__ == "__main__":
    main()
