#!/usr/bin/env python3
"""Independent post-hoc TopK ground-truth verification of retained returned IDs."""
import argparse, csv, hashlib, json
from collections import defaultdict
from pathlib import Path
import pyarrow.parquet as pq

def main():
    ap=argparse.ArgumentParser(); ap.add_argument("--queries", required=True); ap.add_argument("--root", required=True); args=ap.parse_args()
    recall=pq.read_table(args.queries, columns=["recall"])["recall"].to_pylist()
    root=Path(args.root); report=[]
    for d in ("qenlo", "faiss", "cuvs"):
        path=root/d/"raw_samples.csv"; counts=defaultdict(lambda: [0,0,1.0])
        with path.open(newline="") as f:
            for r in csv.DictReader(f):
                t=int(r["threshold"]); q=int(r["query_id"])
                truth=[int(x) for x in recall[q][str(t)]["10000"] if int(x)>=0][:10]
                got=json.loads(r["returned_ids"]); score=len(set(got)&set(truth))/10
                counts[t][0]+=1; counts[t][1]+= score == 1.0; counts[t][2]=min(counts[t][2],score)
        report.append({"system":d,"raw_sha256":hashlib.sha256(path.read_bytes()).hexdigest(),
          "thresholds":[{"int_filter_lt":t,"samples":v[0],"perfect_recall_rows":v[1],"min_recall_at_10":v[2],"mean_recall_at_10":v[1]/v[0] if v[1]==v[0] else None} for t,v in sorted(counts.items())]})
    (root/"independent_recall_verification.json").write_text(json.dumps(report,indent=2)); print(json.dumps(report,indent=2))

if __name__=="__main__": main()
