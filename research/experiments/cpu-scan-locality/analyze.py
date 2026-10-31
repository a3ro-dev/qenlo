"""Apply the cpu-scan-locality gate: median of three interleaved runs per (E, mode) cell.

python research/experiments/cpu-scan-locality/analyze.py <runs_dir> <base> <candidate> [floor_ms]
"""
import csv, statistics, sys
from pathlib import Path

runs, base, cand = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
# Amendment 1: a regression must also exceed this absolute floor (default 0 = original gate).
floor = float(sys.argv[4]) if len(sys.argv) > 4 else 0.0


def load(name):
    cells = {}
    for f in sorted(runs.glob(f"{name}_*.csv")):
        for row in csv.DictReader(f.open()):
            cell = cells.setdefault((int(row["E"]), row["mode"]), {"m": [], "p": []})
            cell["m"].append(float(row["median_ms"]))
            cell["p"].append(float(row["p95_ms"]))
    return {k: (statistics.median(v["m"]), statistics.median(v["p"])) for k, v in cells.items()}


b, c = load(base), load(cand)
print("E,mode,base_med,cand_med,med_ratio,base_p95,cand_p95,p95_ratio")
wins, regressions = 0, []
for key in sorted(b):
    (bm, bp), (cm, cp) = b[key], c[key]
    print(f"{key[0]},{key[1]},{bm:.3f},{cm:.3f},{cm/bm:.3f},{bp:.3f},{cp:.3f},{cp/bp:.3f}")
    if key[1] == "Certified" and key[0] in (10000, 20000, 30000) and cm <= 0.95 * bm and cp <= 0.95 * bp:
        wins += 1
    if key[1] in ("Certified", "Auto") and cm > 1.05 * bm + floor:
        regressions.append(key)
verdict = "KEEP" if wins >= 2 and not regressions else "REJECT"
print(f"# gate: {wins}/3 scattered Certified wins, regressions {regressions} -> {verdict}")
