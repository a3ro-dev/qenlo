"""Independent recomputation of Qenlo paper Tables 1-5, 7, 8 and in-text numbers directly from raw samples.

Written for the red-team audit; does NOT import or call any of the author's scripts.
Run with:  python -I recompute_tables.py <EXTRACT_ROOT> <REPO_ROOT>
where EXTRACT_ROOT contains:
  d1/2026-09-25-dvfs-d1/block-*/...                  (from research/data/raw/2026-09-25-dvfs-d1.tar.gz.part*)
  d1r/{b0,b1,b2,b3r,b4}/2026-09-25-dvfs-d1r/block-*  (from research/data/raw/2026-09-25-dvfs-d1r/*.tar.gz)
  d1r/b3-partial/...                                 (excluded slow pod)
  h100/2026-09-25-dvfs-h100/block-*
  e0e2/e0-e2/{e0,e2}/b<B>-e<E>/block-NN/<engine>/samples.csv  (host A archive, runpod-e0-e2-partial-20260924)
and REPO_ROOT is the clone (for research/data/raw/2026-10-19-dvfs-lock and 2026-10-25-dvfs-split).

Conventions chosen independently and stated explicitly:
  * per-process device time = median over all 15,000 timed calls of device_selection_ns (also scoring);
  * per-process end-to-end P50 = median of the three per-pass p50_batch_ns in runs.csv (the bench's own
    nearest-rank P50); with three passes the lower median equals the ordinary median;
  * clocks: nvidia-smi rows with utilization > 0 ("GPU-active"); both median-of-per-process-medians and
    pooled-sample medians are reported;
  * paired effects: exp(median over blocks of ln(on/off)); 95% percentile bootstrap over blocks with
    numpy default_rng(12345), 20,000 draws, np.percentile(..., [2.5, 97.5]) (method 'linear').
"""
import sys, os, glob, json, math, itertools
import numpy as np
import pandas as pd
from scipy.stats import spearmanr

EX, REPO = sys.argv[1], sys.argv[2]
RAW = os.path.join(REPO, "research/data/raw")
USE = ["device_selection_ns", "device_scoring_ns", "batch_latency_ns", "row_materialization_ns", "recall_at_10"]


def samples(d):
    p = os.path.join(d, "bench", "samples.csv") if os.path.isdir(os.path.join(d, "bench")) else os.path.join(d, "samples.csv")
    s = pd.read_csv(p, usecols=lambda c: c in USE)
    return s


def p50_runs(d):
    p = os.path.join(d, "bench", "runs.csv") if os.path.isdir(os.path.join(d, "bench")) else os.path.join(d, "runs.csv")
    r = pd.read_csv(p)
    return float(np.median(r["p50_batch_ns"])) / 1e6


def clocks(d):
    rows = []
    for line in open(os.path.join(d, "clocks.csv")):
        f = [x.strip() for x in line.split(",")]
        if len(f) > 4 and f[1].isdigit() and f[2].isdigit() and f[4].isdigit():
            rows.append((int(f[1]), int(f[2]), int(f[4])))
    a = np.array(rows) if rows else np.zeros((0, 3))
    act = a[a[:, 2] > 0] if len(a) else a
    return act


def proc(d):
    s = samples(d)
    out = dict(sel=s.device_selection_ns.median() / 1e6, score=s.device_scoring_ns.median() / 1e6,
               mat=s.row_materialization_ns.median() / 1e6 if "row_materialization_ns" in s else np.nan,
               call_med=s.batch_latency_ns.median() / 1e6, p50=p50_runs(d), min_recall=s.recall_at_10.min(),
               n=len(s))
    if os.path.exists(os.path.join(d, "clocks.csv")):
        act = clocks(d)
        out["sm_act"] = float(np.median(act[:, 0])) if len(act) else np.nan
        out["mem_act"] = float(np.median(act[:, 1])) if len(act) else np.nan
        out["_act"] = act
    # kernel share of call: median over calls of (score+sel)/latency
    out["kshare"] = float(((s.device_selection_ns + s.device_scoring_ns) / s.batch_latency_ns).median())
    return out


