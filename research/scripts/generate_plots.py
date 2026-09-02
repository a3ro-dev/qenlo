#!/usr/bin/env python3
"""Generate publication figures from processed benchmark tables."""

from pathlib import Path

import matplotlib as mpl
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd


ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "research/data/processed"
FIG = ROOT / "paper/figures"

BLUE = "#176B87"
ORANGE = "#D95F02"
GREEN = "#4C956C"
PURPLE = "#7B61A8"
GREY = "#667085"
RED = "#B42318"


def style() -> None:
    mpl.rcParams.update({
        "font.family": "DejaVu Sans",
        "font.size": 8.5,
        "axes.titlesize": 9.5,
        "axes.labelsize": 8.5,
        "legend.fontsize": 7.5,
        "xtick.labelsize": 7.5,
        "ytick.labelsize": 7.5,
        "axes.spines.top": False,
        "axes.spines.right": False,
        "axes.grid": True,
        "axes.grid.axis": "y",
        "grid.color": "#D9DEE7",
        "grid.linewidth": 0.55,
        "figure.dpi": 180,
        "savefig.dpi": 300,
        "pdf.fonttype": 42,
    })


def save(fig: plt.Figure, name: str) -> None:
    FIG.mkdir(parents=True, exist_ok=True)
    fig.savefig(FIG / f"{name}.pdf", bbox_inches="tight", pad_inches=0.03)
    fig.savefig(FIG / f"{name}.png", bbox_inches="tight", pad_inches=0.03)
    plt.close(fig)


def phase_map() -> None:
    cross = pd.read_csv(DATA / "native_crossover_summary.csv")
    win = pd.read_csv(DATA / "windows_summary.csv")
    a6 = pd.read_csv(DATA / "a6000_exact_summary.csv")

    cpu_end = win[(win.system.str.startswith("CPU")) & (win.eligible.isin([1_000, 100_000]))]
    gpu_end = win[(win.system == "WGPU compact rows") & (win.eligible.isin([1_000, 100_000]))]
    native_cpu = pd.concat([
        cpu_end[["eligible", "p95_ms"]],
        cross[["eligible", "cpu_p95_ms"]].rename(columns={"cpu_p95_ms": "p95_ms"}),
    ]).sort_values("eligible")
    native_gpu = pd.concat([
        gpu_end[["eligible", "p95_ms"]],
        cross[["eligible", "gpu_rows_p95_ms"]].rename(columns={"gpu_rows_p95_ms": "p95_ms"}),
    ]).sort_values("eligible")

    fig, axes = plt.subplots(1, 2, figsize=(7.05, 2.55))
    ax = axes[0]
    ax.plot(native_cpu.eligible, native_cpu.p95_ms, "o-", color=ORANGE, lw=1.8, ms=4, label="CPU exhaustive")
    ax.plot(native_gpu.eligible, native_gpu.p95_ms, "s-", color=BLUE, lw=1.8, ms=4, label="WGPU compact rows")
    ax.axvspan(2_000, 3_000, color="#F4C95D", alpha=.28, lw=0)
    ax.text(2_450, 8.0, "observed\nreversal band", ha="center", va="center", fontsize=8.2, color="#765A00")
    ax.set(xscale="log", yscale="log", xlabel="eligible vectors, E", ylabel="P95 completed-call latency (ms)",
           title="(a) shipped CPU/WGPU, RTX 4050, D=384")
    ax.legend(loc="lower right", frameon=False)
    ax.grid(True, which="both", axis="both", alpha=.7)

    ax = axes[1]
    labels = {
        "faiss_gpu_flat_ip": ("FAISS GPU Flat", ORANGE, "o"),
        "qenlo_cuda_buffered_prototype": ("research CUDA prototype", BLUE, "s"),
    }
    for system, (label, color, marker) in labels.items():
        subset = a6[a6.system == system].sort_values("eligible")
        ax.plot(subset.eligible, subset.p95_ms, marker=marker, color=color, lw=1.8, ms=4, label=label)
    ax.axvspan(10_000, 100_000, color="#F4C95D", alpha=.28, lw=0)
    ax.text(31_600, .42, "observed\nreversal band", ha="center", va="center", fontsize=8.2, color="#765A00")
    ax.set(xscale="log", yscale="log", xlabel="eligible vectors, E", title="(b) exploratory CUDA/FAISS, A6000, D=768")
    ax.legend(loc="upper left", frameon=False)
    ax.grid(True, which="both", axis="both", alpha=.7)
    fig.suptitle("Two implementation-specific exhaustive-search reversals", y=1.03, fontsize=10.5)
    fig.tight_layout(w_pad=1.2)
    save(fig, "phase_map")


