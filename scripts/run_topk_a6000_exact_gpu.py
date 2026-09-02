#!/usr/bin/env python3
"""A6000 TopK Bench exact filtered-GPU comparison.

Measurement boundary: a host-side search call through IDs/distances copied back to
host. Dataset decoding, eligible-set construction and index construction are all
outside the timed region. This script intentionally records an adapter failure
instead of silently substituting another implementation.
"""
from __future__ import annotations

import argparse, csv, datetime as dt, hashlib, json, os, platform, subprocess
import time, traceback
from pathlib import Path

import numpy as np
import pyarrow.parquet as pq
torch = None

K, NQ, WARMUP, REPS = 10, 1000, 100, 10


def command(cmd):
    try:
        return subprocess.check_output(cmd, text=True, stderr=subprocess.STDOUT).strip()
    except Exception as e:
        return f"<failed: {e}>"


def load(path_docs: str, path_queries: str):
    docs = pq.read_table(path_docs, columns=["id", "dense", "int_filter"])
    qtab = pq.read_table(path_queries, columns=["dense", "recall"])
    dvals = docs["dense"].combine_chunks().values.to_numpy(zero_copy_only=False)
    qvals = qtab["dense"].combine_chunks().values.to_numpy(zero_copy_only=False)
    vecs = np.asarray(dvals, dtype=np.float32).reshape(len(docs), -1)
    queries = np.asarray(qvals, dtype=np.float32).reshape(len(qtab), -1)
    # TopK Bench providers use cosine. Keep the source Parquet untouched and
    # normalize only these in-memory working copies before exact IP search.
    vecs /= np.linalg.norm(vecs, axis=1, keepdims=True)
    queries /= np.linalg.norm(queries, axis=1, keepdims=True)
    ids = np.asarray(docs["id"].to_pylist(), dtype=np.int64)
    filt = np.asarray(docs["int_filter"].to_numpy(), dtype=np.int32)
    recalls = qtab["recall"].to_pylist()
    if len(vecs) != 1_000_000 or vecs.shape[1] != 768 or len(queries) < NQ:
        raise RuntimeError(f"unexpected TopK shapes docs={vecs.shape}, queries={queries.shape}")
    return vecs, ids, filt, queries[:NQ], recalls[:NQ]


def expected(recall_row, threshold):
    # TopK records recall[int_filter_threshold][keyword_filter_threshold].
    # We vary only the integer predicate and use an unfiltered keyword field.
    values = recall_row[str(threshold)]["10000"]
    answer = [int(x) for x in values if int(x) >= 0][:K]
    if len(answer) != K:
        raise RuntimeError(f"ground truth has fewer than {K} valid IDs for {key}")
    return answer


class QenloCuda:
    name = "qenlo_cuda_predicate_prototype"
    def __init__(self, vectors):
        global torch
        if torch is None:
            import torch as imported_torch
            torch = imported_torch
        self.vecs = torch.as_tensor(vectors, device="cuda", dtype=torch.float32)
    def search(self, query):
        q = torch.as_tensor(query, device="cuda", dtype=torch.float32)
        vals, pos = torch.topk(torch.mv(self.vecs, q), K, largest=True, sorted=True)
        return pos.cpu().numpy(), vals.cpu().numpy()


class FaissGpuFlat:
    name = "faiss_gpu_flat_ip"
    def __init__(self, vectors):
        import faiss
        self.faiss = faiss
        self.res = faiss.StandardGpuResources()
        self.index = faiss.GpuIndexFlatIP(self.res, vectors.shape[1])
        self.index.add(np.ascontiguousarray(vectors))
    def search(self, query):
        vals, pos = self.index.search(np.ascontiguousarray(query[None, :]), K)
        return pos[0], vals[0]


class CuvsBruteForce:
    name = "cuvs_brute_force_ip"
    def __init__(self, vectors):
        import cupy as cp
        from cuvs.neighbors import brute_force
        self.cp, self.bf = cp, brute_force
        self.vecs = cp.asarray(vectors)
        # The pre-materialized eligible set is identical to the other adapters;
        # its construction is deliberately outside all timed calls.
        self.index = brute_force.build(self.vecs, metric="inner_product")
    def search(self, query):
        q = self.cp.asarray(query[None, :])
        vals, pos = self.bf.search(self.index, q, K)
        # cuVS 26.x returns pylibraft device_ndarray, not a CuPy ndarray.
        return pos.copy_to_host()[0], vals.copy_to_host()[0]


def make_adapter(kind, vectors):
    return {"qenlo": QenloCuda, "faiss": FaissGpuFlat, "cuvs": CuvsBruteForce}[kind](vectors)


