#!/usr/bin/env python3
"""Independent statistics/compatibility checker for retained TopK GPU raw samples."""
import argparse, csv, hashlib, json, math, random
from collections import defaultdict
from pathlib import Path

def percentile(xs, p):
    xs = sorted(xs); i = (len(xs) - 1) * p
    lo, hi = int(i), math.ceil(i)
    return xs[lo] if lo == hi else xs[lo] + (xs[hi] - xs[lo]) * (i - lo)

def boot_median(xs, seed):
    rng = random.Random(seed); n = len(xs); medians = []
    for _ in range(1000): medians.append(percentile([xs[rng.randrange(n)] for _ in range(n)], .5))
    return percentile(medians, .025), percentile(medians, .975)

def main():
    ap = argparse.ArgumentParser(); ap.add_argument("root"); ap.add_argument("--allow-inexact", action="store_true"); args = ap.parse_args()
    root = Path(args.root); groups = defaultdict(list); errors = []
    for path in sorted(root.glob("*/raw_samples.csv")):
        with path.open(newline="") as f:
            for row in csv.DictReader(f):
                key = (row["system"], int(row["threshold"]))
                groups[key].append(row)
                if float(row["recall_at_10"]) != 1.0 or int(row["incorrect_top10"]) != 0: errors.append((str(path), row))
    if errors and not args.allow_inexact: raise SystemExit(f"correctness failure: {len(errors)} rows; first={errors[0]}")
    summary = []
    for key, rows in sorted(groups.items()):
        samples = [int(r["latency_ns_e2e"])/1e6 for r in rows]
        reps = sorted({int(r["rep"]) for r in rows})
        if len(rows) != len(reps) * 1000: raise SystemExit(f"incomplete raw data for {key}: {len(rows)}")
        seed = int.from_bytes(hashlib.sha256(f"{key[0]}:{key[1]}".encode()).digest()[:4], "big")
        lo, hi = boot_median(samples, seed)
        recalls=[float(r["recall_at_10"]) for r in rows]
        summary.append({"system": key[0], "int_filter_lte": key[1], "samples": len(rows), "repetitions": len(reps),
          "mean_recall_at_10": sum(recalls)/len(recalls), "min_recall_at_10": min(recalls), "perfect_recall_rows": sum(x == 1.0 for x in recalls), "p50_ms": percentile(samples,.5), "p95_ms": percentile(samples,.95),
          "p99_ms": percentile(samples,.99), "median_bootstrap_95_ci_ms": [lo,hi],
          "raw_sha256": hashlib.sha256((root / {"qenlo_cuda_predicate_prototype":"qenlo","faiss_gpu_flat_ip":"faiss","cuvs_brute_force_ip":"cuvs"}[key[0]] / "raw_samples.csv").read_bytes()).hexdigest()})
    (root / "summary.json").write_text(json.dumps(summary, indent=2))
    with (root / "summary.csv").open("w", newline="") as f:
        w=csv.DictWriter(f, fieldnames=summary[0].keys()); w.writeheader(); w.writerows(summary)
    print(json.dumps(summary, indent=2))

if __name__ == "__main__": main()