def windows_strategies() -> None:
    df = pd.read_csv(DATA / "windows_summary.csv")
    fig, axes = plt.subplots(1, 2, figsize=(7.05, 2.65), sharey=True)
    panels = [
        (1_000, ["CPU exhaustive FP64 accum.", "WGPU compact rows", "WGPU predicate"], "(a) E=1,000 (1% eligible)"),
        (100_000, ["CPU exhaustive FP64 accum.", "WGPU compact rows", "WGPU mask", "WGPU predicate", "USearch HNSW ef=128"], "(b) E=100,000 (all rows)"),
    ]
    short = {
        "CPU exhaustive FP64 accum.": "CPU",
        "WGPU compact rows": "WGPU\nrows",
        "WGPU mask": "WGPU\nmask",
        "WGPU predicate": "WGPU\npredicate",
        "USearch HNSW ef=128": "USearch\nef=128",
    }
    for ax, (eligible, systems, title) in zip(axes, panels):
        sub = df[(df.eligible == eligible) & (df.system.isin(systems))].set_index("system").loc[systems].reset_index()
        colors = [ORANGE if name.startswith("CPU") else PURPLE if name.startswith("USearch") else BLUE for name in systems]
        bars = ax.bar(range(len(sub)), sub.p95_ms, color=colors, width=.66)
        ax.set_xticks(range(len(sub)), [short[name] for name in systems])
        ax.set_yscale("log")
        ax.set_title(title)
        for bar, latency, recall in zip(bars, sub.p95_ms, sub.recall_at_10):
            ax.text(bar.get_x() + bar.get_width()/2, latency*1.12, f"{latency:.3g} ms\nR={recall:.5f}",
                    ha="center", va="bottom", fontsize=6.4)
        ax.set_ylim(.14, 32)
    axes[0].set_ylabel("P95 completed-call latency (ms, log scale)")
    fig.suptitle("Matched Windows strategies; 25,000 held-out calls per bar", y=1.02, fontsize=10.5)
    fig.tight_layout(w_pad=1.0)
    save(fig, "windows_strategies")


def android_matched() -> None:
    df = pd.read_csv(DATA / "android_device_lab.csv")
    fig, axes = plt.subplots(1, 2, figsize=(7.05, 2.7))

    ax = axes[0]
    suites = ["quick", "full", "soak"]
    cpu = [df[(df.suite == s) & (df.cell == "cpu-exact-all")].p95_ms.iloc[0] for s in suites]
    gpu = [df[(df.suite == s) & (df.cell == "gpu-exact-all")].p95_ms.iloc[0] for s in suites]
    x = np.arange(3)
    width = .34
    ax.bar(x-width/2, cpu, width, color=ORANGE, label="CPU exhaustive")
    ax.bar(x+width/2, gpu, width, color=BLUE, label="WGPU exhaustive")
    ax.set_xticks(x, ["quick\n10k rows, n=16", "full\n100k rows, n=64", "soak\n100k rows, n=512"])
    ax.set(ylabel="reported P95 latency (ms)", title="(a) dense batch-1 route")
    ax.legend(frameon=False, loc="upper left")

    ax = axes[1]
    soak = df[(df.suite == "soak") & (df.cell.isin([
        "cpu-exact-all", "gpu-exact-all", "gpu-ivf-flat-recall-95", "gpu-ivf-sq8-recall-95"
    ]))]
    order = ["cpu-exact-all", "gpu-exact-all", "gpu-ivf-flat-recall-95", "gpu-ivf-sq8-recall-95"]
    soak = soak.set_index("cell").loc[order]
    bars = ax.bar(range(4), soak.p95_ms, color=[ORANGE, BLUE, GREEN, PURPLE], width=.68)
    ax.set_xticks(range(4), ["CPU\nexhaustive", "WGPU\nexhaustive", "IVF-Flat", "IVF-SQ8"])
    ax.set_title("(b) matched 100k-row soak, batch 1, n=512")
    for bar, latency in zip(bars, soak.p95_ms):
        ax.text(bar.get_x()+bar.get_width()/2, latency+.8, f"{latency:.1f}", ha="center", fontsize=7)
    ax.set_ylim(0, 43)
    fig.suptitle("Android 16 / Mali-G615 device-lab evidence; observed recall@10 = 1.0", y=1.02, fontsize=10.2)
    fig.tight_layout(w_pad=1.0)
    save(fig, "android_matched")