def load_dvfs(root_blocks, label):
    recs = []
    for d in sorted(glob.glob(os.path.join(root_blocks, "block-*", "*-h[01]"))):
        name = os.path.basename(d)
        eng, e, h = name.rsplit("-", 2)
        j = json.load(open(os.path.join(d, "run.json")))
        if j["returncode"] != 0:
            continue
        r = proc(d)
        r.update(host=label, block=int(os.path.basename(os.path.dirname(d)).split("-")[1]), engine=eng, E=int(e[1:]), h=int(h[1:]), dir=d)
        recs.append(r)
    return recs


def fmt(x, n=3):
    return f"{x:.{n}f}"


# ---------------- load Sep-25 data
D1 = load_dvfs(os.path.join(EX, "d1/2026-09-25-dvfs-d1"), "laptop")
POD = []
for b in ["b0", "b1", "b2", "b3r", "b4"]:
    POD += load_dvfs(os.path.join(EX, f"d1r/{b}/2026-09-25-dvfs-d1r"), "pods")
H100 = load_dvfs(os.path.join(EX, "h100/2026-09-25-dvfs-h100"), "h100")
ALL = pd.DataFrame([{k: v for k, v in r.items() if k != "_act"} for r in D1 + POD + H100])
ACT = {(r["host"], r["block"], r["engine"], r["E"], r["h"]): r.get("_act") for r in D1 + POD + H100}
print("loaded processes:", ALL.groupby("host").size().to_dict())
print("min per-call recall@10 across all Sep-25 processes:", ALL.min_recall.min())

# ---------------- host A (Sep 24 archive): batch 1 cells, e0 and e2
HA = []
for ph in ("e0", "e2"):
    for cell in sorted(glob.glob(os.path.join(EX, "e0e2/e0-e2", ph, "b1-e*"))):
        E = int(os.path.basename(cell).split("-e")[1])
        for blk in sorted(glob.glob(os.path.join(cell, "block-*"))):
            for eng in ("gpu-rows", "gpu-predicate"):
                d = os.path.join(blk, eng)
                if not os.path.exists(os.path.join(d, "samples.csv")):
                    continue
                s = samples(d)
                meta = json.load(open(os.path.join(blk, f"{eng}.run.json")))
                ca = meta.get("clocks_after") or ""
                try:
                    clk_end = float(ca.split(",")[3].split()[0])
                except Exception:
                    clk_end = np.nan
                HA.append(dict(phase=ph, E=E, block=int(blk[-2:]), engine=eng, sel=s.device_selection_ns.median() / 1e6, clk_end=clk_end))
HA = pd.DataFrame(HA)

# ---------------- Table 1
print("\n=== TABLE 1 (heater-off device top-k ms): median (min-max) max/min ; n")
def t1row(v):
    v = np.asarray(v)
    return f"{np.median(v):.3f} ({v.min():.3f}-{v.max():.3f}) x{v.max() / v.min():.2f} n={len(v)}"
for E in (1000, 3000, 10000):
    for lab, df in (("laptop", ALL[ALL.host == "laptop"]), ("pods", ALL[ALL.host == "pods"]), ("h100", ALL[ALL.host == "h100"])):
        h = df[(df.engine == "gpu-predicate") & (df.E == E) & (df.h == 0)].sel
        l = df[(df.engine == "gpu-rows") & (df.E == E) & (df.h == 0)].sel
        print(f"E={E:>6} {lab:7s} heavy {t1row(h)} | light {t1row(l)}")
    for phases, tag in ((("e2",), "hostA e2 only"), (("e0", "e2"), "hostA e0+e2")):
        x = HA[(HA.E == E) & HA.phase.isin(phases)]
        h, l = x[x.engine == "gpu-predicate"].sel, x[x.engine == "gpu-rows"].sel
        if len(h) and len(l):
            print(f"E={E:>6} {tag:13s} heavy {t1row(h)} | light {t1row(l)}")
