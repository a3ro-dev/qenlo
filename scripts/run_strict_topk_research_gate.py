#!/usr/bin/env python3
"""Bounded strict-1% TopK research-gate cohort.

This is deliberately a prototype comparison, not a shipped-Qenlo benchmark.
It retains a separate float64 CPU oracle and never replaces failed samples.
"""
from __future__ import annotations

import csv, datetime as dt, hashlib, json, os, platform, subprocess, time, traceback
from pathlib import Path

import numpy as np
import pyarrow.parquet as pq
import torch

N, D, K, NQ, WARMUP, REPS, THRESHOLD = 1_000_000, 768, 10, 1_000, 100, 5, 100

def command(cmd):
    try: return subprocess.check_output(cmd, text=True, stderr=subprocess.STDOUT).strip()
    except Exception as e: return f"FAILED: {e}"

def ordered_topk(scores, ids):
    # Stable secondary ID ordering makes the oracle explicit even if scores tie.
    pos = np.argpartition(-scores, K - 1)[:K]
    return pos[np.lexsort((ids[pos], -scores[pos]))]

def load_inputs(docs_path, queries_path):
    docs = pq.read_table(docs_path, columns=["id", "dense", "int_filter"])
    queries = pq.read_table(queries_path, columns=["dense", "recall"])
    d = np.asarray(docs["dense"].combine_chunks().values.to_numpy(zero_copy_only=False), dtype=np.float64).reshape(len(docs), D)
    q = np.asarray(queries["dense"].combine_chunks().values.to_numpy(zero_copy_only=False), dtype=np.float64).reshape(len(queries), D)[:NQ]
    ids = np.asarray(docs["id"].to_pylist(), dtype=np.int64)
    filt = np.asarray(docs["int_filter"].to_numpy(), dtype=np.int32)
    if d.shape != (N, D): raise RuntimeError(f"unexpected docs shape {d.shape}")
    selected = np.flatnonzero(filt < THRESHOLD)
    # The benchmarked corpus is FP32. Normalization is applied to temporary
    # working copies only, to implement cosine while retaining original files.
    raw = np.ascontiguousarray(d[selected].astype(np.float32))
    q32 = np.ascontiguousarray(q.astype(np.float32))
    raw /= np.linalg.norm(raw, axis=1, keepdims=True)
    q32 /= np.linalg.norm(q32, axis=1, keepdims=True)
    return raw, ids[selected], q32, queries["recall"].to_pylist()[:NQ], selected

def make_oracle(vectors32, candidate_ids, queries32):
    # Separate, f64, CPU-only exact oracle. It intentionally shares no GPU code.
    v = vectors32.astype(np.float64)
    q = queries32.astype(np.float64)
    truth = np.empty((len(q), K), dtype=np.int64)
    for start in range(0, len(q), 64):
        scores = v @ q[start:start + 64].T
        for col in range(scores.shape[1]):
            truth[start + col] = candidate_ids[ordered_topk(scores[:, col], candidate_ids)]
    return truth

class CpuExact:
    name = "numpy_cpu_exact_fp32"
    def __init__(self, vectors): self.vectors = vectors
    def search(self, query):
        scores = self.vectors @ query
        pos = np.argpartition(-scores, K - 1)[:K]
        return pos, scores[pos]

class QenloPrototype:
    name = "qenlo_cuda_predicate_prototype"
    def __init__(self, vectors): self.vectors = torch.as_tensor(vectors, device="cuda", dtype=torch.float32)
    def search(self, query):
        q = torch.as_tensor(query, device="cuda", dtype=torch.float32)
        values, pos = torch.topk(torch.mv(self.vectors, q), K, largest=True, sorted=False)
        return pos.cpu().numpy(), values.cpu().numpy()

class QenloBufferedPrototype:
    name = "qenlo_cuda_buffered_prototype"
    def __init__(self, vectors):
        self.vectors = torch.as_tensor(vectors, device="cuda", dtype=torch.float32)
        self.query = torch.empty(D, device="cuda", dtype=torch.float32)
        self.scores = torch.empty(len(vectors), device="cuda", dtype=torch.float32)
        self.values = torch.empty(K, device="cuda", dtype=torch.float32)
        self.positions = torch.empty(K, device="cuda", dtype=torch.int64)
    def search(self, query):
        self.query.copy_(torch.from_numpy(query))
        torch.mv(self.vectors, self.query, out=self.scores)
        torch.topk(
            self.scores,
            K,
            largest=True,
            sorted=False,
            out=(self.values, self.positions),
        )
        return self.positions.cpu().numpy(), self.values.cpu().numpy()

class FaissFlat:
    name = "faiss_gpu_flat_ip"
    def __init__(self, vectors):
        import faiss
        self.res = faiss.StandardGpuResources(); self.index = faiss.GpuIndexFlatIP(self.res, D)
        self.index.add(vectors)
    def search(self, query):
        values, pos = self.index.search(query[None, :], K)
        return pos[0], values[0]

class CuvsBruteForce:
    name = "cuvs_brute_force_ip"
    def __init__(self, vectors):
        import cupy as cp
        from cuvs.neighbors import brute_force
        self.cp, self.bf = cp, brute_force
        self.index = brute_force.build(cp.asarray(vectors), metric="inner_product")
    def search(self, query):
        # cuVS returns (neighbor_indices, distances), unlike FAISS.
        pos, values = self.bf.search(self.index, self.cp.asarray(query[None, :]), K)
        return pos.copy_to_host()[0], values.copy_to_host()[0]