def run_one(adapter, candidate_ids, queries, gt, threshold, out_rows, failure_rows):
    try:
        for q in queries[:WARMUP]: adapter.search(q)
        if torch is not None:
            torch.cuda.synchronize()
        for rep in range(REPS):
            order = np.random.default_rng(20260902 + threshold * 100 + rep).permutation(NQ)
            for ordinal, qi in enumerate(order):
                start = time.perf_counter_ns()
                pos, _ = adapter.search(queries[qi])
                elapsed = time.perf_counter_ns() - start
                predicted = candidate_ids[np.asarray(pos, dtype=np.int64)].tolist()
                wanted = gt[qi]
                hits = len(set(predicted) & set(wanted))
                out_rows.append({"system": adapter.name, "threshold": threshold, "rep": rep,
                    "ordinal": ordinal, "query_id": int(qi), "latency_ns_e2e": int(elapsed),
                    "recall_at_10": hits / K, "incorrect_top10": K - hits,
                    "returned_ids": json.dumps([int(x) for x in predicted]),
                    "ground_truth_ids": json.dumps(wanted)})
    except Exception:
        failure_rows.append({"system": getattr(adapter, "name", type(adapter).__name__),
            "threshold": threshold, "stage": "search", "traceback": traceback.format_exc()})


def main():
    global torch
    ap = argparse.ArgumentParser()
    ap.add_argument("--docs", default="/workspace/topk-data/docs-1m.parquet")
    ap.add_argument("--queries", default="/workspace/topk-data/queries-1m.parquet")
    ap.add_argument("--out", default="/workspace/results/topk-a6000")
    ap.add_argument("--systems", nargs="+", choices=("qenlo", "faiss", "cuvs"),
                    default=("qenlo", "faiss", "cuvs"))
    args = ap.parse_args()
    out = Path(args.out); out.mkdir(parents=True, exist_ok=True)
    rows, failures = [], []
    vecs, ids, filt, queries, recall_rows = load(args.docs, args.queries)
    gt = {t: [expected(x, t) for x in recall_rows] for t in (10000, 1000, 100)}
    manifest = {"timestamp_utc": dt.datetime.now(dt.timezone.utc).isoformat(), "protocol": {
        "dataset": "TopK Bench docs-1m/queries-1m unchanged", "N": int(len(vecs)),
        "D": int(vecs.shape[1]), "k": K, "queries": NQ, "warmup_queries": WARMUP,
        "repetitions": REPS, "measurement": "host search call through host IDs/distances; H2D, GPU work, D2H and synchronization included; dataset/filter/index setup excluded",
        "metric": "cosine (normalized in-memory working copies; source Parquet unchanged)",
        "eligible_set": "int_filter <= threshold, matching TopK provider implementation; materialized before timing for every adapter"},
        "environment": {"python": platform.python_version(), "torch": None,
            "cuda": None, "gpu": command(["nvidia-smi", "--query-gpu=name", "--format=csv,noheader"]),
            "nvidia_smi": command(["nvidia-smi", "--query-gpu=name,driver_version,memory.total", "--format=csv,noheader"]),
            "faiss": None, "cuvs": None}}
    if "qenlo" in args.systems:
        try:
            import torch as imported_torch
            torch = imported_torch
            manifest["environment"]["torch"] = torch.__version__
            manifest["environment"]["cuda"] = torch.version.cuda
        except Exception as e: manifest["environment"]["torch"] = f"IMPORT_FAILED: {e}"
    if "faiss" in args.systems:
        try:
            import faiss; manifest["environment"]["faiss"] = faiss.__version__
        except Exception as e: manifest["environment"]["faiss"] = f"IMPORT_FAILED: {e}"
    if "cuvs" in args.systems:
        try:
            import cuvs; manifest["environment"]["cuvs"] = getattr(cuvs, "__version__", "imported")
        except Exception as e: manifest["environment"]["cuvs"] = f"IMPORT_FAILED: {e}"
    (out / "environment.json").write_text(json.dumps(manifest, indent=2))
    for threshold in (10000, 1000, 100):
        selected = np.flatnonzero(filt <= threshold)
        candidate_vecs, candidate_ids = np.ascontiguousarray(vecs[selected]), ids[selected]
        print(f"threshold={threshold} candidates={len(selected)}", flush=True)
        for kind in args.systems:
            try:
                adapter = make_adapter(kind, candidate_vecs)
                run_one(adapter, candidate_ids, queries, gt[threshold], threshold, rows, failures)
                del adapter
                if torch is not None: torch.cuda.empty_cache()
            except Exception:
                failures.append({"system": kind, "threshold": threshold, "stage": "initialization", "traceback": traceback.format_exc()})
    with (out / "raw_samples.csv").open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=["system","threshold","rep","ordinal","query_id","latency_ns_e2e","recall_at_10","incorrect_top10","returned_ids","ground_truth_ids"]); w.writeheader(); w.writerows(rows)
    (out / "failures.json").write_text(json.dumps(failures, indent=2))
    for p in out.iterdir():
        if p.is_file(): print(hashlib.sha256(p.read_bytes()).hexdigest(), p.name)
    print(json.dumps({"samples": len(rows), "failures": len(failures)}, indent=2))

if __name__ == "__main__": main()
