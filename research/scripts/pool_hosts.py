"""Pool D1 (laptop), D1R (RTX 4090 pods) and H100 into one per-host summary for the paper.

    python research/scripts/pool_hosts.py LAPTOP_RAW D1R_RAW H100_RAW
Writes research/data/processed/dvfs-pooled/{host_effects.csv,host_states.csv}. Each host is analyzed with
analyze_dvfs_d1.py first (its outputs are read from research/data/processed/dvfs-<tag>/).
"""
import csv, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "research/data/processed/dvfs-pooled"; OUT.mkdir(parents=True, exist_ok=True)
HOSTS = [("laptop-4050", "RTX 4050 Laptop, Windows"), ("pods-4090", "RTX 4090 pods, Linux"), ("h100", "H100 SXM, Linux")]
effects, states = [], []
for (tag, label), raw in zip(HOSTS, sys.argv[1:4]):
    proc = ROOT / f"research/data/processed/dvfs-{tag}"
    subprocess.run([sys.executable, str(ROOT / "research/scripts/analyze_dvfs_d1.py"), raw, str(proc)], check=True,
                   stdout=subprocess.DEVNULL)
    for r in csv.DictReader(open(proc / "paired.csv")):
        if r["comparison"] == "heater_on/off" and r["engine"] != "cpu" and r["metric"] in ("p50", "p95", "sel", "score", "mat"):
            effects.append({"host": label, **{k: r[k] for k in ("engine", "E", "metric", "n", "ratio", "lo", "hi")}})
    for r in csv.DictReader(open(proc / "conditions.csv")):
        states.append({"host": label, **r})
for name, rows in (("host_effects.csv", effects), ("host_states.csv", states)):
    with open(OUT / name, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)
print("wrote", OUT)
