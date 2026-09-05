#!/usr/bin/env python3
"""Replay one native Qenlo cell against a tensor or exact flat engine.

The completed native cell is authoritative for corpus bytes, metadata, filter,
FP64 truth, k, batch boundaries, and shuffled query order. Host timing ends only
after IDs and cosine distances are materialized as NumPy arrays.
"""
from __future__ import annotations

import argparse
import csv
import gc
import hashlib
import importlib.metadata
import json
import math
import os
import platform
import sys
import time
from pathlib import Path

try:
    import resource
except ImportError:  # Windows does not provide the POSIX resource module.
    resource = None

os.environ.setdefault("OMP_NUM_THREADS", "2")
os.environ.setdefault("MKL_NUM_THREADS", "2")
os.environ.setdefault("OPENBLAS_NUM_THREADS", "2")
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "sdk/python/src"))
sys.path.insert(0, str(ROOT / "scripts"))

import numpy as np

from chroma_replay import eligible, load_dataset, properties, read_metadata


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def percentile(values: list[int], fraction: float) -> int:
    return sorted(values)[max(0, math.ceil(len(values) * fraction) - 1)]


def package_version(name: str) -> str:
    return importlib.metadata.version(name)


def peak_process_rss_kib() -> int | None:
    if resource is None:
        return None
    value = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    # macOS reports bytes; Linux reports KiB.
    return value // 1024 if sys.platform == "darwin" else value


def normalize(values: np.ndarray) -> np.ndarray:
    result = np.asarray(values, dtype=np.float32).copy(order="C")
    scale = np.max(np.abs(result), axis=1, keepdims=True)
    if not np.isfinite(result).all() or np.any(scale == 0):
        raise ValueError("vectors must be finite and nonzero")
    result /= scale
    result /= np.linalg.norm(result.astype(np.float64), axis=1, keepdims=True).astype(np.float32)
    return result


class NumpyFlat:
    package = "numpy"
    device = "cpu"

    def __init__(self, vectors: np.ndarray, ids: np.ndarray):
        self.vectors = normalize(vectors)
        self.ids = ids.copy()
        self.allocation_bytes = self.vectors.nbytes + self.ids.nbytes

    def search(self, queries: np.ndarray, k: int):
        query = normalize(queries)
        distances = 1.0 - query @ self.vectors.T
        count = min(k, len(self.ids))
        positions = np.empty((len(query), count), dtype=np.int64)
        for row in range(len(query)):
            positions[row] = np.lexsort((self.ids, distances[row]))[:count]
        return self.ids[positions], np.take_along_axis(distances, positions, axis=1), None


class FaissFlat:
    package = "faiss-cpu"
    device = "cpu"
    allocation_bytes = None

    def __init__(self, vectors: np.ndarray, ids: np.ndarray, threads: int):
        import faiss

        faiss.omp_set_num_threads(threads)
        self.faiss = faiss
        self.index = faiss.IndexIDMap2(faiss.IndexFlatIP(vectors.shape[1]))
        self.index.add_with_ids(normalize(vectors), ids)

    def search(self, queries: np.ndarray, k: int):
        scores, ids = self.index.search(normalize(queries), min(k, self.index.ntotal))
        distances = 1.0 - scores
        for row in range(len(ids)):
            order = np.lexsort((ids[row], distances[row]))
            ids[row], distances[row] = ids[row, order], distances[row, order]
        return ids, distances, None


class TorchFlat:
    package = "torch"

    def __init__(self, vectors: np.ndarray, ids: np.ndarray, device: str):
        import torch
        from qenlo import TorchIndex

        self.torch = torch
        self.device = device
        self.index = TorchIndex(vectors, ids, device=device)
        self.allocation_bytes = self.index.allocation_bytes

    def search(self, queries: np.ndarray, k: int):
        event_start = event_end = None
        if self.device == "cuda":
            event_start = self.torch.cuda.Event(enable_timing=True)
            event_end = self.torch.cuda.Event(enable_timing=True)
            event_start.record()
        ids, distances = self.index.search(queries, k)
        if event_end is not None:
            event_end.record()
        host_ids = ids.cpu().numpy()
        host_distances = distances.cpu().numpy()
        device_ns = None
        if event_start is not None and event_end is not None:
            event_end.synchronize()
            device_ns = int(event_start.elapsed_time(event_end) * 1_000_000)
        return host_ids, host_distances, device_ns


