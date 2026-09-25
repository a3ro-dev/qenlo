"""Analyze D1 (protocol: research/experiments/dvfs-d1/protocol.md). Read-only over raw data.

    python research/scripts/analyze_dvfs_d1.py [RAW_DIR] [OUT_DIR]
"""
import collections, csv, json, math, random, statistics as st, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RAW = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "research/data/raw/2026-09-25-dvfs-d1"
OUT = Path(sys.argv[2]) if len(sys.argv) > 2 else ROOT / "research/data/processed/dvfs-d1"
OUT.mkdir(parents=True, exist_ok=True)
mid = lambda xs: sorted(xs)[(len(xs) - 1) // 2]  # lower median, as in the harness


def load(d: Path):
    run = json.load(open(d / "run.json"))
    if run["returncode"] != 0 or not (d / "bench/summary.txt").exists():
        return None
    runs = list(csv.DictReader(open(d / "bench/runs.csv")))
    S = list(csv.DictReader(open(d / "bench/samples.csv")))
    med = lambda k: st.median(int(s[k] or 0) for s in S) / 1e6
    clk = [[x.strip() for x in l.split(",")] for l in open(d / "clocks.csv") if l.strip()]
    clk = [c for c in clk if len(c) > 3 and c[1].isdigit()]
    ps = collections.Counter(c[3] for c in clk)
    engine, e, h = d.name.rsplit("-", 2)
    return {
        "block": int(d.parent.name.split("-")[1]), "engine": engine, "E": int(e[1:]), "heater": int(h[1:]),
        "p50": mid([int(r["p50_batch_ns"]) for r in runs]) / 1e6,
        "p95": mid([int(r["p95_batch_ns"]) for r in runs]) / 1e6,
        "sel": med("device_selection_ns"), "score": med("device_scoring_ns"),
        "mat": med("row_materialization_ns"), "backend": med("backend_execution_ns"),
        "clk_med": st.median(int(c[1]) for c in clk), "mem_med": st.median(int(c[2]) for c in clk),
        # clock while the GPU is busy (utilization > 0): excludes load/oracle phases; empty for CPU-only processes
        "clk_active": st.median([int(c[1]) for c in clk if c[4].isdigit() and int(c[4]) > 0] or [0]),
        "mem_active": st.median([int(c[2]) for c in clk if c[4].isdigit() and int(c[4]) > 0] or [0]),
        "active_samples": sum(1 for c in clk if c[4].isdigit() and int(c[4]) > 0),
        "high_state": sum(v for k, v in ps.items() if k in ("P0", "P1", "P2", "P3")) / max(1, sum(ps.values())),
        "recall": float(runs[0]["recall_at_10"]), "started": run["started_unix"],
    }


rows = [r for d in sorted(RAW.glob("block-*/*")) if (d / "run.json").exists() for r in [load(d)] if r]
with open(OUT / "runs.csv", "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)
idx = {(r["block"], r["engine"], r["E"], r["heater"]): r for r in rows}


def boot_median(xs, n=20000, seed=7):
    rng = random.Random(seed)
    if len(xs) < 2:
        return (math.nan, math.nan)
    meds = sorted(st.median(rng.choices(xs, k=len(xs))) for _ in range(n))
    return meds[int(0.025 * n)], meds[int(0.975 * n) - 1]


def paired(metric, a_key, b_key):
    """median over blocks of ln(a/b), returned as a ratio with bootstrap CI and per-block ratios."""
    logs = [math.log(idx[a_key(b)][metric] / idx[b_key(b)][metric]) for b in range(5)
            if a_key(b) in idx and b_key(b) in idx and idx[b_key(b)][metric] > 0 and idx[a_key(b)][metric] > 0]
    if not logs:
        return None
    lo, hi = boot_median(logs)
    return {"n": len(logs), "ratio": math.exp(st.median(logs)), "lo": math.exp(lo), "hi": math.exp(hi),
            "per_block": [round(math.exp(x), 3) for x in logs]}


out = []
for engine in ["cpu", "gpu-rows", "gpu-predicate"]:
    for E in [100, 1000, 3000, 10000, 30000]:
        for metric in ["p50", "p95", "sel", "score", "mat"]:
            r = paired(metric, lambda b: (b, engine, E, 1), lambda b: (b, engine, E, 0))
            if r:
                out.append({"comparison": "heater_on/off", "engine": engine, "E": E, "metric": metric, **r})
for E in [100, 1000, 3000, 10000, 30000]:
    for h in [0, 1]:
        for metric in ["p50", "p95"]:
            r = paired(metric, lambda b: (b, "gpu-rows", E, h), lambda b: (b, "cpu", E, h))
            if r:
                out.append({"comparison": f"rows/cpu heater={h}", "engine": "gpu-rows", "E": E, "metric": metric, **r})
            r = paired(metric, lambda b: (b, "gpu-rows", E, h), lambda b: (b, "gpu-predicate", E, h))
            if r:
                out.append({"comparison": f"rows/predicate heater={h}", "engine": "gpu-rows", "E": E, "metric": metric, **r})
with open(OUT / "paired.csv", "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=list(out[0])); w.writeheader(); w.writerows(out)

cond = collections.defaultdict(list)
for r in rows:
    cond[(r["engine"], r["E"], r["heater"])].append(r)
state = [{"engine": k[0], "E": k[1], "heater": k[2], "n": len(v),
          **{m: st.median(x[m] for x in v) for m in ["p50", "p95", "clk_med", "mem_med", "clk_active", "mem_active", "high_state", "sel", "score", "mat"]},
          "min_recall": min(x["recall"] for x in v)} for k, v in sorted(cond.items())]
with open(OUT / "conditions.csv", "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=list(state[0])); w.writeheader(); w.writerows(state)

for o in out:
    if o["metric"] in ("p50", "p95", "sel", "mat"):
        print(f'{o["comparison"]:26s} {o["engine"]:14s} E={o["E"]:6d} {o["metric"]:5s} n={o["n"]} '
              f'{o["ratio"]:.3f} [{o["lo"]:.3f}, {o["hi"]:.3f}] {o["per_block"]}')
print()
for s in state:
    print({k: (round(v, 3) if isinstance(v, float) else v) for k, v in s.items()})