# light spread at all E (contribution claims 4.2x laptop, 4.3-4.6x pods)
print("light max/min heater-off, all E:")
for host in ("laptop", "pods", "h100"):
    x = ALL[(ALL.host == host) & (ALL.engine == "gpu-rows") & (ALL.h == 0)].groupby("E").sel.agg(lambda v: v.max() / v.min())
    y = ALL[(ALL.host == host) & (ALL.engine == "gpu-predicate") & (ALL.h == 0)].groupby("E").sel.agg(lambda v: v.max() / v.min())
    print(f"  {host}: light {x.round(2).to_dict()}  heavy {y.round(3).to_dict()}")
x = HA[(HA.phase == "e2")]
print("  hostA e2 light", x[x.engine == "gpu-rows"].groupby("E").sel.agg(lambda v: round(v.max() / v.min(), 2)).to_dict(),
      "heavy", x[x.engine == "gpu-predicate"].groupby("E").sel.agg(lambda v: round(v.max() / v.min(), 4)).to_dict())
# pooled 4090 (host A e2 + pods) at E=10000
v = list(HA[(HA.phase == "e2") & (HA.engine == "gpu-rows") & (HA.E == 10000)].sel) + list(ALL[(ALL.host == "pods") & (ALL.engine == "gpu-rows") & (ALL.E == 10000) & (ALL.h == 0)].sel)
print(f"  all RTX4090 light E=10000 heater-off: min {min(v):.4f} max {max(v):.4f} span x{max(v) / min(v):.2f}")

# ---------------- Section 4: host A E0 Spearman and 2.44x span
e0 = HA[(HA.phase == "e0") & (HA.engine == "gpu-rows")].sort_values("block")
rho, p = spearmanr(e0.sel, e0.clk_end)
print(f"\n=== HOST A E0 b1-e3000 light: n={len(e0)} span x{e0.sel.max() / e0.sel.min():.3f} Spearman rho={rho:.3f} p={p:.4f}")
print("  per-block sel ms:", e0.sel.round(4).tolist(), " end clocks:", e0.clk_end.tolist())
hv = HA[(HA.phase == "e2") & (HA.engine == "gpu-predicate")]
print("  host A heavy max/min by E (e2):", hv.groupby("E").sel.agg(lambda v: round(v.max() / v.min(), 4)).to_dict())

# ---------------- Table 2: clocks while GPU-active
print("\n=== TABLE 2 GPU-active median SM/mem MHz: [median of per-process medians] / [pooled samples]")
for host in ("laptop", "pods", "h100"):
    for E in (1000, 3000, 10000):
        cells = []
        for eng, h in (("gpu-predicate", 0), ("gpu-rows", 0), ("gpu-rows", 1)):
            sub = ALL[(ALL.host == host) & (ALL.engine == eng) & (ALL.E == E) & (ALL.h == h)]
            mm = f"{sub.sm_act.median():.0f}/{sub.mem_act.median():.0f}"
            pool = np.vstack([ACT[(host, b, eng, E, h)] for b in sub.block if ACT.get((host, b, eng, E, h)) is not None and len(ACT[(host, b, eng, E, h)])])
            pl = f"{np.median(pool[:, 0]):.0f}/{np.median(pool[:, 1]):.0f}"
            cells.append(f"{mm} [{pl}]")
        print(f"  {host:6s} E={E:>6} heavy {cells[0]:22s} light {cells[1]:22s} light+heater {cells[2]}")

# ---------------- Table 3: relative to best
print("\n=== TABLE 3 device time / best for that GPU & E (best over both heater arms)")
for eng, lab in (("gpu-rows", "Light"), ("gpu-predicate", "Heavy")):
    for host in ("laptop", "pods", "h100"):
        sub = ALL[(ALL.host == host) & (ALL.engine == eng)]
        pairs = sub.pivot_table(index=["block", "E"], columns="h", values="sel").dropna()
        best = pairs.groupby(level="E").apply(lambda g: min(g[0].min(), g[1].min()))
        cells = []
        for h in (0, 1):
            rel = np.sort([pairs.loc[(b, E), h] / best[E] for b, E in pairs.index])
            p90_author = rel[int(0.9 * (len(rel) - 1))]
            cells.append(f"med {np.median(rel):.2f} p90(idx) {p90_author:.2f} p90(np) {np.percentile(rel, 90):.2f} >1.5 {100 * np.mean(rel > 1.5):.0f}%")
        print(f"  {lab:5s} {host:6s} n={len(pairs)} OFF: {cells[0]} | ON: {cells[1]}")

