"""Summarize the cross-host run: python analyze.py [raw_dir]

Per host: int8/Auto vs base Certified/Auto (median of the two rounds), probe heap and exactness,
and the CPU vs gpu-rows crossover from qenlo-bench summaries.
"""
import csv, re, statistics, sys
from pathlib import Path

raw = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).parent / "raw")


def cells(paths):
    out = {}
    for p in paths:
        for r in csv.DictReader(open(p)):
            out.setdefault((int(r["E"]), r["mode"]), []).append((float(r["median_ms"]), float(r["p95_ms"])))
    return {k: (statistics.median(m for m, _ in v), statistics.median(p for _, p in v)) for k, v in out.items()}


def env(o):
    t = (o / "environment.txt").read_text(errors="replace")
    cpu = re.search(r"Model name:\s+(.*)", t)
    gpu = re.search(r"Product Name\s+:\s+(.*)", t)
    n = re.search(r"^CPU\(s\):\s+(\d+)", t, re.M)
    return f"{cpu.group(1).strip() if cpu else '?'} ({n.group(1) if n else '?'} threads)", gpu.group(1).strip() if gpu else "-"


for host in sorted(d for d in raw.iterdir() if (d / "out/DONE").exists()):
    o = host / "out"
    cpu, gpu = env(o)
    print(f"\n## {host.name}: {cpu}; GPU {gpu}")
    base = cells([o / "base_1.csv", o / "base_2.csv"])
    cand = cells([o / "cand_1.csv", o / "cand_2.csv"])
    print("E | base Auto p95 | cand Auto p95 | ratio | cand Int8 p95 / base Certified p95")
    for e in sorted({e for e, _ in cand}):
        b, c = base.get((e, "Auto")), cand.get((e, "Auto"))
        i, bc = cand.get((e, "Int8")), base.get((e, "Certified"))
        if b and c and i and bc:
            print(f"{e} | {b[1]:.3f} | {c[1]:.3f} | {c[1] / b[1]:.2f} | {i[1] / bc[1]:.2f}")
    for name in ("base", "cand"):
        lines = (o / f"{name}_probe.txt").read_text().splitlines()
        rows = [l.split(",") for l in lines[3:]]
        print(f"{name} probe: {lines[0].split('  vectors')[0]}; mismatches {sum(int(r[5]) for r in rows)}; "
              f"sq8/certified at E=100k {rows[-1][3]}")
    xo = o / "xover"
    if xo.exists():
        print("E | cpu p95 ms | gpu-rows p95 ms | gpu/cpu | recall cpu, gpu")
        for e in (4000, 10000, 30000, 100000):
            got = {}
            for be in ("cpu", "gpu-rows"):
                s = xo / f"{be}-e{e}" / "summary.txt"
                if s.exists():
                    kv = dict(l.split("=", 1) for l in s.read_text().splitlines() if "=" in l)
                    got[be] = (int(kv["median_run_p95_batch_ns"]) / 1e6, kv["evaluation_recall_at_k"])
            if len(got) == 2:
                print(f"{e} | {got['cpu'][0]:.3f} | {got['gpu-rows'][0]:.3f} | "
                      f"{got['gpu-rows'][0] / got['cpu'][0]:.2f} | {got['cpu'][1]}, {got['gpu-rows'][1]}")
