"""Central figure: (a) cross-host reproducibility of device kernel time, light vs heavy path;
(b) in-process SM clock for the same cell on the laptop; (c) heater effect on device time by host and path.

    python paper/v2/scripts/fig_central.py
"""
import csv, json
from pathlib import Path
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

ROOT = Path(__file__).resolve().parents[3]
FIG = ROOT / "paper/v2/figures"; FIG.mkdir(exist_ok=True)
LIGHT, HEAVY, INK, MUTED = "#eb6834", "#1baf7a", "#0b0b0b", "#52514e"
plt.rcParams.update({"font.size": 7.5, "axes.edgecolor": MUTED, "axes.labelcolor": INK, "xtick.color": MUTED,
                     "ytick.color": MUTED, "axes.spines.top": False, "axes.spines.right": False, "axes.grid": True,
                     "grid.color": "#e6e5e1", "grid.linewidth": 0.6, "legend.frameon": False, "pdf.fonttype": 42})
HOSTS = ["RTX 4050 Laptop (Windows)", "RTX 4090 host A (Linux, Sep 24)", "RTX 4090 host B (Linux pods, Sep 25)", "H100 SXM (Linux pod)"]
SHORT = {HOSTS[0]: "4050\nlaptop", HOSTS[1]: "4090\nhost A", HOSTS[2]: "4090\nhost B", HOSTS[3]: "H100"}
E_SHOW = 3000


def clock(d):
    t, c = [], []
    for line in open(Path(d) / "clocks.csv"):
        f = [x.strip() for x in line.split(",")]
        if len(f) > 4 and f[1].isdigit() and f[4].isdigit():
            h, m, s = f[0].split(" ")[1].split(":"); t.append(int(h) * 3600 + int(m) * 60 + float(s)); c.append((int(f[1]), int(f[4])))
    t0 = t[0]
    return [x - t0 for x in t], [x[0] for x in c]


fig, ax = plt.subplots(1, 3, figsize=(7.2, 2.9), gridspec_kw={"width_ratios": [1.25, 1.0, 1.1]})
# (a)
proc = list(csv.DictReader(open(ROOT / "research/data/processed/cross-host/processes.csv")))
for i, h in enumerate(HOSTS):
    for eng, col, dx, mk in (("gpu-predicate", HEAVY, -0.12, "^"), ("gpu-rows", LIGHT, 0.12, "s")):
        v = [float(r["sel_ms"]) for r in proc if r["host"] == h and r["engine"] == eng and int(r["E"]) == E_SHOW]
        if v:
            ax[0].scatter([i + dx] * len(v), v, s=16, marker=mk, color=col, edgecolor="white", linewidth=0.5, zorder=3,
                          label=None if i else ("heavy path (scans all N)" if eng == "gpu-predicate" else "light path (scans E rows)"))
ax[0].set_yscale("log"); ax[0].set_xticks(range(len(HOSTS)), [SHORT[h] for h in HOSTS])
ax[0].set_ylabel("device top-k time per query (ms)")
ax[0].set_title("(a) one query cell, four GPUs", loc="left")
ax[0].legend(loc="upper center", bbox_to_anchor=(0.5, -0.28), fontsize=6.5, ncol=1)
# (b)
base = ROOT / "research/data/raw/2026-09-25-dvfs-d1"
for eng, col, lab in (("gpu-predicate", HEAVY, "heavy"), ("gpu-rows", LIGHT, "light")):
    for b in range(5):
        d = base / f"block-{b}" / f"{eng}-e10000-h0"
        if (d / "run.json").exists():
            t, c = clock(d); ax[1].plot(t, c, color=col, lw=1.0, label=lab); break
d = next((base / f"block-{b}" / "gpu-rows-e10000-h1" for b in range(5) if (base / f"block-{b}" / "gpu-rows-e10000-h1" / "run.json").exists()), None)
if d:
    t, c = clock(d); ax[1].plot(t, c, color=LIGHT, lw=0.9, ls=(0, (2, 1.4)), label="light + heater")
ax[1].set(xlabel="seconds since process start", ylabel="SM clock (MHz)")
ax[1].set_title("(b) clock the driver picks", loc="left")
ax[1].legend(fontsize=6.5, loc="upper center", bbox_to_anchor=(0.5, -0.28), ncol=3)
# (c) heater collapses the light kernel's slow tail (device time / best on that GPU)
dr = list(csv.DictReader(open(ROOT / "research/data/processed/dvfs-pooled/dose_response.csv")))
GPUS = [("RTX 4050 Laptop", "4050" + chr(10) + "laptop"), ("RTX 4090 pods", "4090" + chr(10) + "pods"), ("H100 SXM", "H100")]
for i, (g, _) in enumerate(GPUS):
    for eng, col, base, mk in (("gpu-rows", LIGHT, 0.0, "s"), ("gpu-predicate", HEAVY, 0.0, "^")):
        x = [r for r in dr if r["gpu"] == g and r["engine"] == eng]
        for arm, dx, fill in (("sel_off", -0.17, "white"), ("sel_on", 0.17, col)):
            v = []
            for r in x:
                best = min(min(float(y["sel_off"]), float(y["sel_on"])) for y in x if y["E"] == r["E"])
                v.append(float(r[arm]) / best)
            off = -0.06 if eng == "gpu-predicate" else 0.06
            ax[2].scatter([i + dx + off] * len(v), v, s=11, marker=mk, facecolor=fill, edgecolor=col, linewidth=0.8, zorder=3)
ax[2].set_yscale("log"); ax[2].set_xticks(range(3), [g[1] for g in GPUS])
ax[2].set_ylabel("device time / best on that GPU")
ax[2].set_title("(c) add GPU load", loc="left")
ax[2].set_yticks([1, 1.5, 2, 3, 4], ["1", "1.5", "2", "3", "4"]); ax[2].minorticks_off()
ax[2].text(0.5, -0.30, "hollow = heater off, filled = heater on" + chr(10) + "square = light, triangle = heavy", transform=ax[2].transAxes, ha="center", va="top", fontsize=6.5, color=MUTED)
fig.tight_layout()
fig.savefig(FIG / "fig1_central.pdf"); fig.savefig(FIG / "fig1_central.png", dpi=220)
print("ok")