# ---------------- Table 7 + Table 6: paired heater ratios
rng = np.random.default_rng(12345)
def paired(host, eng, E, metric):
    sub = ALL[(ALL.host == host) & (ALL.engine == eng) & (ALL.E == E)]
    p = sub.pivot_table(index="block", columns="h", values=metric).dropna()
    if len(p) == 0:
        return None
    logs = np.log(p[1] / p[0]).to_numpy()
    idx = rng.integers(0, len(logs), size=(20000, len(logs)))
    meds = np.median(logs[idx], axis=1)
    lo, hi = np.percentile(meds, [2.5, 97.5])
    return math.exp(np.median(logs)), math.exp(lo), math.exp(hi), len(logs), np.round(np.exp(logs), 3).tolist()
print("\n=== TABLE 7 paired heater-on/off ratio, median over blocks [95% bootstrap]")
T7 = {}
for host, Es in (("laptop", (100, 1000, 3000, 10000, 30000)), ("pods", (100, 1000, 3000, 10000, 30000)), ("h100", (1000, 3000, 10000))):
    for E in Es:
        row = []
        for eng, m in (("gpu-rows", "sel"), ("gpu-predicate", "sel"), ("gpu-rows", "p50"), ("gpu-predicate", "p50"), ("gpu-rows", "mat")):
            r = paired(host, eng, E, m); T7[(host, E, eng, m)] = r
            row.append(f"{r[0]:.2f} [{r[1]:.2f},{r[2]:.2f}]")
        print(f"  {host:6s} E={E:>6} n={T7[(host, E, 'gpu-rows', 'sel')][3]} Ldev {row[0]} Hdev {row[1]} Lcall {row[2]} Hcall {row[3]} | Lmat {row[4]}")
print("\n=== TABLE 6 scorecard recomputed (P1 call<0.9 & hi<1 for E<=3000; P2 heavy call>=0.95; P3 call/device; P4 dev hi<1 & mat in [0.9,1.1])")
for (host, E, eng, m) in [k for k in T7 if k[2] == "gpu-rows" and k[3] == "sel"]:
    lp, hp, ls, hs, lm = T7[(host, E, "gpu-rows", "p50")], T7[(host, E, "gpu-predicate", "p50")], T7[(host, E, "gpu-rows", "sel")], T7[(host, E, "gpu-predicate", "sel")], T7[(host, E, "gpu-rows", "mat")]
    yn = lambda b: "yes" if b else "no"
    p1 = yn(lp[0] < 0.9 and lp[2] < 1) if E <= 3000 else "n/a"
    print(f"  {host:6s} E={E:>6} P1 {p1:3s} P2 {yn(hp[0] >= 0.95)} P3 {yn(lp[0] < hp[0])}/{yn(ls[0] < hs[0])} P4 {yn(ls[2] < 1 and 0.9 <= lm[0] <= 1.1)}")

# pods block-4 perturbed cells: drop b4 gpu-rows E=100 pair
sub = ALL[(ALL.host == "pods") & (ALL.engine == "gpu-rows") & (ALL.E == 100)].pivot_table(index="block", columns="h", values="p50")
r_all = math.exp(np.median(np.log(sub[1] / sub[0]))); r_drop = math.exp(np.median(np.log(sub.drop(4)[1] / sub.drop(4)[0])))
print(f"\npods light E=100 call ratio all blocks {r_all:.4f}; dropping block 4 {r_drop:.4f}; per-block {np.round(sub[1] / sub[0], 4).to_dict()}")

