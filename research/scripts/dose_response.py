"""Dose-response: does the heater's effect on the light kernel scale with how slow that kernel was without it?

For each (host, block, E) with both heater arms: deficit = light device time (heater off) / best heater-off light
device time on that GPU model at that E; effect = heater-on / heater-off light device time. Same for heavy path.
    python research/scripts/dose_response.py LAPTOP_RAW D1R_RAW H100_RAW
"""
import csv, math, statistics as st, sys
from pathlib import Path
from scipy.stats import spearmanr

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "research/data/processed/dvfs-pooled"
hosts = [("RTX 4050 Laptop", sys.argv[1]), ("RTX 4090 pods", sys.argv[2]), ("H100 SXM", sys.argv[3])]
med = lambda d, k: st.median(int(s[k] or 0) for s in csv.DictReader(open(d / "bench/samples.csv"))) / 1e6
rows = []
for label, raw in hosts:
    for blk in sorted(Path(raw).glob("block-*")):
        for eng in ("gpu-rows", "gpu-predicate"):
            for E in (100, 1000, 3000, 10000, 30000):
                off, on = blk / f"{eng}-e{E}-h0", blk / f"{eng}-e{E}-h1"
                if (off / "bench/summary.txt").exists() and (on / "bench/summary.txt").exists():
                    rows.append({"gpu": label, "block": blk.name, "engine": eng, "E": E,
                                 "sel_off": med(off, "device_selection_ns"), "sel_on": med(on, "device_selection_ns")})
for r in rows:
    best = min(x["sel_off"] for x in rows if x["gpu"] == r["gpu"] and x["engine"] == r["engine"] and x["E"] == r["E"])
    r["deficit"] = r["sel_off"] / best
    r["effect"] = r["sel_on"] / r["sel_off"]
with open(OUT / "dose_response.csv", "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)
for eng in ("gpu-rows", "gpu-predicate"):
    x = [r for r in rows if r["engine"] == eng]
    rho, p = spearmanr([r["deficit"] for r in x], [r["effect"] for r in x])
    big = [r["effect"] for r in x if r["deficit"] >= 1.5]
    small = [r["effect"] for r in x if r["deficit"] < 1.1]
    print(f"{eng}: n={len(x)} spearman(deficit, effect)={rho:.2f} p={p:.1e}; "
          f"median effect when deficit>=1.5: {st.median(big) if big else float('nan'):.2f} (n={len(big)}); "
          f"when deficit<1.1: {st.median(small) if small else float('nan'):.2f} (n={len(small)})")