def linux_context() -> None:
    df = pd.read_csv(DATA / "linux_library_summary.csv")
    fig, ax = plt.subplots(figsize=(5.3, 2.85))
    markers = {"exhaustive": "o", "ANN": "s"}
    colors = {"exhaustive": BLUE, "ANN": PURPLE}
    for kind in ["exhaustive", "ANN"]:
        sub = df[df.search_type == kind]
        ax.scatter(sub.recall_at_10, sub.p95_ms, s=38, marker=markers[kind], color=colors[kind], label=kind, zorder=3)
        for _, row in sub.iterrows():
            dx, dy = (3, 3)
            if row.system == "FAISS HNSW":
                dy = 5
            if row.system == "LanceDB Flat":
                dy = -2
            ax.annotate(row.system, (row.recall_at_10, row.p95_ms), xytext=(dx, dy), textcoords="offset points", fontsize=6.8)
    ax.set_yscale("log")
    ax.set_xlim(.9908, 1.00065)
    ax.set_ylim(1.05, 360)
    ax.set(xlabel="recall@10", ylabel="P95 latency (ms, log scale)",
           title="Descriptive Linux library cohort: same data and host, 1,500 calls/system")
    ax.legend(frameon=False, loc="upper left")
    ax.grid(True, which="both", axis="both", alpha=.7)
    save(fig, "linux_library_context")


def architecture() -> None:
    fig, ax = plt.subplots(figsize=(7.05, 2.15))
    ax.axis("off")
    boxes = [
        (.02, .29, .23, .46, "canonical CoreStore\nFP32 vectors + metadata\ntombstones + generation", BLUE, "#EAF4F7"),
        (.36, .68, .25, .22, "CPU exhaustive", ORANGE, "#FFF0E6"),
        (.36, .39, .25, .22, "filtered HNSW", PURPLE, "#F2EDF8"),
        (.36, .10, .25, .22, "WGPU exhaustive / IVF", BLUE, "#EAF4F7"),
        (.73, .29, .25, .46, "observable router\nrequested vs executed route\nfallback + E + transfers", GREEN, "#ECF6F0"),
    ]
    for x, y, w, h, text, edge, face in boxes:
        ax.add_patch(mpl.patches.FancyBboxPatch((x, y), w, h, boxstyle="round,pad=.012", ec=edge, fc=face, lw=1.3))
        ax.text(x+w/2, y+h/2, text, ha="center", va="center", transform=ax.transAxes, fontsize=8)
    for y0, y1 in [(.60, .79), (.52, .50), (.44, .21)]:
        ax.annotate("", xy=(.36, y1), xytext=(.25, y0), xycoords="axes fraction",
                    arrowprops=dict(arrowstyle="->", color=GREY, lw=1.1))
    for y0, y1 in [(.79, .60), (.50, .52), (.21, .44)]:
        ax.annotate("", xy=(.73, y1), xytext=(.61, y0), xycoords="axes fraction",
                    arrowprops=dict(arrowstyle="->", color=GREY, lw=1.1))
    ax.text(.5, .01, "logical truth remains canonical; every search structure is rebuildable derived state",
            ha="center", va="bottom", transform=ax.transAxes, fontsize=7.2, color=GREY)
    save(fig, "architecture")


def main() -> None:
    style()
    phase_map()
    windows_strategies()
    android_matched()
    linux_context()
    architecture()
    print("generated phase_map, windows_strategies, android_matched, linux_library_context, architecture")


if __name__ == "__main__":
    main()