def validate(
    ids: np.ndarray,
    distances: np.ndarray,
    queries: np.ndarray,
    corpus: np.ndarray,
    metadata: list[dict[str, int]],
    config: dict[str, str],
    truths: list[list[int]],
    k: int,
) -> float:
    if ids.shape != distances.shape or ids.shape != (len(queries), min(k, int(config["eligible_count"]))):
        raise ValueError("wrong result shape")
    recalls = []
    for got, scores, query, truth in zip(ids, distances, queries, truths, strict=True):
        got_list = [int(value) for value in got]
        if len(got_list) != len(set(got_list)) or any(
            value < 0 or value >= len(corpus) or not eligible(metadata[value], config)
            for value in got_list
        ):
            raise ValueError("invalid, duplicate, or filter-violating result ID")
        if not np.isfinite(scores).all() or np.any(scores[:-1] > scores[1:]):
            raise ValueError("nonfinite or unordered distances")
        if got_list:
            vectors = corpus[got_list].astype(np.float64)
            query64 = np.asarray(query, dtype=np.float64)
            expected = 1.0 - vectors @ query64 / (
                np.linalg.norm(vectors, axis=1) * np.linalg.norm(query64)
            )
            if np.max(np.abs(expected - scores)) > 1e-5:
                raise ValueError("distance differs from independent FP64 score")
        recalls.append(len(set(got_list) & set(truth)) / len(truth) if truth else float(not got_list))
    return float(np.mean(recalls))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--backend", choices=["torch-cpu", "torch-cuda", "faiss-flat", "numpy"], required=True)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--dataset", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--threads", type=int, default=2)
    args = parser.parse_args()
    if args.output.exists() or args.threads < 1:
        parser.error("output must not exist and threads must be positive")
    config = properties(args.reference / "configuration.txt")
    summary = properties(args.reference / "summary.txt")
    if summary.get("status") != "completed":
        raise ValueError("reference native cell is incomplete")
    corpus, tuning, evaluation = load_dataset(args.dataset, config)
    metadata = read_metadata(args.reference / "metadata.csv", len(corpus))
    eligible_ids = np.asarray(
        [row["id"] for row in metadata if eligible(row, config)], dtype=np.int64
    )
    if len(eligible_ids) != int(config["eligible_count"]):
        raise ValueError("metadata eligibility differs from native reference")
    k = int(config["k"])
    with (args.reference / "truth.csv").open(newline="", encoding="utf-8") as stream:
        truth = {
            (row["split"], int(row["query_index"])): [int(value) for value in row["ids"].split(";") if value]
            for row in csv.DictReader(stream)
        }
    with (args.reference / "samples.csv").open(newline="", encoding="utf-8") as stream:
        reference_samples = list(csv.DictReader(stream))
    grouped: dict[int, list[list[int]]] = {}
    for row in reference_samples:
        indices = [int(value) for value in row["query_indices"].split(";")]
        grouped.setdefault(int(row["run"]), []).append(indices)
    args.output.mkdir(parents=True)
    build_started = time.perf_counter_ns()
    vectors = corpus[eligible_ids]
    if args.backend == "numpy":
        adapter = NumpyFlat(vectors, eligible_ids)
    elif args.backend == "faiss-flat":
        adapter = FaissFlat(vectors, eligible_ids, args.threads)
    else:
        adapter = TorchFlat(vectors, eligible_ids, args.backend.removeprefix("torch-"))
    build_ns = time.perf_counter_ns() - build_started
    for index, query in enumerate(tuning):
        ids, distances, _ = adapter.search(np.asarray([query]), k)
        validate(ids, distances, np.asarray([query]), corpus, metadata, config, [truth[("tuning", index)]], k)
    for index in range(int(config["warmup_queries"])):
        adapter.search(np.asarray([tuning[index % len(tuning)]]), k)

    sample_rows: list[dict[str, object]] = []
    run_rows: list[dict[str, object]] = []
    for run, batches in sorted(grouped.items()):
        latencies: list[int] = []
        recalls: list[float] = []
        wall_started = time.perf_counter_ns()
        for batch_index, indices in enumerate(batches):
            queries = np.asarray(evaluation[indices])
            started = time.perf_counter_ns()
            ids, distances, device_ns = adapter.search(queries, k)
            completed_ns = time.perf_counter_ns() - started
            recall = validate(
                ids, distances, queries, corpus, metadata, config,
                [truth[("evaluation", index)] for index in indices], k,
            )
            latencies.append(completed_ns)
            recalls.append(recall * len(indices))
            sample_rows.append({
                "engine": args.backend, "run": run, "batch_index": batch_index,
                "query_indices": ";".join(map(str, indices)), "query_count": len(indices),
                "completed_ns": completed_ns, "device_resident_ns": device_ns,
                "k": k, "recall_at_k": recall, "recall_at_10": recall if k == 10 else None,
            })
        wall_ns = time.perf_counter_ns() - wall_started
        mean_recall = sum(recalls) / len(evaluation)
        run_rows.append({
            "engine": args.backend, "run": run, "batches": len(batches), "queries": len(evaluation),
            "p50_completed_ns": percentile(latencies, 0.5),
            "p95_completed_ns": percentile(latencies, 0.95),
            "p99_completed_ns": percentile(latencies, 0.99),
            "wall_ns": wall_ns, "qps": len(evaluation) * 1e9 / wall_ns,
            "k": k, "recall_at_k": mean_recall,
            "recall_at_10": mean_recall if k == 10 else None,
        })
    for name, rows in (("samples.csv", sample_rows), ("runs.csv", run_rows)):
        with (args.output / name).open("w", newline="", encoding="utf-8") as stream:
            writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
            writer.writeheader()
            writer.writerows(rows)
    recalls = [float(row["recall_at_k"]) for row in run_rows]
    p95s = [int(row["p95_completed_ns"]) for row in run_rows]
    result = {
        "status": "completed",
        "format": "qenlo-small-replay-v1",
        "engine": args.backend,
        "algorithm": "exhaustive cosine",
        "package": adapter.package,
        "package_version": package_version(adapter.package),
        "device": adapter.device,
        "rows": len(corpus),
        "eligible_rows": len(eligible_ids),
        "dimension": corpus.shape[1],
        "batch": int(config["batch"]),
        "k": k,
        "build_and_filter_prepare_ns": build_ns,
        "allocation_bytes": adapter.allocation_bytes,
        "peak_process_rss_kib": peak_process_rss_kib(),
        "median_run_p95_completed_ns": percentile(p95s, 0.5),
        "mean_recall_at_k": float(np.mean(recalls)),
        "minimum_run_recall_at_k": min(recalls),
        "recall_target": float(config["recall_target"]),
        "qualified": min(recalls) + 1e-12 >= float(config["recall_target"]),
        "timing_scope": "host query input through host-visible IDs and cosine distances",
        "filter_scope": "prefiltered canonical eligible rows; build_and_filter_prepare_ns reported separately",
        "dataset_sha256": digest(args.dataset),
        "reference_configuration_sha256": digest(args.reference / "configuration.txt"),
        "python": sys.version,
        "platform": platform.platform(),
    }
    (args.output / "summary.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    gc.collect()
    print(json.dumps(result, indent=2))
    return 0 if result["qualified"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
