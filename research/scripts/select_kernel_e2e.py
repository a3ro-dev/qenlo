"""End-to-end view of the k-pass vs tiled selection A/B (no new runs): per process, the median
batch latency and median device time; per (E, k), the median over blocks of tiled/k-pass ratios.

    python -I research/scripts/select_kernel_e2e.py data/select-kernel-ab-2026-10-25
"""
import csv, math, statistics as st, sys
from collections import defaultdict
from pathlib import Path

root = Path(sys.argv[1])
cell = defaultdict(dict)  # (E, k) -> {(block, kernel): (call, device)}
for d in sorted(root.iterdir()):
    if not (d / "samples.csv").exists():
        continue
    b, e, k, kernel = d.name.split("-", 3)
    rows = list(csv.DictReader(open(d / "samples.csv")))
    call = st.median(int(r["batch_latency_ns"]) for r in rows) / 1e6
    dev = st.median(int(r["device_selection_ns"] or 0) + int(r["device_scoring_ns"] or 0) for r in rows) / 1e6
    cell[(int(e[1:]), int(k[1:]))][(b, kernel)] = (call, dev)

print("E      k   call k-pass  call tiled  call ratio   device ratio   blocks")
for (E, k), m in sorted(cell.items()):
    blocks = sorted({b for b, _ in m})
    rc = [m[(b, "tiled")][0] / m[(b, "k-pass")][0] for b in blocks]
    rd = [m[(b, "tiled")][1] / m[(b, "k-pass")][1] for b in blocks]
    med = lambda v: math.exp(st.median(math.log(x) for x in v))
    ck = st.median(m[(b, "k-pass")][0] for b in blocks); ct = st.median(m[(b, "tiled")][0] for b in blocks)
    print(f"{E:<6} {k:<3} {ck:10.3f}  {ct:10.3f}  {med(rc):9.2f} ({min(rc):.2f}-{max(rc):.2f})  {med(rd):5.2f} ({min(rd):.2f}-{max(rd):.2f})  {len(blocks)}")
