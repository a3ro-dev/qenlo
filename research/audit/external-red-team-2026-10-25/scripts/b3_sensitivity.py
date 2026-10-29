"""Compare slow pod b3 (excluded partial block) with replacement b3r, cell by cell.
python -I b3_sensitivity.py <b3-partial raw dir> <b3r raw dir>"""
import sys, glob, os, json, statistics as st, csv
def med(d, k):
    return st.median(int(r[k] or 0) for r in csv.DictReader(open(os.path.join(d, 'bench/samples.csv')))) / 1e6
for d in sorted(glob.glob(os.path.join(sys.argv[1], 'block-3', '*'))):
    n = os.path.basename(d); r = os.path.join(sys.argv[2], 'block-3', n)
    j = json.load(open(os.path.join(d, 'run.json'))); jr = json.load(open(os.path.join(r, 'run.json')))
    line = f"{n:22s} wall {j['ended_unix']-j['started_unix']:7.1f}s vs {jr['ended_unix']-jr['started_unix']:6.1f}s"
    if not n.startswith('cpu'):
        line += f"  sel {med(d,'device_selection_ns'):.4f} vs {med(r,'device_selection_ns'):.4f}"
    line += f"  call {med(d,'batch_latency_ns'):.4f} vs {med(r,'batch_latency_ns'):.4f}"
    print(line)
