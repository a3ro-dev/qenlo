#!/usr/bin/env python3
"""Single-thread FAISS IndexFlatIP baseline over a prepared Qenlo dataset."""

import argparse
import csv
import json
import platform
import statistics
import struct
import time
from pathlib import Path

import faiss
import numpy as np


def percentile(values: list[int], fraction: float) -> int:
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int(np.ceil(fraction * len(ordered))) - 1)]


def load_truth(path: Path) -> list[list[int]]:
    truth: dict[int, list[int]] = {}
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if row["split"] == "evaluation":
                truth[int(row["query_index"])] = [int(value) for value in row["ids"].split(";")]
    return [truth[index] for index in range(len(truth))]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dataset", type=Path, required=True)
    parser.add_argument("--truth", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--warmups", type=int, default=200)
    parser.add_argument("--repetitions", type=int, default=5)
    args = parser.parse_args()

    with args.dataset.open("rb") as handle:
        magic, dimension, corpus_rows, tuning_rows, evaluation_rows, seed, source = struct.unpack(
            "<8s6Q", handle.read(56)
        )
    if magic != b"QNLOB001":
        raise ValueError("unsupported dataset")

    rows = np.memmap(
        args.dataset,
        dtype="<f4",
        mode="r",
        offset=56,
        shape=(corpus_rows + tuning_rows + evaluation_rows, dimension),
    )
    # normalize_L2 mutates its input; copy away from the read-only mmap explicitly.
    corpus = rows[:corpus_rows].copy(order="C")
    evaluation = rows[corpus_rows + tuning_rows :].copy(order="C")
    faiss.normalize_L2(corpus)
    faiss.normalize_L2(evaluation)
    faiss.omp_set_num_threads(1)

    started = time.perf_counter_ns()
    index = faiss.IndexFlatIP(dimension)
    index.add(corpus)
    build_ns = time.perf_counter_ns() - started

    truth = load_truth(args.truth)
    if len(truth) != evaluation_rows:
        raise ValueError("truth/evaluation cardinality mismatch")
    for index_value in range(args.warmups):
        index.search(evaluation[index_value % evaluation_rows : index_value % evaluation_rows + 1], 10)

    output_runs = []
    recalls = []
    for repetition in range(args.repetitions):
        latencies = []
        matches = 0
        wall_started = time.perf_counter_ns()
        for query_index, query in enumerate(evaluation):
            call_started = time.perf_counter_ns()
            _, ids = index.search(query.reshape(1, -1), 10)
            latencies.append(time.perf_counter_ns() - call_started)
            matches += len(set(map(int, ids[0])) & set(truth[query_index]))
        wall_ns = time.perf_counter_ns() - wall_started
        recall = matches / (evaluation_rows * 10)
        recalls.append(recall)
        output_runs.append(
            {
                "run": repetition,
                "queries": evaluation_rows,
                "p50_call_ns": percentile(latencies, 0.50),
                "p95_call_ns": percentile(latencies, 0.95),
                "p99_call_ns": percentile(latencies, 0.99),
                "wall_ns": wall_ns,
                "qps": evaluation_rows * 1e9 / wall_ns,
                "recall_at_10": recall,
            }
        )

    args.output.mkdir(parents=True, exist_ok=False)
    with (args.output / "runs.csv").open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=output_runs[0].keys())
        writer.writeheader()
        writer.writerows(output_runs)
    summary = {
        "status": "completed",
        "algorithm": "faiss.IndexFlatIP",
        "faiss_version": faiss.__version__,
        "numpy_version": np.__version__,
        "python_version": platform.python_version(),
        "threads": 1,
        "dimension": dimension,
        "corpus_rows": corpus_rows,
        "evaluation_rows": evaluation_rows,
        "warmups": args.warmups,
        "repetitions": args.repetitions,
        "build_ns": build_ns,
        "median_run_p95_call_ns": int(statistics.median_low(row["p95_call_ns"] for row in output_runs)),
        "mean_recall_at_10": statistics.fmean(recalls),
        "dataset_seed": seed,
        "dataset_source": source,
    }
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, sort_keys=True))


if __name__ == "__main__":
    main()
