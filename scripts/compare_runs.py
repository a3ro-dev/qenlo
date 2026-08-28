"""Compare compatible completed Qenlo/Chroma cells, without treating failed recall as a win.

Uses only the Python standard library. The interval resamples independent whole
runs, not individual queries; five runs provide a coarse uncertainty estimate.
"""

import argparse
import csv
import hashlib
import json
import math
import random
from pathlib import Path


MATCH_FIELDS = (
    "dataset_crc32", "dimensions", "rows", "eligible_count", "batch", "metadata",
    "filter_user_id", "filter_timestamp_from", "filter_timestamp_to", "filter_mode",
    "corpus_range", "tuning_range", "evaluation_range", "seed", "k",
    "warmup_queries", "repetitions", "recall_target",
)
INTEGER_FIELDS = {"dimensions", "rows", "eligible_count", "batch", "seed", "k",
                  "warmup_queries", "repetitions"}
BOOTSTRAP_DRAWS = 10_000


def read_record(path):
    if path.suffix == ".json":
        value = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(value, dict):
            raise ValueError(f"expected object: {path}")
        return value
    result = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            if key in result:
                raise ValueError(f"duplicate field {key}: {path}")
            result[key] = value
    return result


def lower_median(values):
    return sorted(values)[(len(values) - 1) // 2]


def recall_value(value):
    value = float(value)
    if not math.isfinite(value) or not 0 <= value <= 1:
        raise ValueError("recall must be finite and within [0, 1]")
    return value


def canonical_field(key, value):
    if key in INTEGER_FIELDS:
        return int(value)
    if key.startswith("filter_") and key != "filter_mode":
        return None if value in ("", None) else int(value)
    if key == "recall_target":
        result = recall_value(value)
        if result == 0:
            raise ValueError("recall target must be positive")
        return result
    if key == "dataset_crc32":
        if len(str(value)) != 8:
            raise ValueError("dataset CRC32 must contain eight hex digits")
        return f"{int(str(value), 16):08x}"
    if key.endswith("_range"):
        start, end = map(int, str(value).split(".."))
        if start < 0 or end <= start:
            raise ValueError(f"invalid {key}")
        return [start, end]
    return str(value)


def read_run(directory):
    directory = directory.resolve()
    chroma = (directory / "configuration.json").is_file()
    suffix = ".json" if chroma else ".txt"
    config_path, summary_path = [directory / (name + suffix) for name in ("configuration", "summary")]
    config, summary = read_record(config_path), read_record(summary_path)
    if summary.get("status") != "completed":
        raise ValueError(f"run is not completed: {directory}")
    missing = set(MATCH_FIELDS) - config.keys()
    if missing:
        raise ValueError(f"missing workload fields {sorted(missing)}: {directory}")
    workload = {key: canonical_field(key, config[key]) for key in MATCH_FIELDS}
    if min(workload[key] for key in ("dimensions", "rows", "batch", "k", "repetitions")) < 1:
        raise ValueError("dimensions, rows, batch, k and repetitions must be positive")
    if not 0 <= workload["eligible_count"] <= workload["rows"] or workload["warmup_queries"] < 0:
        raise ValueError("invalid eligibility or warmup count")
    if workload["corpus_range"] != [0, workload["rows"]] or workload["tuning_range"][0] != workload["rows"] or workload["evaluation_range"][0] != workload["tuning_range"][1]:
        raise ValueError("corpus, tuning and evaluation ranges must be consecutive disjoint splits")
    expected_queries = workload["evaluation_range"][1] - workload["evaluation_range"][0]
    with (directory / "runs.csv").open(newline="", encoding="utf-8") as stream:
        rows = list(csv.DictReader(stream))
    if len(rows) != workload["repetitions"] or sorted(int(row["run"]) for row in rows) != list(range(len(rows))):
        raise ValueError("missing, duplicate, or unexpected run IDs")
    runs = []
    for row in rows:
        latency = int(row["p95_batch_ns"])
        if latency <= 0 or int(row["queries"]) != expected_queries or int(row["batches"]) != math.ceil(expected_queries / workload["batch"]):
            raise ValueError("invalid P95 or run query/batch count")
        runs.append({"run": int(row["run"]), "p95_batch_ns": latency,
                     "recall_at_10": recall_value(row["recall_at_10"])})
    runs.sort(key=lambda row: row["run"])
    recalls = [row["recall_at_10"] for row in runs]
    tuning = recall_value(summary["tuning_recall_at_10"])
    median = lower_median([row["p95_batch_ns"] for row in runs])
    if int(summary["median_run_p95_batch_ns"]) != median:
        raise ValueError("summary P95 disagrees with runs.csv")
    if not math.isclose(recall_value(summary["evaluation_recall_at_10"]), sum(recalls) / len(recalls), rel_tol=0, abs_tol=1e-12):
        raise ValueError("summary recall disagrees with runs.csv")
    if int(summary["filter_violations"]) != 0:
        raise ValueError("filter violations invalidate comparison")
    declared = str(summary["recall_target_passed"]).lower()
    if declared not in ("true", "false"):
        raise ValueError("invalid declared recall gate")
    target = workload["recall_target"]
    boundary = ("Python PersistentClient query, including validation, native bindings, serialization and local execution"
                if chroma else "Rust in-memory Collection search_batch call, including completed backend execution")
    return {
        "directory": str(directory), "backend_requested": config["backend"],
        "api_boundary": boundary, "recorded_query_latency": config.get("query_latency"),
        "platform": config.get("platform"), "diagnostics": config.get("diagnostics"),
        "workload": workload, "runs": runs, "median_run_p95_batch_ns": median,
        "min_evaluation_recall_at_10": min(recalls),
        "mean_evaluation_recall_at_10": sum(recalls) / len(recalls),
        "tuning_recall_at_10": tuning,
        "recall_gate": {"target": target, "tuning_passed": tuning >= target,
                        "every_evaluation_run_passed": min(recalls) >= target,
                        "reported_passed": declared == "true",
                        "passed": tuning >= target and min(recalls) >= target and declared == "true"},
        "artifact_sha256": {path.name: hashlib.sha256(path.read_bytes()).hexdigest()
                            for path in (config_path, summary_path, directory / "runs.csv")},
    }


def compare(baseline_directory, candidate_directory, seed=20260828):
    baseline, candidate = map(read_run, (baseline_directory, candidate_directory))
    mismatched = [key for key in MATCH_FIELDS if baseline["workload"][key] != candidate["workload"][key]]
    if mismatched:
        raise ValueError(f"incompatible workloads: {', '.join(mismatched)}")
    valid = baseline["recall_gate"]["passed"] and candidate["recall_gate"]["passed"]
    ratio, interval = None, None
    if valid:
        base = [row["p95_batch_ns"] for row in baseline["runs"]]
        other = [row["p95_batch_ns"] for row in candidate["runs"]]
        ratio = lower_median(base) / lower_median(other)
        rng = random.Random(seed)
        ratios = sorted(lower_median(rng.choices(base, k=len(base))) /
                        lower_median(rng.choices(other, k=len(other))) for _ in range(BOOTSTRAP_DRAWS))
        interval = [ratios[math.ceil(BOOTSTRAP_DRAWS * quantile) - 1] for quantile in (.025, .975)]
    return {
        "format": "qenlo-run-comparison-v1", "baseline": baseline, "candidate": candidate,
        "latency_comparison_valid": valid,
        "withheld_reason": None if valid else "at least one tuning/evaluation recall gate failed; no latency advantage claimed",
        "baseline_over_candidate_median_p95_ratio": ratio,
        "bootstrap_95_percent_interval": interval,
        "bootstrap": {"seed": seed, "draws": BOOTSTRAP_DRAWS if valid else 0,
                      "baseline_runs": len(baseline["runs"]), "candidate_runs": len(candidate["runs"]),
                      "method": "independent whole-run resampling; ratio of lower-middle medians; percentile interval",
                      "warning": "five runs give a coarse interval; this does not cover workload, hardware, thermal or scheduling changes"},
        "api_boundaries_differ": baseline["api_boundary"] != candidate["api_boundary"],
        "interpretation": "A ratio above 1 means lower candidate API latency for this cell. Different API boundaries can dominate; this is not proof of a better ANN algorithm, GPU execution, or the scale gate.",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True, help="new JSON file; never overwritten")
    parser.add_argument("--seed", type=int, default=20260828)
    args = parser.parse_args()
    result = compare(args.baseline, args.candidate, args.seed)
    with args.output.open("x", encoding="utf-8") as stream:
        json.dump(result, stream, indent=2, allow_nan=False)
        stream.write("\n")
    print(f"saved {args.output}; latency_comparison_valid={result['latency_comparison_valid']}")


if __name__ == "__main__":
    main()
