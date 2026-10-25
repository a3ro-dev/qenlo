"""Analyze E-SPLIT (research/experiments/dvfs-split/protocol.md). Prints S0-S4 and writes paper/v2/tables/split.tex.

    python research/scripts/analyze_dvfs_split.py
"""
import csv, json, math, statistics as st, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RAW = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "research/data/raw/2026-10-25-dvfs-split"
LOCK = ROOT / "research/data/raw/2026-10-19-dvfs-lock"
EP = ["gpu-rows", "gpu-predicate"]
ES = [1000, 3000, 10000]
REQ = {"HH": (2670, 8001), "HL": (2670, 810), "LH": (585, 8001), "LL": (585, 810)}
NAME = {"gpu-rows": "light", "gpu-predicate": "heavy"}


def measure(d):
    S = list(csv.DictReader(open(d / "bench/samples.csv")))
    med = lambda k: st.median(int(s[k] or 0) for s in S) / 1e6
    sm, mem = [], []
    for line in open(d / "clocks.csv"):
        f = [x.strip() for x in line.split(",")]
        if len(f) > 4 and f[1].isdigit() and f[2].isdigit() and f[4].isdigit() and int(f[4]) > 0:
            sm.append(int(f[1])); mem.append(int(f[2]))
    nan = float("nan")
    return dict(sel=med("device_selection_ns"), score=med("device_scoring_ns"), call=med("batch_latency_ns"),
                sm=st.median(sm) if sm else nan, mem=st.median(mem) if mem else nan)


def split_name(name):
    eng = "gpu-rows" if name.startswith("gpu-rows") else "gpu-predicate"
    E, cond = name[len(eng) + 2:].split("-")[:2]
    return eng, int(E), cond


runs, excluded = {}, []
for d in sorted(RAW.glob("block-*/gpu-*")):
    if ".aborted-" in d.name or not (d / "lock.json").exists():
        continue
    b = int(d.parent.name.split("-")[1]); eng, E, cond = split_name(d.name)
    info = json.load(open(d / "lock.json"))
    rec = dict(block=b, engine=eng, E=E, cond=cond, tmax=info["temperature_max_c"], **measure(d))
    sm_req, mem_req = REQ[cond]
    off = abs(rec["sm"] - sm_req) > 0.05 * sm_req or abs(rec["mem"] - mem_req) > 0.05 * mem_req
    if info["returncode"] != 0 or info["thermal_abort"] or off or math.isnan(rec["sm"]):
        excluded.append(rec)
    else:
        runs[(b, eng, E, cond)] = rec

print(f"runs loaded: {len(runs) + len(excluded)}, excluded: {len(excluded)}")
for r in excluded:
    print("  EXCLUDED", r)
if runs:
    print("max temperature:", max(r["tmax"] for r in runs.values()))


def vals(eng, E, cond, key):
    return [r[key] for (b, e, n, c), r in sorted(runs.items()) if (e, n, c) == (eng, E, cond)]


def med(eng, E, cond, key):
    v = vals(eng, E, cond, key)
    return st.median(v) if v else float("nan")


def share(eng, E, key, cond):
    """ln(t_cond / t_HH) / ln(t_LL / t_HH): 0 = clock in `cond` that is low does not matter, 1 = it explains all."""
    hh, ll, x = med(eng, E, "HH", key), med(eng, E, "LL", key), med(eng, E, cond, key)
    return math.log(x / hh) / math.log(ll / hh)


# S0: rebuilt binary vs E-LOCK (Lhi-h0 ~ HH, Llo-h0 ~ LL), selection medians
lock = {}
for d in LOCK.glob("block-*/gpu-*-h0"):
    eng, E, cond = split_name(d.name)
    if cond in ("Lhi", "Llo"):
        lock.setdefault((eng, E, cond), []).append(measure(d)["sel"])
print("\nS0 rebuild check (selection median, E-SPLIT / E-LOCK, within 10%):")
s0 = True
for eng in EP:
    for E in ES:
        for new, old in (("HH", "Lhi"), ("LL", "Llo")):
            r = med(eng, E, new, "sel") / st.median(lock[(eng, E, old)])
            ok = abs(r - 1) <= 0.10; s0 &= ok
            print(f"  {NAME[eng]:5} E={E:>5} {new}/{old} = {r:.3f} {'ok' if ok else 'FAIL'}")
print("S0", "HOLDS" if s0 else "FAILS")

print("\nS1 light selection max/min across blocks (<= 1.15):")
s1 = True
for E in ES:
    for c in REQ:
        v = vals("gpu-rows", E, c, "sel")
        if not v:
            print(f"  E={E:>5} {c}: not testable (n=0)"); continue
        mm = max(v) / min(v); s1 &= mm <= 1.15
        print(f"  E={E:>5} {c}: {mm:.2f} (n={len(v)})")
print("S1", "HOLDS" if s1 else "FAILS")

print("\nmemory share m and SM share s (S2: selection m < 0.3; S3: scoring m < 0.5; S4: m + s):")
s2 = s3 = True
tested = {"sel": 0, "score": 0}
for key, lim in (("sel", 0.3), ("score", 0.5)):
    for eng in EP:
        for E in ES:
            m, s = share(eng, E, key, "HL"), share(eng, E, key, "LH")
            if math.isnan(m):
                print(f"  {key:5} {NAME[eng]:5} E={E:>5}: m not testable (s={s:.2f})"); continue
            ok = m < lim; tested[key] += 1
            if key == "sel":
                s2 &= ok
            else:
                s3 &= ok
            print(f"  {key:5} {NAME[eng]:5} E={E:>5}: m={m:.2f} s={s:.2f} m+s={m + s:.2f} {'ok' if ok else 'FAIL'}")
print("S2", "NOT TESTABLE" if not tested["sel"] else "HOLDS" if s2 else "FAILS")
print("S3", "NOT TESTABLE" if not tested["score"] else "HOLDS" if s3 else "FAILS")

# paper table: selection and scoring medians per condition, with m
rows = []
for eng in EP:
    for E in ES:
        cells = [f"{med(eng, E, c, 'sel'):.3f}" for c in REQ] + [f"{share(eng, E, 'sel', 'HL'):.2f}"]
        cells += [f"{med(eng, E, c, 'score'):.3f}" for c in REQ] + [f"{share(eng, E, 'score', 'HL'):.2f}"]
        rows.append(f"{NAME[eng].capitalize()} & {E:,} & " + " & ".join(cells) + r" \\")
    rows.append(r"\midrule")
out = ROOT / "paper/v2/tables/split.tex"
out.write_text("\n".join(rows[:-1]) + "\n", encoding="utf-8")
print("\nwrote", out.relative_to(ROOT))
