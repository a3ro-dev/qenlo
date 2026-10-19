"""Analyze E-LOCK (research/experiments/dvfs-lock/protocol.md). Prints K1-K5 and writes paper/v2/tables/lock.tex.

    python research/scripts/analyze_dvfs_lock.py
"""
import csv, json, math, random, statistics as st
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RAW = ROOT / "research/data/raw/2026-10-19-dvfs-lock"
EP = ["gpu-rows", "gpu-predicate"]
ES = [1000, 3000, 10000]
CONDS = [("U", 0), ("U", 1), ("Lhi", 0), ("Lhi", 1), ("Llo", 0)]
REQ = {"Lhi": 2670, "Llo": 585}

runs, excluded = {}, []
for d in sorted(RAW.glob("block-*/*-h[01]")):
    b = int(d.parent.name.split("-")[1]); name = d.name
    eng = "gpu-rows" if name.startswith("gpu-rows") else "gpu-predicate"
    rest = name[len(eng) + 2:]; E, cond, h = rest.split("-"); E = int(E); h = int(h[1])
    info = json.load(open(d / "lock.json"))
    S = list(csv.DictReader(open(d / "bench/samples.csv")))
    sel = st.median(int(s["device_selection_ns"] or 0) for s in S) / 1e6
    call = st.median(int(s["batch_latency_ns"]) for s in S) / 1e6
    # observed SM clock while GPU active (utilization > 0)
    clk = []
    for line in open(d / "clocks.csv"):
        f = [x.strip() for x in line.split(",")]
        if len(f) > 4 and f[1].isdigit() and f[4].isdigit() and int(f[4]) > 0:
            clk.append(int(f[1]))
    sm = st.median(clk) if clk else float("nan")
    bad = info["returncode"] != 0 or info["thermal_abort"]
    if cond in REQ and not bad and abs(sm - REQ[cond]) > 0.05 * REQ[cond]:
        bad = True
    rec = dict(block=b, engine=eng, E=E, cond=cond, h=h, sel=sel, call=call, sm=sm, tmax=info["temperature_max_c"])
    (excluded if bad else runs.__setitem__) and None
    if bad:
        excluded.append(rec)
    else:
        runs[(b, eng, E, cond, h)] = rec

print(f"runs loaded: {len(runs) + len(excluded)}, excluded: {len(excluded)}")
for r in excluded:
    print("  EXCLUDED", r)
print("max temperature:", max(r["tmax"] for r in list(runs.values()) + excluded))


def vals(eng, E, cond, h, key="sel"):
    return [r[key] for (b, e, n, c, hh), r in sorted(runs.items()) if (e, n, c, hh) == (eng, E, cond, h)]


def med_ratio(eng, E, ca, cb, key="sel"):
    """median over blocks of per-block ratio ca/cb, each (cond, heater)."""
    rs = []
    for b in range(5):
        x, y = runs.get((b, eng, E, *ca)), runs.get((b, eng, E, *cb))
        if x and y:
            rs.append(x[key] / y[key])
    return st.median(rs) if rs else float("nan"), rs


def boot(rs, n=20000):
    g = random.Random(1); m = sorted(st.median(g.choices(rs, k=len(rs))) for _ in range(n))
    return m[int(.025 * n)], m[int(.975 * n)]


print("\n== medians over blocks (ms), n, max/min, SM clock while active")
tab = {}
for eng in EP:
    for E in ES:
        for c, h in CONDS:
            v = vals(eng, E, c, h); sm = [r["sm"] for r in runs.values() if (r["engine"], r["E"], r["cond"], r["h"]) == (eng, E, c, h)]
            tab[(eng, E, c, h)] = (st.median(v), len(v), max(v) / min(v), st.median(sm))
            print(f"{eng:14s} E={E:6d} {c:3s} h{h}  med {st.median(v):.4f}  n={len(v)}  max/min {max(v)/min(v):.2f}  SM {st.median(sm):.0f}")

print("\n== K1 light max/min under Lhi-h0 (<=1.15); unlocked U-h0 for reference")
for E in ES:
    print(f"E={E}: Lhi-h0 {tab[('gpu-rows', E, 'Lhi', 0)][2]:.2f}   U-h0 {tab[('gpu-rows', E, 'U', 0)][2]:.2f}   Lhi-h1 {tab[('gpu-rows', E, 'Lhi', 1)][2]:.2f}")

print("\n== K2 light Lhi-h0 median vs best of U-h0/U-h1 medians (within 15%)")
for E in ES:
    best = min(tab[("gpu-rows", E, "U", 0)][0], tab[("gpu-rows", E, "U", 1)][0])
    best_run = min(vals("gpu-rows", E, "U", 0) + vals("gpu-rows", E, "U", 1))
    m = tab[("gpu-rows", E, "Lhi", 0)][0]
    print(f"E={E}: Lhi {m:.4f}  best unlocked median {best:.4f} ({m/best:.2f}x)  best unlocked single run {best_run:.4f} ({m/best_run:.2f}x)")

print("\n== K3 E=10000 Llo-h0 median vs D1 laptop heater-off median 0.461 (within 25%)")
m = tab[("gpu-rows", 10000, "Llo", 0)][0]; print(f"Llo {m:.4f}  ratio {m/0.461:.2f}")
for E in ES:
    print(f"  Llo-h0 light E={E}: {tab[('gpu-rows', E, 'Llo', 0)][0]:.4f} vs Lhi-h0 {tab[('gpu-rows', E, 'Lhi', 0)][0]:.4f}")

print("\n== K4 heater on/off ratio of light device time (median over blocks [95% CI]); want >=0.95 under Lhi")
for cond in ["U", "Lhi"]:
    for E in ES:
        r, rs = med_ratio("gpu-rows", E, (cond, 1), (cond, 0)); lo, hi = boot(rs)
        print(f"{cond:3s} E={E:6d}: {r:.2f} [{lo:.2f}, {hi:.2f}]")

print("\n== K5 heavy Lhi-h0 / U-h0 median (within 3%)")
for E in ES:
    print(f"E={E}: {tab[('gpu-predicate', E, 'Lhi', 0)][0] / tab[('gpu-predicate', E, 'U', 0)][0]:.3f}"
          f"   (heavy max/min Lhi {tab[('gpu-predicate', E, 'Lhi', 0)][2]:.2f}, U {tab[('gpu-predicate', E, 'U', 0)][2]:.2f})")

rows = []
for E in ES:
    c = lambda eng, cond, h: tab[(eng, E, cond, h)]
    L = lambda cond, h: c("gpu-rows", cond, h); H = c("gpu-predicate", "Lhi", 0)
    rows.append(f"{E:,} & {L('U',0)[0]:.3f} & {L('U',0)[2]:.2f} & {L('Lhi',0)[0]:.3f} & {L('Lhi',0)[2]:.2f} & "
                f"{L('Llo',0)[0]:.3f} & {L('Llo',0)[2]:.2f} & {H[0]:.3f} & {H[2]:.2f} \\\\")
(ROOT / "paper/v2/tables/lock.tex").write_text("\n".join(rows) + "\n")
print("\nwrote paper/v2/tables/lock.tex")
