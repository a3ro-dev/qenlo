"""Figures for paper/v2 from processed D1/D1R outputs and raw clock logs.

    python paper/v2/scripts/make_figures.py
"""
import csv, json, math
from pathlib import Path
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

ROOT = Path(__file__).resolve().parents[3]
FIG = ROOT / "paper/v2/figures"
FIG.mkdir(exist_ok=True)
COL = {"cpu": "#2a78d6", "gpu-rows": "#eb6834", "gpu-predicate": "#1baf7a"}
MARK = {"cpu": "o", "gpu-rows": "s", "gpu-predicate": "^"}
NAME = {"cpu": "CPU scan", "gpu-rows": "GPU compact rows (light)", "gpu-predicate": "GPU shader predicate (heavy)"}
INK, MUTED = "#0b0b0b", "#52514e"
plt.rcParams.update({"font.size": 8, "axes.edgecolor": MUTED, "axes.labelcolor": INK, "xtick.color": MUTED,
                     "ytick.color": MUTED, "axes.spines.top": False, "axes.spines.right": False,
                     "axes.grid": True, "grid.color": "#e6e5e1", "grid.linewidth": 0.6, "lines.linewidth": 1.6,
                     "legend.frameon": False, "pdf.fonttype": 42})


def read(p):
    return list(csv.DictReader(open(p))) if Path(p).exists() else []


def clock_trace(d):
    d = Path(d)
    run = json.load(open(d / "run.json"))
    t, c = [], []
    for line in open(d / "clocks.csv"):
        f = [x.strip() for x in line.split(",")]
        if len(f) > 2 and f[1].isdigit():
            hh, mm, ss = f[0].split(" ")[1].split(":")
            t.append(int(hh) * 3600 + int(mm) * 60 + float(ss)); c.append(int(f[1]))
    t0 = t[0]
    return [x - t0 for x in t], c


def central(raw, proc, out, host_label, block=0, E=3000):
    paired = read(Path(proc) / "paired.csv")
    cond = read(Path(proc) / "conditions.csv")
    fig, ax = plt.subplots(1, 3, figsize=(7.2, 2.35), gridspec_kw={"width_ratios": [1.15, 1, 1]})
    # (a) SM clock timeline, same cell, heater off
    for eng in ["gpu-rows", "gpu-predicate"]:
        d = Path(raw) / f"block-{block}" / f"{eng}-e{E}-h0"
        if d.exists():
            t, c = clock_trace(d)
            ax[0].plot(t, c, color=COL[eng], lw=1.2, label=NAME[eng].split(" (")[0])
    d = Path(raw) / f"block-{block}" / f"gpu-rows-e{E}-h1"
    if d.exists():
        t, c = clock_trace(d)
        ax[0].plot(t, c, color=COL["gpu-rows"], lw=1.0, ls=(0, (2, 1.5)), label="compact rows + heater")
    ax[0].set(xlabel="time in process (s)", ylabel="GPU SM clock (MHz)", title=f"(a) clock during one cell, E={E:,}")
    ax[0].legend(loc="lower right", fontsize=6.5)
    # (b) paired heater effect
    for i, eng in enumerate(["cpu", "gpu-rows", "gpu-predicate"]):
        r = [x for x in paired if x["comparison"] == "heater_on/off" and x["engine"] == eng and x["metric"] == "p50"]
        r.sort(key=lambda x: int(x["E"]))
        if not r:
            continue
        E_ = [int(x["E"]) * (1 + 0.06 * (i - 1)) for x in r]
        y = [float(x["ratio"]) for x in r]
        lo = [y_ - float(x["lo"]) for y_, x in zip(y, r)]
        hi = [float(x["hi"]) - y_ for y_, x in zip(y, r)]
        ax[1].errorbar(E_, y, yerr=[lo, hi], color=COL[eng], marker=MARK[eng], ms=4, capsize=2, lw=1.2,
                       label=NAME[eng].split(" (")[0])
    ax[1].axhline(1, color=MUTED, lw=0.8)
    ax[1].set(xscale="log", xlabel="eligible rows E", ylabel="P50 latency, heater on / off",
              title="(b) busier GPU: effect by path")
    ax[1].legend(fontsize=6.5, loc="lower right")
    # (c) crossover shift
    for eng in ["cpu", "gpu-rows"]:
        for h, ls, mf in [(0, "-", "white"), (1, "--", None)]:
            r = sorted([x for x in cond if x["engine"] == eng and int(x["heater"]) == h], key=lambda x: int(x["E"]))
            if not r or (eng == "cpu" and h == 1):
                continue
            ax[2].plot([int(x["E"]) for x in r], [float(x["p50"]) for x in r], ls=ls, color=COL[eng], marker=MARK[eng],
                       ms=4, mfc=mf or COL[eng], label=("CPU scan" if eng == "cpu" else f"compact rows, heater {'on' if h else 'off'}"))
    ax[2].set(xscale="log", yscale="log", xlabel="eligible rows E", ylabel="median-block P50 (ms)",
              title="(c) CPU/GPU crossover moves")
    ax[2].legend(fontsize=6.5, loc="upper left")
    fig.suptitle(host_label, fontsize=7, color=MUTED, y=0.02, va="bottom")
    fig.tight_layout(rect=(0, 0.04, 1, 1))
    fig.savefig(FIG / f"{out}.pdf"); fig.savefig(FIG / f"{out}.png", dpi=200)
    plt.close(fig)


if __name__ == "__main__":
    central(ROOT / "research/data/raw/2026-09-25-dvfs-d1", ROOT / "research/data/processed/dvfs-d1", "fig1_local",
            "RTX 4050 Laptop, Windows, Vulkan; AG News 100k x 384, B=1, k=10; 5 blocks, 95% block-bootstrap intervals")
    if (ROOT / "research/data/processed/dvfs-d1r/paired.csv").exists():
        central(ROOT / "research/data/raw/2026-09-25-dvfs-d1r", ROOT / "research/data/processed/dvfs-d1r", "fig1_cloud",
                "RTX 4090, Linux, Vulkan (one pod per block); same workload")
    print("figures written to", FIG)
