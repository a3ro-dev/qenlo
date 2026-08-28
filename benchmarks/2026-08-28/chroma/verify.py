"""Recompute archived Chroma recall and nearest-rank P95 from raw samples."""

import csv
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def percentile(values, fraction):
    return sorted(values)[math.ceil(len(values) * fraction) - 1]


def verify(name, reference):
    path = ROOT / name
    summary = json.loads((path / "summary.json").read_text())
    config = json.loads((path / "configuration.json").read_text())
    with (ROOT / reference / "truth.csv").open(newline="") as stream:
        truth = {int(row["query_index"]): set(row["ids"].split(";")) - {""}
                 for row in csv.DictReader(stream) if row["split"] == "evaluation"}
    with (path / "samples.csv").open(newline="") as stream:
        samples = list(csv.DictReader(stream))
    runs = {}
    for sample in samples:
        indices = [int(value) for value in sample["query_indices"].split(";")]
        results = json.loads(sample["result_ids"])
        assert len(results) == len(indices) == int(sample["query_count"])
        recalls = []
        for index, ids in zip(indices, results):
            assert len(ids) == len(set(ids)) == len(truth[index])
            recalls.append(len(set(ids) & truth[index]) / len(truth[index]) if truth[index] else float(not ids))
        assert abs(sum(recalls) / len(recalls) - float(sample["recall_at_10"])) < 1e-12
        run = runs.setdefault(sample["run"], dict(indices=[], latencies=[], recalls=[]))
        run["indices"].extend(indices)
        run["latencies"].append(int(sample["batch_latency_ns"]))
        run["recalls"].extend(recalls)
    p95s, recalls = [], []
    for run in runs.values():
        assert sorted(run["indices"]) == sorted(truth)
        p95s.append(percentile(run["latencies"], .95))
        recalls.append(sum(run["recalls"]) / len(run["recalls"]))
    assert summary["median_run_p95_batch_ns"] == percentile(p95s, .5)
    assert abs(summary["evaluation_recall_at_10"] - sum(recalls) / len(recalls)) < 1e-12
    passed = min(recalls + [summary["tuning_recall_at_10"]]) + 1e-12 >= float(config["recall_target"])
    assert summary["recall_target_passed"] == passed
    print(f"{name}: {len(samples)} batches, recall={summary['evaluation_recall_at_10']}, P95={percentile(p95s, .5)}ns, pass={passed}")


if __name__ == "__main__":
    verify("compound-smoke", "compound-smoke-reference")
    verify("all-ef8192", "../gpu-tuning/gpu-parallel-large-100k384-all")
    verify("onepct-ef128", "../gpu-tuning/gpu-parallel-100k384-onepct-cpu")
    verify("invalid-runtime-ef-sweep", "../gpu-tuning/gpu-parallel-large-100k384-all")
