"""Light/heavy device time relative to best on that GPU (best over both heater arms), heater off vs on.
Regression-to-the-mean-free: both arms are compared with the same reference. Writes paper/v2/tables/tail.tex."""
import csv, statistics as st
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
R = list(csv.DictReader(open(ROOT / "research/data/processed/dvfs-pooled/dose_response.csv")))
lines = []
for eng, lab in (("gpu-rows", "Light"), ("gpu-predicate", "Heavy")):
    for gpu in ("RTX 4050 Laptop", "RTX 4090 pods", "H100 SXM"):
        x = [r for r in R if r["engine"] == eng and r["gpu"] == gpu]
        cells = []
        for arm in ("sel_off", "sel_on"):
            v = sorted(float(r[arm]) / min(min(float(y["sel_off"]), float(y["sel_on"])) for y in x if y["E"] == r["E"]) for r in x)
            cells += [f"{st.median(v):.2f}", f"{v[int(.9 * (len(v) - 1))]:.2f}", f"{100 * sum(a > 1.5 for a in v) / len(v):.0f}" + r"\%"]
        lines.append(f"{lab} & {gpu} & {len(x)} & " + " & ".join(cells) + " \\\\")
    if eng == "gpu-rows":
        lines.append(r"\midrule")
(ROOT / "paper/v2/tables/tail.tex").write_text("\n".join(lines) + "\n")
print("\n".join(lines))