def run_adapter(adapter, candidate_ids, queries, truth, out_dir, rows, failures):
    try:
        for q in queries[:WARMUP]: adapter.search(q)
        if torch.cuda.is_available(): torch.cuda.synchronize()
        for rep in range(REPS):
            order = np.random.default_rng(20260902 + rep).permutation(NQ)
            for ordinal, qi in enumerate(order):
                start = time.perf_counter_ns(); pos, _ = adapter.search(queries[qi]); elapsed = time.perf_counter_ns() - start
                got = candidate_ids[np.asarray(pos, dtype=np.int64)]
                expected = truth[qi]
                recall = len(set(got.tolist()) & set(expected.tolist())) / K
                rows.append({"system":adapter.name,"rep":rep,"ordinal":ordinal,"query_id":int(qi),
                    "latency_ns_e2e":elapsed,"recall_at_10":recall,"returned_ids":json.dumps(got.tolist()),"oracle_ids":json.dumps(expected.tolist())})
    except Exception:
        failures.append({"system":adapter.name,"traceback":traceback.format_exc()})

def stats(rows):
    result=[]
    for name in sorted({r["system"] for r in rows}):
        values=np.asarray([r["latency_ns_e2e"] for r in rows if r["system"]==name],dtype=np.float64)/1e6
        recall=np.asarray([r["recall_at_10"] for r in rows if r["system"]==name])
        result.append({"system":name,"samples":len(values),"p50_ms":float(np.percentile(values,50)),"p95_ms":float(np.percentile(values,95)),"p99_ms":float(np.percentile(values,99)),"mean_recall_at_10":float(recall.mean()),"min_recall_at_10":float(recall.min()),"perfect_recall_rows":int((recall==1).sum())})
    return result

def main():
    import argparse
    p=argparse.ArgumentParser(); p.add_argument("--docs",default="/workspace/topk/docs-1m.parquet"); p.add_argument("--queries",default="/workspace/topk/queries-1m.parquet"); p.add_argument("--out",default="/workspace/strict-gate"); p.add_argument("--systems", nargs="+", choices=["cpu", "qenlo", "qenlo-buffered", "faiss", "cuvs"], default=["cpu", "qenlo", "qenlo-buffered", "faiss", "cuvs"]); args=p.parse_args()
    cpu_count = os.cpu_count() or 1
    pinned_cpus = set(range(min(8, cpu_count)))
    try: os.sched_setaffinity(0, pinned_cpus)
    except Exception: pass
    out=Path(args.out); out.mkdir(parents=True,exist_ok=True)
    vectors, ids, queries, supplied, selected=load_inputs(args.docs,args.queries)
    truth=make_oracle(vectors,ids,queries)
    np.savez_compressed(out/"independent_fp64_oracle.npz",ids=truth)
    # Record agreement with public TopK oracle but never use it as this run's oracle.
    topk_truth=np.asarray([[int(x) for x in r["100"]["10000"] if int(x)>=0][:K] for r in supplied],dtype=np.int64)
    topk_agreement=int(np.sum(np.all(truth==topk_truth,axis=1)))
    manifest={"timestamp_utc":dt.datetime.now(dt.timezone.utc).isoformat(),"workload":{"N":N,"D":D,"k":K,"batch":1,"queries":NQ,"warmup":WARMUP,"repetitions":REPS,"strict_filter":"int_filter < 100","eligible_rows":int(len(selected)),"metric":"cosine via FP32-normalized temporary working copies","measurement":"host call through host result; H2D/GPU/D2H/synchronization included; index/filter preparation excluded","oracle":"separate CPU float64 exhaustive ranking over the same FP32 working vectors","topk_supplied_oracle_full_query_agreement":topk_agreement},"environment":{"python":platform.python_version(),"torch":torch.__version__,"cuda":torch.version.cuda,"gpu":torch.cuda.get_device_name(0),"nvidia_smi":command(["nvidia-smi","--query-gpu=name,driver_version,memory.total","--format=csv,noheader"]),"cpu":command(["lscpu"]),"threads":{"cpu_affinity":sorted(pinned_cpus),"OMP_NUM_THREADS":os.environ.get("OMP_NUM_THREADS"),"OPENBLAS_NUM_THREADS":os.environ.get("OPENBLAS_NUM_THREADS"),"MKL_NUM_THREADS":os.environ.get("MKL_NUM_THREADS")}},"transfer_bytes_per_query":{"cpu":0,"gpu_query_h2d":D*4,"gpu_result_d2h":K*8},"energy":"not available"}
    (out/"environment.json").write_text(json.dumps(manifest,indent=2))
    rows=[]; failures=[]
    adapters = {"cpu": CpuExact, "qenlo": QenloPrototype, "qenlo-buffered": QenloBufferedPrototype, "faiss": FaissFlat, "cuvs": CuvsBruteForce}
    for key in args.systems:
        cls = adapters[key]
        adapter=None
        try:
            adapter=cls(vectors); run_adapter(adapter,ids,queries,truth,out,rows,failures)
        except Exception: failures.append({"system":cls.__name__,"traceback":traceback.format_exc()})
        finally:
            del adapter
            if torch.cuda.is_available(): torch.cuda.empty_cache()
    with (out/"raw_samples.csv").open("w",newline="") as f:
        w=csv.DictWriter(f,fieldnames=["system","rep","ordinal","query_id","latency_ns_e2e","recall_at_10","returned_ids","oracle_ids"]);w.writeheader();w.writerows(rows)
    (out/"failures.json").write_text(json.dumps(failures,indent=2))
    summary=stats(rows) if rows else []
    (out/"summary.json").write_text(json.dumps(summary,indent=2))
    for pth in out.iterdir():
        if pth.is_file(): print(hashlib.sha256(pth.read_bytes()).hexdigest(),pth.name)
    print(json.dumps({"summary":summary,"failures":len(failures)},indent=2))
if __name__=="__main__": main()
