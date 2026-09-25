"""Generate LaTeX table bodies for paper/v2 from processed CSVs (no hand-typed numbers).

    python paper/v2/scripts/make_tables.py
"""
import csv, statistics as st
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
T = ROOT / "paper/v2/tables"; T.mkdir(exist_ok=True)
P = ROOT / "research/data/processed"
proc = list(csv.DictReader(open(P / "cross-host/processes.csv")))
eff = list(csv.DictReader(open(P / "dvfs-pooled/host_effects.csv")))
states = list(csv.DictReader(open(P / "dvfs-pooled/host_states.csv")))
HOSTS = [("RTX 4050 Laptop (Windows)", "RTX 4050 Laptop"), ("RTX 4090 host A (Linux, Sep 24)", "RTX 4090 host A"),
         ("RTX 4090 host B (Linux pods, Sep 25)", "RTX 4090 pods B"), ("H100 SXM (Linux pod)", "H100 SXM")]
PH = [("RTX 4050 Laptop, Windows", "RTX 4050 Laptop"), ("RTX 4090 pods, Linux", "RTX 4090 pods B"), ("H100 SXM, Linux", "H100 SXM")]
f3 = lambda x: f"{x:.3f}"


def rng(v):
    return f"{st.median(v):.3f} ({min(v):.3f}--{max(v):.3f})"


# Table: cross-host device selection time
lines = []
for E in (1000, 3000, 10000):
    for key, lab in HOSTS:
        h = [float(r["sel_ms"]) for r in proc if r["host"] == key and r["engine"] == "gpu-predicate" and int(r["E"]) == E]
        l = [float(r["sel_ms"]) for r in proc if r["host"] == key and r["engine"] == "gpu-rows" and int(r["E"]) == E]
        if h and l:
            lines.append(f"{E:,} & {lab} & {len(h)} & {rng(h)} & {max(h)/min(h):.2f} & {rng(l)} & {max(l)/min(l):.2f} \\\\")
    lines.append(r"\midrule")
(T / "cross_host.tex").write_text("\n".join(lines[:-1]) + "\n")


def e(host, eng, E, metric):
    r = [x for x in eff if x["host"] == host and x["engine"] == eng and int(x["E"]) == E and x["metric"] == metric]
    if not r:
        return "--"
    r = r[0]
    ci = "" if r["lo"] == "nan" else f" [{float(r['lo']):.2f}, {float(r['hi']):.2f}]"
    return f"{float(r['ratio']):.2f}{ci}"


lines = []
for key, lab in PH:
    for E in (100, 1000, 3000, 10000, 30000):
        row = [e(key, "gpu-rows", E, "sel"), e(key, "gpu-predicate", E, "sel"), e(key, "gpu-rows", E, "p50"), e(key, "gpu-predicate", E, "p50")]
        if row[0] != "--":
            n = [x for x in eff if x["host"] == key and int(x["E"]) == E][0]["n"]
            lines.append(f"{lab} & {E:,} & {n} & " + " & ".join(row) + " \\\\")
    lines.append(r"\midrule")
(T / "heater_effects.tex").write_text("\n".join(lines[:-1]) + "\n")

lines = []
for key, lab in PH:
    for E in (1000, 3000, 10000):
        g = {x["engine"]: x for x in states if x["host"] == key and int(x["E"]) == E and x["heater"] == "0"}
        gh = {x["engine"]: x for x in states if x["host"] == key and int(x["E"]) == E and x["heater"] == "1"}
        if "gpu-rows" in g and "gpu-predicate" in g:
            c = lambda d, k: f"{float(d[k]):.0f}"
            lines.append(f"{lab} & {E:,} & {c(g['gpu-predicate'],'clk_active')} / {c(g['gpu-predicate'],'mem_active')} & "
                         f"{c(g['gpu-rows'],'clk_active')} / {c(g['gpu-rows'],'mem_active')} & "
                         f"{c(gh['gpu-rows'],'clk_active')} / {c(gh['gpu-rows'],'mem_active')} \\\\")
    lines.append(r"\midrule")
(T / "power_states.tex").write_text("\n".join(lines[:-1]) + "\n")

# CPU/GPU crossover (median-block P50, ms), hosts with CPU arm
lines = []
for key, lab in PH[:2]:
    for E in (100, 1000, 3000, 10000, 30000):
        g = {(x["engine"], x["heater"]): float(x["p50"]) for x in states if x["host"] == key and int(x["E"]) == E}
        if ("cpu", "0") in g and ("gpu-rows", "0") in g:
            cpu, r0, r1 = g[("cpu", "0")], g[("gpu-rows", "0")], g.get(("gpu-rows", "1"), float("nan"))
            win = lambda r: "GPU" if r < cpu else "CPU"
            lines.append(f"{lab} & {E:,} & {cpu:.3f} & {r0:.3f} ({win(r0)}) & {r1:.3f} ({win(r1)}) & {g.get(('gpu-predicate','0'), float('nan')):.3f} \\\\")
    lines.append(r"\midrule")
(T / "crossover.tex").write_text("\n".join(lines[:-1]) + "\n")
print("tables:", sorted(p.name for p in T.glob("*.tex")))


# Prediction scorecard (D1 P1-P4, applied identically to every host)
def get(host, eng, E, metric):
    r = [x for x in eff if x["host"] == host and x["engine"] == eng and int(x["E"]) == E and x["metric"] == metric]
    return (float(r[0]["ratio"]), float(r[0]["lo"]), float(r[0]["hi"])) if r else None


ok = lambda b: "yes" if b else r"\textbf{no}"
lines = []
for key, lab in PH:
    for E in (100, 1000, 3000, 10000, 30000):
        lp, hp = get(key, "gpu-rows", E, "p50"), get(key, "gpu-predicate", E, "p50")
        ls, hs, lm = get(key, "gpu-rows", E, "sel"), get(key, "gpu-predicate", E, "sel"), get(key, "gpu-rows", E, "mat")
        if not (lp and hp and ls and hs):
            continue
        p1 = ok(lp[0] < 0.9 and lp[2] < 1) if E <= 3000 else "n/a"
        p2 = ok(hp[0] >= 0.95)
        p3 = ok(lp[0] < hp[0]) + " / " + ok(ls[0] < hs[0])
        p4 = ok(ls[2] < 1 and lm is not None and 0.9 <= lm[0] <= 1.1)
        lines.append(f"{lab} & {E:,} & {p1} & {p2} & {p3} & {p4} \\\\")
    lines.append(r"\midrule")
(T / "scorecard.tex").write_text("\n".join(lines[:-1]) + "\n")
print("scorecard written")
