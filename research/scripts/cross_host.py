"""Cross-host reproducibility of device kernel time (light vs heavy path), heater-off processes only.

Sources: E2/E0 archive RTX 4090 host A (retrospective CSV), D1R RTX 4090 pods (host B), D1 laptop RTX 4050,
H100 pod. Writes research/data/processed/cross-host/{processes.csv,summary.csv}.
    python research/scripts/cross_host.py D1R_DIR H100_DIR
"""
import csv, statistics as st, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "research/data/processed/cross-host"; OUT.mkdir(parents=True, exist_ok=True)
rows = []
for r in csv.DictReader(open(ROOT / "research/data/processed/dvfs-retrospective/block_device_times.csv")):
    if r["batch"] == "1" and r["engine"] in ("gpu-rows", "gpu-predicate") and r["device_selection_med_ns"]:
        rows.append({"host": "RTX 4090 host A (Linux, Sep 24)", "engine": r["engine"], "E": int(r["eligible"]),
                     "sel_ms": float(r["device_selection_med_ns"]) / 1e6, "score_ms": float(r["device_scoring_med_ns"]) / 1e6,
                     "p50_ms": float(r["lat_p50_ns"]) / 1e6})


def from_raw(label, d):
    for p in Path(d).glob("block-*/*-h0"):
        name = p.name.rsplit("-", 2)
        if name[0] not in ("gpu-rows", "gpu-predicate") or not (p / "bench/summary.txt").exists():
            continue
        S = list(csv.DictReader(open(p / "bench/samples.csv")))
        m = lambda k: st.median(int(s[k] or 0) for s in S) / 1e6
        rows.append({"host": label, "engine": name[0], "E": int(name[1][1:]), "sel_ms": m("device_selection_ns"),
                     "score_ms": m("device_scoring_ns"), "p50_ms": m("batch_latency_ns")})


from_raw("RTX 4050 Laptop (Windows)", ROOT / "research/data/raw/2026-09-25-dvfs-d1")
if len(sys.argv) > 1: from_raw("RTX 4090 host B (Linux pods, Sep 25)", sys.argv[1])
if len(sys.argv) > 2: from_raw("H100 SXM (Linux pod)", sys.argv[2])
with open(OUT / "processes.csv", "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)
summ = []
for host in sorted({r["host"] for r in rows}):
    for eng in ("gpu-rows", "gpu-predicate"):
        for E in sorted({r["E"] for r in rows}):
            v = [r["sel_ms"] for r in rows if r["host"] == host and r["engine"] == eng and r["E"] == E]
            if v:
                summ.append({"host": host, "engine": eng, "E": E, "n": len(v), "sel_med_ms": st.median(v),
                             "sel_min_ms": min(v), "sel_max_ms": max(v), "max_over_min": max(v) / min(v)})
with open(OUT / "summary.csv", "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=list(summ[0])); w.writeheader(); w.writerows(summ)
for s in summ:
    print(f'{s["host"][:22]:22s} {s["engine"]:14s} E={s["E"]:6d} n={s["n"]} sel med {s["sel_med_ms"]:.4f} [{s["sel_min_ms"]:.4f},{s["sel_max_ms"]:.4f}] x{s["max_over_min"]:.2f}')
