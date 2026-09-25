"""Retrospective DVFS audit of existing E0/E2 (RTX 4090) and R1 (RTX 4050) archives.

Read-only over extracted archives; CPU-only. Clocks are nvidia-smi snapshots taken
before/after each engine process (NOT in-run telemetry) -- treat "end clock" as a
coarse proxy for the power state the workload left the GPU in.

usage: python dvfs_retrospective.py [E0E2_ROOT] [R1_ROOT]
"""
import json, os, sys
from pathlib import Path

import numpy as np
import pandas as pd
from scipy import stats
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

TMP = Path(os.environ.get("TEMP", "/tmp")) / "qenlo_audit"
E_ROOT = Path(sys.argv[1]) if len(sys.argv) > 1 else TMP / "e0-e2"
R1_ROOT = Path(sys.argv[2]) if len(sys.argv) > 2 else TMP / "alpha5-router-heldout-rtx4050"
OUT = Path(__file__).resolve().parents[1] / "data" / "processed" / "dvfs-retrospective"
DIM, CORPUS = 384, 100_000
PEAK_GBPS_4090 = 1008  # GDDR6X spec peak; used only as a plausibility bound
GPU = ["gpu-rows", "gpu-predicate", "gpu-mask"]
COLS = ["device_selection_ns", "device_scoring_ns", "backend_execution_ns",
        "row_materialization_ns", "upload_enqueue_ns", "readback_completion_ns"]


def smi(s):
    # "timestamp, name, uuid, 435 MHz, 24, 15.43 W"
    f = [x.strip() for x in (s or "").split(",")]
    if len(f) < 6:
        return np.nan, np.nan, np.nan
    return float(f[3].split()[0]), float(f[4]), float(f[5].split()[0])


def load_blocks():
    rows = []
    for phase in ("e0", "e2"):
        for cell in sorted((E_ROOT / phase).iterdir()):
            b, e = cell.name.split("-")
            for blk in sorted(cell.glob("block-*")):
                for eng in ["cpu"] + GPU:
                    f = blk / eng / "samples.csv"
                    if not f.exists():
                        continue
                    d = pd.read_csv(f, usecols=["batch_latency_ns"] + COLS)
                    rj = blk / f"{eng}.run.json"
                    meta = json.load(open(rj)) if rj.exists() else {}
                    c0, t0, w0 = smi(meta.get("clocks_before"))
                    c1, t1, w1 = smi(meta.get("clocks_after"))
                    lat = d.batch_latency_ns.to_numpy()
                    r = dict(phase=phase, cell=cell.name, batch=int(b[1:]), eligible=int(e[1:]),
                             block=int(blk.name[-2:]), engine=eng, n_samples=len(d),
                             started_unix=meta.get("started_unix"), elapsed_s=meta.get("elapsed_seconds"),
                             clk_before_mhz=c0, clk_end_mhz=c1, temp_end_c=t1, power_end_w=w1,
                             lat_p50_ns=np.percentile(lat, 50, method="inverted_cdf"),
                             lat_p95_ns=np.percentile(lat, 95, method="inverted_cdf"))
                    for c in COLS:
                        r[c.replace("_ns", "_med_ns")] = d[c].median()
                    r["device_total_med_ns"] = (d.device_selection_ns + d.device_scoring_ns).median()
                    rows.append(r)
    return pd.DataFrame(rows)