# ---------------- Table 8
print("\n=== TABLE 8 median-block end-to-end P50 ms")
for host in ("laptop", "pods"):
    for E in (100, 1000, 3000, 10000, 30000):
        g = lambda eng, h: ALL[(ALL.host == host) & (ALL.engine == eng) & (ALL.E == E) & (ALL.h == h)].p50.median()
        cpu, l0, l1, hv = g("cpu", 0), g("gpu-rows", 0), g("gpu-rows", 1), g("gpu-predicate", 0)
        w = lambda x: "GPU" if x < cpu else "CPU"
        print(f"  {host:6s} E={E:>6} CPU {cpu:.3f} light-off {l0:.3f} ({w(l0)}) light-on {l1:.3f} ({w(l1)}) heavy {hv:.3f}")

# ---------------- kernel share of call (text: heavy 52-84%, light 13-43%)
ks = ALL[(ALL.h == 0) & ALL.engine.isin(["gpu-rows", "gpu-predicate"])].groupby(["engine", "host", "E"]).kshare.median()
print("\nkernel (score+select)/call median, heater off:")
for eng in ("gpu-predicate", "gpu-rows"):
    for Es, tag in (((1000, 3000, 10000), "E in 1k-10k"), ((100, 1000, 3000, 10000, 30000), "all E")):
        v = ks[eng][ks[eng].index.get_level_values("E").isin(Es)]
        print(f"  {eng:14s} {tag:12s} range {v.min():.2f}-{v.max():.2f}")

# ---------------- E-LOCK Table 4
def load_locked(root, conds_from_name):
    recs, exc = [], []
    for d in sorted(glob.glob(os.path.join(root, "block-*", "gpu-*"))):
        if not os.path.exists(os.path.join(d, "lock.json")):
            continue
        name = os.path.basename(d); eng = "gpu-rows" if name.startswith("gpu-rows") else "gpu-predicate"
        parts = name[len(eng) + 2:].split("-"); E = int(parts[0]); cond = "-".join(parts[1:])
        j = json.load(open(os.path.join(d, "lock.json")))
        r = proc(d); r.pop("_act", None)
        r.update(block=int(os.path.basename(os.path.dirname(d)).split("-")[1]), engine=eng, E=E, cond=cond, rc=j["returncode"], req=j.get("requested"), tmax=j.get("temperature_max_c"))
        recs.append(r)
    return pd.DataFrame(recs)
LK = load_locked(os.path.join(RAW, "2026-10-19-dvfs-lock"), None)
print(f"\n=== E-LOCK: {len(LK)} runs, rc!=0: {(LK.rc != 0).sum()}, max temp {LK.tmax.max()}, min recall {LK.min_recall.min()}")
bad = LK[LK.req.notna() & LK.apply(lambda r: r.req is not None and abs(r.sm_act - r.req[0]) > 0.05 * r.req[0], axis=1)]
print("  locked runs whose GPU-active SM median differs >5% from request:", len(bad))
memoff = LK[LK.apply(lambda r: r.req is not None and abs(r.mem_act - r.req[1]) > 0.05 * r.req[1], axis=1)]
print("  locked runs whose GPU-active MEM median differs >5% from request:", len(memoff), sorted(set(zip(memoff.cond, memoff.mem_act))))
def lmed(df, eng, E, cond, key="sel"):
    v = df[(df.engine == eng) & (df.E == E) & (df.cond == cond)][key]
    return v.median(), v.max() / v.min(), len(v)
print("  TABLE 4: E | U ms max/min | Lhi | Llo | Heavy Lhi")
for E in (1000, 3000, 10000):
    c = [lmed(LK, "gpu-rows", E, k) for k in ("U-h0", "Lhi-h0", "Llo-h0")] + [lmed(LK, "gpu-predicate", E, "Lhi-h0")]
    print("   ", E, " | ".join(f"{m:.4f} x{mm:.2f} n={n}" for m, mm, n in c))
