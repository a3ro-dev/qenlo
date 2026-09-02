#!/usr/bin/env python3
"""Recompute the paper's A6000 exact-search table from raw samples."""
import csv
import json
import math
import statistics
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RAW = ROOT / "research/data/raw/2026-09-02-a6000-cuda/heldout/raw_samples.csv"
OUT = ROOT / "research/data/processed"


def percentile(values, p):
    values = sorted(values)
    rank = (len(values) - 1) * p / 100
    lo, hi = math.floor(rank), math.ceil(rank)
    return values[lo] if lo == hi else values[lo] * (hi - rank) + values[hi] * (rank - lo)


def main():
    rows = list(csv.DictReader(RAW.open(newline="")))
    OUT.mkdir(parents=True, exist_ok=True)
    grouped = {}
    for row in rows:
        grouped.setdefault((row["system"], int(row["eligible"])), []).append(int(row["latency_ns_e2e"]) / 1e6)
    result = []
    for (system, eligible), values in sorted(grouped.items(), key=lambda item: (item[0][1], item[0][0])):
        result.append({"system": system, "eligible": eligible, "samples": len(values),
                       "p50_ms": percentile(values, 50), "p95_ms": percentile(values, 95),
                       "p99_ms": percentile(values, 99), "qps_from_mean": 1000 / statistics.mean(values)})
    with (OUT / "a6000_exact_summary.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(result[0]))
        writer.writeheader(); writer.writerows(result)
    (OUT / "a6000_exact_summary.json").write_text(json.dumps(result, indent=2) + "\n")
    retained = [
        {"cohort":"windows-rtx4050","system":"cpu-exact","eligible":1000,"dimension":384,"batch":1,"p95_ms":0.2304,"recall":1.0},
        {"cohort":"windows-rtx4050","system":"gpu-rows","eligible":1000,"dimension":384,"batch":1,"p95_ms":0.6766,"recall":1.0},
        {"cohort":"windows-rtx4050","system":"gpu-predicate","eligible":1000,"dimension":384,"batch":1,"p95_ms":2.8832,"recall":1.0},
        {"cohort":"windows-rtx4050","system":"cpu-exact","eligible":100000,"dimension":384,"batch":1,"p95_ms":16.6567,"recall":0.99998},
        {"cohort":"windows-rtx4050","system":"gpu-predicate","eligible":100000,"dimension":384,"batch":1,"p95_ms":3.2404,"recall":0.99998},
        {"cohort":"windows-rtx4050","system":"gpu-rows","eligible":100000,"dimension":384,"batch":1,"p95_ms":4.2428,"recall":0.99998},
        {"cohort":"windows-rtx4050","system":"gpu-mask","eligible":100000,"dimension":384,"batch":1,"p95_ms":4.4591,"recall":0.99998},
        {"cohort":"windows-rtx4050","system":"usearch-hnsw-ef128","eligible":100000,"dimension":384,"batch":1,"p95_ms":3.6245,"recall":0.99224},
        {"cohort":"intel-arc-soak","system":"cpu-exact","eligible":100000,"dimension":384,"batch":1,"p95_ms":16.486,"recall":1.0},
        {"cohort":"intel-arc-soak","system":"gpu-predicate","eligible":100000,"dimension":384,"batch":1,"p95_ms":4.444,"recall":1.0},
        {"cohort":"intel-arc-soak","system":"gpu-predicate","eligible":100000,"dimension":384,"batch":8,"p95_ms":1.245,"recall":1.0},
        {"cohort":"intel-arc-soak","system":"automatic-cpu","eligible":1000,"dimension":384,"batch":1,"p95_ms":0.273,"recall":1.0},
        {"cohort":"intel-arc-soak","system":"automatic-gpu","eligible":1000,"dimension":384,"batch":8,"p95_ms":0.555,"recall":1.0},
    ]
    with (OUT / "retained_evidence.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(retained[0]))
        writer.writeheader(); writer.writerows(retained)
    completeness = [
        ("architecture", "measured/verified"), ("eligible-set crossover", "bounded"),
        ("dimension effect", "unmeasured"), ("batch effect", "device-lab only"),
        ("HNSW tradeoff", "one controlled point"), ("routing regret", "unmeasured"),
        ("matched corpus size", "unmeasured"), ("metadata correlation", "unmeasured")]
    with (OUT / "evidence_completeness.csv").open("w", newline="") as handle:
        writer = csv.writer(handle); writer.writerow(["question","status"]); writer.writerows(completeness)
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