def between_block(df):
    out = []
    for (ph, cell, eng), g in df[df.engine.isin(GPU)].groupby(["phase", "cell", "engine"]):
        if len(g) < 5:
            continue
        mm = lambda c: g[c].max() / g[c].min()
        sp = lambda a, b: stats.spearmanr(g[a], g[b])
        rs, ps = sp("device_selection_med_ns", "clk_end_mhz")
        rm, pm = sp("row_materialization_med_ns", "device_total_med_ns")
        out.append(dict(phase=ph, cell=cell, engine=eng, n_blocks=len(g),
                        sel_maxmin=mm("device_selection_med_ns"), score_maxmin=mm("device_scoring_med_ns"),
                        mat_maxmin=mm("row_materialization_med_ns"), p50_maxmin=mm("lat_p50_ns"),
                        clk_end_min=g.clk_end_mhz.min(), clk_end_max=g.clk_end_mhz.max(),
                        rho_sel_clk=rs, p_sel_clk=ps, rho_mat_dev=rm, p_mat_dev=pm))
    return pd.DataFrame(out)


def throughput(df):
    g = df[df.engine.isin(GPU) & (df.phase == "e2")].copy()
    rows_scored = np.where(g.engine == "gpu-rows", g.eligible, CORPUS)
    # ponytail: corpus bytes read once per batch; B=16 kernels may re-read per query (upper bound x16)
    g["gbps_assumed"] = rows_scored * DIM * 4 / g.device_scoring_med_ns
    g["gbps_if_eligible_only"] = g.eligible * DIM * 4 / g.device_scoring_med_ns
    t = g.groupby(["batch", "eligible", "engine"]).agg(
        score_us=("device_scoring_med_ns", lambda x: x.median() / 1e3),
        gbps_assumed=("gbps_assumed", "median"), gbps_eligible_only=("gbps_if_eligible_only", "median"),
        clk_end=("clk_end_mhz", "median")).reset_index()
    return t


def counterfactual(df):
    g = df[(df.phase == "e2") & (df.batch == 1)]
    out = []
    for (cell, blk), b in g.groupby(["cell", "block"]):
        rr = b[b.engine == "gpu-rows"]
        for pe in ("gpu-predicate", "gpu-mask"):
            p = b[b.engine == pe]
            if rr.empty or p.empty:
                continue
            r, p1 = rr.iloc[0], p.iloc[0]
            scale = min(r.clk_end_mhz / p1.clk_end_mhz, 1.0)  # only speed rows up, never slow down
            dev = r.device_total_med_ns
            cf = r.lat_p50_ns - dev * (1 - scale)
            out.append(dict(cell=cell, eligible=r.eligible, block=blk, vs=pe,
                            clk_rows=r.clk_end_mhz, clk_other=p1.clk_end_mhz,
                            p50_rows_us=r.lat_p50_ns / 1e3, p50_other_us=p1.lat_p50_ns / 1e3,
                            dev_rows_us=dev / 1e3, gap_us=(r.lat_p50_ns - p1.lat_p50_ns) / 1e3,
                            gap_cf_us=(cf - p1.lat_p50_ns) / 1e3))
    d = pd.DataFrame(out)
    s = d.groupby(["eligible", "vs"])[["clk_rows", "clk_other", "p50_rows_us", "p50_other_us",
                                        "dev_rows_us", "gap_us", "gap_cf_us"]].median().reset_index()
    return d, s


def r1():
    out = []
    for cell in sorted(p for p in R1_ROOT.iterdir() if p.is_dir()):
        for arm in ("gpu-rows", "automatic"):
            f = cell / arm / "samples.csv"
            if not f.exists():
                continue
            d = pd.read_csv(f, usecols=["run", "actual_backend", "device_selection_ns",
                                        "device_scoring_ns", "eligibility_representation"])
            d = d[d.actual_backend == "Wgpu"]
            if d.empty:
                continue
            per = d.groupby("run").device_selection_ns.median()
            x = d.device_selection_ns.to_numpy(float)
            n, sk, ku = len(x), stats.skew(x), stats.kurtosis(x)
            bc = (sk ** 2 + 1) / (ku + 3 * (n - 1) ** 2 / ((n - 2) * (n - 3)))  # >0.555 hints bimodal
            out.append(dict(cell=cell.name, arm=arm, repr=d.eligibility_representation.iloc[0],
                            n=n, run_medians_us=";".join(f"{v/1e3:.1f}" for v in per),
                            run_maxmin=per.max() / per.min(),
                            p10_us=np.percentile(x, 10) / 1e3, p90_us=np.percentile(x, 90) / 1e3,
                            bimodality_coef=bc))
    return pd.DataFrame(out)