for E in (1000, 3000, 10000):
    u0, u1, hi = lmed(LK, "gpu-rows", E, "U-h0")[0], lmed(LK, "gpu-rows", E, "U-h1")[0], lmed(LK, "gpu-rows", E, "Lhi-h0")[0]
    best_run = LK[(LK.engine == "gpu-rows") & (LK.E == E) & LK.cond.isin(["U-h0", "U-h1"])].sel.min()
    hu = lmed(LK, "gpu-predicate", E, "Lhi-h0")[0] / lmed(LK, "gpu-predicate", E, "U-h0")[0]
    def pr(c1, c0):
        p = LK[(LK.engine == "gpu-rows") & (LK.E == E) & LK.cond.isin([c1, c0])].pivot_table(index="block", columns="cond", values="sel")
        return float(np.median(p[c1] / p[c0]))
    print(f"   E={E}: Lhi/bestUmedian {hi / min(u0, u1):.3f}; Lhi/best U run {hi / best_run:.3f}; U/Lhi (unlocked slowdown) {u0 / hi:.2f}; heavy Lhi/U {hu:.4f}; "
          f"U heater ratio {pr('U-h1', 'U-h0'):.3f}; Lhi heater ratio {pr('Lhi-h1', 'Lhi-h0'):.3f}; Llo/D1(0.461) {lmed(LK, 'gpu-rows', E, 'Llo-h0')[0] / 0.461:.3f}")
print("   heavy Llo/Lhi:", {E: round(lmed(LK, "gpu-predicate", E, "Llo-h0")[0] / lmed(LK, "gpu-predicate", E, "Lhi-h0")[0], 3) for E in (1000, 3000, 10000)})

# ---------------- E-SPLIT Table 5
SP = load_locked(os.path.join(RAW, "2026-10-25-dvfs-split"), None)
REQ = {"HH": (2670, 8001), "HL": (2670, 810), "LH": (585, 8001), "LL": (585, 810)}
SP["excl"] = SP.apply(lambda r: r.rc != 0 or abs(r.sm_act - REQ[r.cond][0]) > 0.05 * REQ[r.cond][0] or abs(r.mem_act - REQ[r.cond][1]) > 0.05 * REQ[r.cond][1], axis=1)
print(f"\n=== E-SPLIT: {len(SP)} runs, excluded {SP.excl.sum()}: {SP[SP.excl][['block', 'engine', 'E', 'cond', 'sm_act', 'mem_act', 'sel']].to_dict('records')}")
print("  observed mem_act per condition:", SP.groupby(["engine", "E", "cond"]).mem_act.agg(lambda v: sorted(set(v))).to_dict())
K = SP[~SP.excl]
print("  TABLE 5: path E | sel HH HL LH LL m s | score HH HL LH LL m s")
for eng in ("gpu-rows", "gpu-predicate"):
    for E in (1000, 3000, 10000):
        out = []
        for key in ("sel", "score"):
            t = {c: K[(K.engine == eng) & (K.E == E) & (K.cond == c)][key].median() for c in REQ}
            n = {c: int(((K.engine == eng) & (K.E == E) & (K.cond == c)).sum()) for c in REQ}
            m = math.log(t["HL"] / t["HH"]) / math.log(t["LL"] / t["HH"]); s = math.log(t["LH"] / t["HH"]) / math.log(t["LL"] / t["HH"])
            out.append(" ".join(f"{t[c]:.3f}" for c in REQ) + f" m={m:.3f} s={s:.3f} m+s={m + s:.2f} n={list(n.values())}")
        print(f"   {eng:14s} {E:>6} | {out[0]} | {out[1]}")
# with the excluded LH runs included (sensitivity)
for E in (1000, 3000):
    a = K[(K.engine == "gpu-rows") & (K.E == E) & (K.cond == "LH")].sel.median()
    b = SP[(SP.engine == "gpu-rows") & (SP.E == E) & (SP.cond == "LH")].sel.median()
    print(f"   sensitivity light LH E={E}: kept-only {a:.4f}, incl. excluded {b:.4f}")
# S0 vs E-LOCK
for eng in ("gpu-rows", "gpu-predicate"):
    for E in (1000, 3000, 10000):
        print(f"   S0 {eng} E={E}: HH/Lhi {K[(K.engine == eng) & (K.E == E) & (K.cond == 'HH')].sel.median() / lmed(LK, eng, E, 'Lhi-h0')[0]:.4f}  LL/Llo {K[(K.engine == eng) & (K.E == E) & (K.cond == 'LL')].sel.median() / lmed(LK, eng, E, 'Llo-h0')[0]:.4f}")
print("\nmin recall E-SPLIT", SP.min_recall.min())
