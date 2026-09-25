"""Summarize DVFS cell directories: latency percentiles and in-run SM clock."""
import csv, json, statistics as st, sys
from pathlib import Path


def summarize(d: Path):
    d = Path(d)
    run = json.load(open(d / "run.json"))
    t0, t1 = run["started_unix"], run["ended_unix"]
    clocks = []
    for line in open(d / "clocks.csv"):
        f = [x.strip() for x in line.split(",")]
        if len(f) > 2 and f[1].isdigit():
            clocks.append(int(f[1]))
    runs = list(csv.DictReader(open(d / "bench" / "runs.csv")))
    mid = lambda xs: sorted(xs)[(len(xs) - 1) // 2]
    lat = [int(r["batch_latency_ns"]) for r in csv.DictReader(open(d / "bench" / "samples.csv"))]
    return {
        "dir": d.name,
        "p50_ms": mid([int(r["p50_batch_ns"]) for r in runs]) / 1e6,
        "p95_ms": mid([int(r["p95_batch_ns"]) for r in runs]) / 1e6,
        "mean_ms": st.mean(lat) / 1e6,
        "clk_median": st.median(clocks) if clocks else None,
        "clk_p10": sorted(clocks)[len(clocks) // 10] if clocks else None,
        "recall": runs[0].get("recall_at_10"),
        "heater": run.get("heater"),
        "rc": run["returncode"],
    }


if __name__ == "__main__":
    for d in sys.argv[1:]:
        print(summarize(Path(d)))