def figure(df, path):
    fig, ax = plt.subplots(1, 3, figsize=(16, 5))
    col = {"gpu-rows": "tab:blue", "gpu-predicate": "tab:orange", "gpu-mask": "tab:green"}
    g = df[df.engine.isin(GPU)]
    for eng, s in g.groupby("engine"):
        for ph, mk in (("e2", "o"), ("e0", "x")):
            t = s[s.phase == ph]
            ax[0].scatter(t.clk_end_mhz, t.device_selection_med_ns / 1e3, c=col[eng], marker=mk,
                          s=22, alpha=.75, label=f"{eng} ({ph.upper()})" if len(t) else None)
    ax[0].set(xlabel="end-of-process SM clock snapshot (MHz)", ylabel="median device_selection (us)",
              yscale="log", title="Device selection time vs end clock (per block)")
    ax[0].legend(fontsize=7)
    e0 = df[(df.phase == "e0") & (df.engine == "gpu-rows")].sort_values("block")
    sc = ax[1].scatter(e0.device_total_med_ns / 1e3, e0.row_materialization_med_ns / 1e3,
                       c=e0.clk_end_mhz, cmap="viridis", s=50)
    for _, r in e0.iterrows():
        ax[1].annotate(str(r.block), (r.device_total_med_ns / 1e3, r.row_materialization_med_ns / 1e3), fontsize=8)
    fig.colorbar(sc, ax=ax[1], label="end clock (MHz)")
    ax[1].set(xlabel="median device time sel+score (us)", ylabel="median host row_materialization (us)",
              title="E0 b1-e3000 gpu-rows: host vs device per block")
    ax[2].plot(e0.block, e0.lat_p50_ns / 1e3, "o-", label="batch P50")
    ax[2].plot(e0.block, e0.device_total_med_ns / 1e3, "s-", label="device sel+score")
    ax[2].plot(e0.block, e0.row_materialization_med_ns / 1e3, "^-", label="host materialization")
    c = df[(df.phase == "e0") & (df.engine == "cpu")].sort_values("block")
    ax[2].plot(c.block, c.lat_p50_ns / 1e3, "d--", label="cpu engine P50 (host ref)")
    ax[2].set(xlabel="E0 block", ylabel="us", title="E0 per-block medians")
    ax[2].legend(fontsize=7)
    fig.tight_layout()
    fig.savefig(path, dpi=130)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    df = load_blocks()
    assert (df.n_samples > 0).all() and df.engine.isin(GPU).sum() > 0
    df.to_csv(OUT / "block_device_times.csv", index=False)
    bb = between_block(df); bb.to_csv(OUT / "between_block.csv", index=False)
    tp = throughput(df); tp.to_csv(OUT / "scoring_throughput.csv", index=False)
    cfd, cfs = counterfactual(df)
    cfd.to_csv(OUT / "counterfactual_b1_blocks.csv", index=False); cfs.to_csv(OUT / "counterfactual_b1.csv", index=False)
    rr = r1(); rr.to_csv(OUT / "r1_device_selection_runs.csv", index=False)
    figure(df, OUT / "retro_clock.png")
    pd.set_option("display.width", 250, "display.max_columns", 30, "display.max_rows", 200)
    v = ["phase", "cell", "block", "engine", "clk_before_mhz", "clk_end_mhz", "power_end_w", "lat_p50_ns",
         "device_selection_med_ns", "device_scoring_med_ns", "row_materialization_med_ns"]
    print(df[df.phase == "e0"][v].to_string())
    for t in (bb, tp, cfs, rr):
        print(t.round(3).to_string())


if __name__ == "__main__":
    main()
