"""Share of each call spent inside the two GPU kernels, heater off: (median sel + median score) / median P50.

Source of the Section 5 "heavy 52-84%, light 13-43%" ranges (all E, all three hosts).
    python -I research/scripts/busy_fraction.py
"""
import csv
from pathlib import Path

P = Path(__file__).resolve().parents[1] / "data/processed"
HOSTS = {"laptop": "dvfs-laptop-4050", "pods": "dvfs-pods-4090", "h100": "dvfs-h100"}
share = {"gpu-predicate": [], "gpu-rows": []}
for host, d in HOSTS.items():
    for r in csv.DictReader(open(P / d / "conditions.csv")):
        if r["engine"] in share and r["heater"] == "0":
            s = (float(r["sel"]) + float(r["score"])) / float(r["p50"])
            share[r["engine"]].append(s)
            print(f"{host:6s} {r['engine']:14s} E={int(r['E']):>6} kernels/call = {s:.2f}")
for eng, v in share.items():
    print(f"{eng}: {min(v):.2f}-{max(v):.2f}")
