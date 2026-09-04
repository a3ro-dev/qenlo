#!/usr/bin/env python3
"""Recompute every processed table used by the paper from retained artifacts."""

from __future__ import annotations

import csv
import io
import json
import math
import statistics
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "research/data/processed"


def percentile(values: list[float], p: float) -> float:
    values = sorted(values)
    rank = (len(values) - 1) * p / 100
    lo, hi = math.floor(rank), math.ceil(rank)
    return values[lo] if lo == hi else values[lo] * (hi - rank) + values[hi] * (rank - lo)


def write_csv(name: str, rows: list[dict]) -> None:
    if not rows:
        raise ValueError(f"refusing to write empty table {name}")
    with (OUT / name).open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]), lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def key_value_summary(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            result[key] = value
    return result


def data_rows(path: Path) -> int:
    with path.open(encoding="utf-8") as handle:
        return max(sum(1 for _ in handle) - 1, 0)


def archive_text(archive: tarfile.TarFile, member: str) -> str:
    handle = archive.extractfile(member)
    if handle is None:
        raise FileNotFoundError(f"missing archive member: {member}")
    return handle.read().decode("utf-8")


def native_crossover_summary() -> list[dict]:
    base = ROOT / "research/data/raw/2026-09-02-native-crossover"
    result = []
    for eligible in (2_000, 3_000, 4_000, 6_000, 8_000, 10_000):
        cpu = key_value_summary(base / f"cpu-e{eligible}" / "summary.txt")
        gpu = key_value_summary(base / f"gpu-rows-e{eligible}" / "summary.txt")
        cpu_p95 = int(cpu["median_run_p95_batch_ns"]) / 1e6
        gpu_p95 = int(gpu["median_run_p95_batch_ns"]) / 1e6
        cpu_recall = float(cpu["evaluation_recall_at_10"])
        gpu_recall = float(gpu["evaluation_recall_at_10"])
        if cpu_recall != gpu_recall:
            raise AssertionError(f"recall mismatch at E={eligible}")
        result.append({
            "eligible": eligible,
            "cpu_p95_ms": cpu_p95,
            "gpu_rows_p95_ms": gpu_p95,
            "cpu_over_gpu": cpu_p95 / gpu_p95,
            "recall_at_10": cpu_recall,
        })
    write_csv("native_crossover_summary.csv", result)
    return result


def static_router_regret(crossover: list[dict]) -> list[dict]:
    result = []
    for row in crossover:
        eligible = int(row["eligible"])
        cpu = float(row["cpu_p95_ms"])
        gpu = float(row["gpu_rows_p95_ms"])
        chosen = "cpu" if eligible < 4_096 else "wgpu-rows"
        fastest = "cpu" if cpu <= gpu else "wgpu-rows"
        chosen_latency = cpu if chosen == "cpu" else gpu
        fastest_latency = min(cpu, gpu)
        regret = chosen_latency - fastest_latency
        result.append({
            "eligible": eligible,
            "chosen_backend": chosen,
            "fastest_backend": fastest,
            "chosen_p95_ms": chosen_latency,
            "fastest_p95_ms": fastest_latency,
            "absolute_p95_regret_ms": regret,
            "relative_p95_regret": regret / fastest_latency,
            "route_correct": chosen == fastest,
        })
    write_csv("static_router_regret.csv", result)
    return result


def phase1_eligibility_ablation() -> list[dict]:
    path = ROOT / "research/artifacts/qenlo-phase1-eligibility-2026-09-04.tar.gz"
    result = []
    with tarfile.open(path, "r:gz") as archive:
        for eligible in (1_000, 4_000, 6_000, 100_000):
            for mode in ("legacy-two-pass", "one-pass", "cached"):
                prefix = f"artifacts/e{eligible}-{mode}"
                summary = {}
                for line in archive_text(archive, f"{prefix}/summary.txt").splitlines():
                    if "=" in line:
                        key, value = line.split("=", 1)
                        summary[key] = value
                runs = list(csv.DictReader(io.StringIO(archive_text(archive, f"{prefix}/runs.csv"))))
                if len(runs) != 5:
                    raise AssertionError(f"{prefix}: expected five complete runs")
                result.append({
                    "eligible": eligible,
                    "mode": mode,
                    "runs": len(runs),
                    "samples": sum(int(row["batches"]) for row in runs),
                    "median_run_p50_ms": statistics.median(int(row["p50_batch_ns"]) for row in runs) / 1e6,
                    "median_run_p95_ms": statistics.median(int(row["p95_batch_ns"]) for row in runs) / 1e6,
                    "median_run_p99_ms": statistics.median(int(row["p99_batch_ns"]) for row in runs) / 1e6,
                    "recall_at_10": float(summary["evaluation_recall_at_10"]),
                })
    write_csv("eligibility_ablation_summary.csv", result)
    return result


def a6000_synthetic() -> list[dict]:
    raw = ROOT / "research/data/raw/2026-09-02-a6000-cuda/heldout/raw_samples.csv"
    rows = list(csv.DictReader(raw.open(newline="", encoding="utf-8")))
    grouped: dict[tuple[str, int], list[float]] = {}
    for row in rows:
        grouped.setdefault((row["system"], int(row["eligible"])), []).append(
            int(row["latency_ns_e2e"]) / 1e6
        )
    result = []
    for (system, eligible), values in sorted(grouped.items(), key=lambda item: (item[0][1], item[0][0])):
        result.append({
            "system": system,
            "eligible": eligible,
            "samples": len(values),
            "p50_ms": percentile(values, 50),
            "p95_ms": percentile(values, 95),
            "p99_ms": percentile(values, 99),
            "qps_from_mean": 1000 / statistics.mean(values),
        })
    write_csv("a6000_exact_summary.csv", result)
    (OUT / "a6000_exact_summary.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return result


def windows_summary() -> list[dict]:
    base = ROOT / "benchmarks/2026-08-28/real"
    specs = [
        ("cpu-384-onepct", "CPU exhaustive FP64 accum.", 1_000, "exhaustive"),
        ("gpu-rows-384-onepct", "WGPU compact rows", 1_000, "exhaustive"),
        ("gpu-predicate-384-onepct-v4", "WGPU predicate", 1_000, "exhaustive"),
        ("cpu-384-all-v4", "CPU exhaustive FP64 accum.", 100_000, "exhaustive"),
        ("gpu-rows-384-all-v4", "WGPU compact rows", 100_000, "exhaustive"),
        ("gpu-mask-384-all-v4", "WGPU mask", 100_000, "exhaustive"),
        ("gpu-predicate-384-all-v5", "WGPU predicate", 100_000, "exhaustive"),
        ("usearch-384-all-v4", "USearch HNSW ef=128", 100_000, "ANN"),
        ("chroma-384-all-ef128", "Chroma HNSW ef=128", 100_000, "ANN"),
    ]
    result = []
    for directory, system, eligible, search_type in specs:
        summary_path = next((base / directory).glob("summary.*"))
        if summary_path.suffix == ".json":
            summary = json.loads(summary_path.read_text(encoding="utf-8"))
            p95_ns = summary["median_run_p95_batch_ns"]
            recall = summary["evaluation_recall_at_10"]
        else:
            summary = key_value_summary(summary_path)
            p95_ns = int(summary["median_run_p95_batch_ns"])
            recall = float(summary["evaluation_recall_at_10"])
        result.append({
            "system": system,
            "search_type": search_type,
            "eligible": eligible,
            "dimension": 384,
            "runs": data_rows(base / directory / "runs.csv"),
            "samples": data_rows(base / directory / "samples.csv"),
            "p95_ms": p95_ns / 1e6,
            "recall_at_10": recall,
        })
    write_csv("windows_summary.csv", result)
    return result


def linux_library_summary() -> list[dict]:
    base = ROOT / "benchmark-results/2026-09-02-runpod-archive/retained-archive/artifacts"
    specs = [
        ("qenlo-cpu", "Qenlo CPU", "exhaustive"),
        ("qenlo-usearch", "USearch", "ANN"),
        ("faiss-flat", "FAISS Flat", "exhaustive"),
        ("faiss-hnsw", "FAISS HNSW", "ANN"),
        ("chroma", "Chroma HNSW", "ANN"),
        ("milvus", "Milvus Lite AUTOINDEX", "ANN"),
        ("lancedb-flat", "LanceDB Flat", "exhaustive"),
    ]
    result = []
    for directory, system, search_type in specs:
        summary_path = next((base / directory).glob("summary.*"))
        if summary_path.suffix == ".json":
            summary = json.loads(summary_path.read_text(encoding="utf-8"))
            p95_ns = summary.get("median_run_p95_ns", summary.get("median_run_p95_batch_ns"))
            recall = summary["evaluation_recall_at_10"]
        else:
            summary = key_value_summary(summary_path)
            p95_ns = int(summary["median_run_p95_batch_ns"])
            recall = float(summary["evaluation_recall_at_10"])
        result.append({
            "system": system,
            "search_type": search_type,
            "runs": data_rows(base / directory / "runs.csv"),
            "samples": data_rows(base / directory / "samples.csv"),
            "p95_ms": p95_ns / 1e6,
            "recall_at_10": recall,
        })
    write_csv("linux_library_summary.csv", result)
    return result


def strict_a6000_summary() -> list[dict]:
    base = ROOT / "benchmark-results/2026-09-02-runpod-a6000-strict-research-gate"
    rows = json.loads((base / "strict-gate/summary.json").read_text(encoding="utf-8"))
    rows += json.loads((base / "strict-qenlo-isolated/summary.json").read_text(encoding="utf-8"))
    labels = {
        "numpy_cpu_exact_fp32": "NumPy CPU exhaustive",
        "faiss_gpu_flat_ip": "FAISS GPU Flat",
        "qenlo_cuda_predicate_prototype": "research CUDA exhaustive prototype",
        "cuvs_brute_force_ip": "cuVS brute force (disqualified)",
    }
    result = []
    for row in rows:
        result.append({
            "system": labels[row["system"]],
            "samples": row["samples"],
            "p50_ms": row["p50_ms"],
            "p95_ms": row["p95_ms"],
            "p99_ms": row["p99_ms"],
            "recall_at_10": row["mean_recall_at_10"],
            "qualified": row["mean_recall_at_10"] == 1.0,
        })
    write_csv("a6000_strict_summary.csv", result)
    return result


def inventory() -> list[dict]:
    cohorts = [
        ("Windows native endpoints", ROOT / "benchmarks/2026-08-28/real", "samples.csv", 225_000),
        ("Windows native crossover", ROOT / "research/data/raw/2026-09-02-native-crossover", "samples.csv", 300_000),
        ("Linux library cohort", ROOT / "benchmark-results/2026-09-02-runpod-archive/retained-archive/artifacts", "samples.csv", 10_500),
        ("A6000 synthetic held-out", ROOT / "research/data/raw/2026-09-02-a6000-cuda/heldout", "raw_samples.csv", 4_000),
        ("A6000 real strict gate", ROOT / "benchmark-results/2026-09-02-runpod-a6000-strict-research-gate", "raw_samples.csv", 20_000),
    ]
    result = []
    for name, directory, pattern, expected in cohorts:
        if name == "A6000 real strict gate":
            files = [directory / "strict-gate/raw_samples.csv", directory / "strict-qenlo-isolated/raw_samples.csv"]
        elif name == "Windows native endpoints":
            files = [p for p in directory.glob(f"*/{pattern}") if p.parent.name != "cpu-384-all-v2"]
        else:
            files = list(directory.rglob(pattern))
        samples = sum(data_rows(path) for path in files)
        if samples != expected:
            raise AssertionError(f"{name}: expected {expected} samples, found {samples}")
        result.append({"cohort": name, "timed_observations": samples, "evidence_form": "raw per-call"})
    android = sum(int(row["samples"]) for row in csv.DictReader((OUT / "android_device_lab.csv").open(encoding="utf-8")))
    result.append({"cohort": "Android device lab", "timed_observations": android, "evidence_form": "author-supplied aggregates"})
    phase1 = phase1_eligibility_ablation()
    phase1_samples = sum(int(row["samples"]) for row in phase1)
    if phase1_samples != 300_000:
        raise AssertionError(f"Phase 1 eligibility ablation: expected 300000 samples, found {phase1_samples}")
    result.append({"cohort": "Runpod eligibility ablation", "timed_observations": phase1_samples, "evidence_form": "raw per-call archive"})
    write_csv("evidence_inventory.csv", result)
    return result


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    crossover = native_crossover_summary()
    tables = {
        "native_crossover": crossover,
        "static_router_regret": static_router_regret(crossover),
        "eligibility_ablation": phase1_eligibility_ablation(),
        "a6000_synthetic": a6000_synthetic(),
        "windows": windows_summary(),
        "linux": linux_library_summary(),
        "a6000_strict": strict_a6000_summary(),
        "inventory": inventory(),
    }
    print(json.dumps({name: len(rows) for name, rows in tables.items()}, indent=2))


if __name__ == "__main__":
    main()
