"""Replay one Qenlo benchmark cell against a local OSS vector-search engine."""

import argparse
import csv
import hashlib
import importlib.metadata
import json
import math
import platform
import sys
import time
from pathlib import Path

from chroma_replay import load_dataset, properties, read_metadata


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def percentile(values, fraction):
    return sorted(values)[max(0, math.ceil(len(values) * fraction) - 1)]


def package_version(name):
    return importlib.metadata.version(name)


class Faiss:
    package = "faiss-cpu"

    def __init__(self, corpus, path, exact=False):
        import faiss
        self.faiss = faiss
        if exact:
            self.index = faiss.IndexFlatIP(corpus.shape[1])
            self.algorithm = "IndexFlatIP exact"
        else:
            self.index = faiss.IndexHNSWFlat(corpus.shape[1], 32, faiss.METRIC_INNER_PRODUCT)
            self.index.hnsw.efConstruction = 200
            self.index.hnsw.efSearch = 256
            self.algorithm = "IndexHNSWFlat M=32 efConstruction=200 efSearch=256"
        self.index.add(corpus)

    def query(self, vector):
        scores, ids = self.index.search(vector.reshape(1, -1), 10)
        return ids[0].tolist(), scores[0].tolist()


class Qdrant:
    package = "qdrant-client"

    def __init__(self, corpus, path, exact=False):
        from qdrant_client import QdrantClient, models
        self.client = QdrantClient(path=str(path))
        self.name = "bench"
        self.client.create_collection(
            self.name,
            vectors_config=models.VectorParams(size=corpus.shape[1], distance=models.Distance.COSINE),
            hnsw_config=models.HnswConfigDiff(m=32, ef_construct=200),
        )
        self.client.upload_collection(self.name, vectors=corpus, ids=list(range(len(corpus))), batch_size=1024)
        self.models = models
        self.exact = exact
        self.algorithm = f"Qdrant Local cosine HNSW m=32 ef=256 exact={exact}"

    def query(self, vector):
        response = self.client.query_points(
            self.name, query=vector, limit=10,
            search_params=self.models.SearchParams(hnsw_ef=256, exact=self.exact),
            with_payload=False, with_vectors=False,
        ).points
        return [int(hit.id) for hit in response], [float(hit.score) for hit in response]


class Milvus:
    package = "pymilvus"

    def __init__(self, corpus, path, exact=False):
        from pymilvus import MilvusClient
        self.client = MilvusClient(str(path / "milvus.db"))
        self.name = "bench"
        self.client.create_collection(self.name, dimension=corpus.shape[1], metric_type="COSINE")
        for start in range(0, len(corpus), 1000):
            self.client.insert(self.name, [{"id": i, "vector": corpus[i].tolist()}
                                           for i in range(start, min(start + 1000, len(corpus)))])
        self.client.load_collection(self.name)
        self.algorithm = "Milvus Lite AUTOINDEX cosine"

    def query(self, vector):
        hits = self.client.search(self.name, data=[vector.tolist()], limit=10,
                                  search_params={"metric_type": "COSINE", "params": {"ef": 256}})[0]
        return [int(hit["id"]) for hit in hits], [float(hit["distance"]) for hit in hits]


class Lance:
    package = "lancedb"

    def __init__(self, corpus, path, exact=False):
        import lancedb
        import pyarrow as pa
        vectors = pa.FixedSizeListArray.from_arrays(pa.array(corpus.reshape(-1)), corpus.shape[1])
        table = pa.table({"id": pa.array(range(len(corpus)), type=pa.int64()), "vector": vectors})
        self.table = lancedb.connect(str(path)).create_table("bench", data=table)
        self.algorithm = "LanceDB exact flat cosine (vector index bypassed)"

    def query(self, vector):
        rows = (self.table.search(vector).metric("cosine").limit(10)
                .bypass_vector_index().select(["id"]).to_list())
        return [int(row["id"]) for row in rows], [float(row.get("_distance", 0)) for row in rows]


BACKENDS = {"faiss-hnsw": (Faiss, False), "faiss-flat": (Faiss, True),
            "qdrant": (Qdrant, False), "qdrant-exact": (Qdrant, True),
            "milvus": (Milvus, False), "lancedb-flat": (Lance, True)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--backend", choices=BACKENDS, required=True)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--dataset", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--warmups", type=int, default=50)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("output already exists")
    args.output.mkdir(parents=True)
    config = properties(args.reference / "configuration.txt")
    corpus, tuning, evaluation = load_dataset(args.dataset, config)
    read_metadata(args.reference / "metadata.csv", len(corpus))
    with (args.reference / "truth.csv").open(newline="") as stream:
        truths = {(row["split"], int(row["query_index"])):
                  [int(value) for value in row["ids"].split(";") if value]
                  for row in csv.DictReader(stream)}
    with (args.reference / "samples.csv").open(newline="") as stream:
        reference_samples = list(csv.DictReader(stream))
    grouped = {}
    for row in reference_samples:
        grouped.setdefault(int(row["run"]), []).append(int(row["query_indices"]))
    cls, exact = BACKENDS[args.backend]
    db_path = args.output / "database"
    build_started = time.perf_counter_ns()
    adapter = cls(corpus, db_path, exact)
    build_ns = time.perf_counter_ns() - build_started

    def recall(ids, truth):
        if len(ids) != len(set(ids)) or any(i < 0 or i >= len(corpus) for i in ids):
            raise ValueError("invalid or duplicate result IDs")
        return len(set(ids) & set(truth)) / len(truth)

    tuning_started = time.perf_counter_ns()
    tuning_recalls = [recall(adapter.query(vector)[0], truths[("tuning", i)])
                      for i, vector in enumerate(tuning)]
    tuning_ns = time.perf_counter_ns() - tuning_started
    target = float(config["recall_target"])
    tuning_recall = sum(tuning_recalls) / len(tuning_recalls)
    if tuning_recall + 1e-12 < target:
        (args.output / "failure.json").write_text(json.dumps({"status": "tuning-failed",
            "tuning_recall_at_10": tuning_recall, "recall_target": target}, indent=2) + "\n")
        raise ValueError("tuning recall target not met")
    for i in range(args.warmups):
        adapter.query(tuning[i % len(tuning)])
    run_rows, sample_rows, p95s, recalls = [], [], [], []
    for run, indices in sorted(grouped.items()):
        latencies, run_recalls = [], []
        wall_started = time.perf_counter_ns()
        for batch_index, index in enumerate(indices):
            started = time.perf_counter_ns()
            ids, scores = adapter.query(evaluation[index])
            latency = time.perf_counter_ns() - started
            value = recall(ids, truths[("evaluation", index)])
            latencies.append(latency); run_recalls.append(value)
            sample_rows.append([run, batch_index, index, latency, value,
                                ";".join(map(str, ids)), ";".join(map(str, scores))])
        wall = time.perf_counter_ns() - wall_started
        mean = sum(run_recalls) / len(run_recalls)
        p95 = percentile(latencies, .95); p95s.append(p95); recalls.append(mean)
        run_rows.append([run, len(indices), percentile(latencies, .5), p95,
                         percentile(latencies, .99), wall, len(indices) * 1e9 / wall, mean])
    with (args.output / "samples.csv").open("w", newline="") as out:
        writer = csv.writer(out); writer.writerow(["run", "batch_index", "query_index", "latency_ns",
            "recall_at_10", "result_ids", "scores"]); writer.writerows(sample_rows)
    with (args.output / "runs.csv").open("w", newline="") as out:
        writer = csv.writer(out); writer.writerow(["run", "queries", "p50_ns", "p95_ns", "p99_ns",
            "wall_ns", "qps", "recall_at_10"]); writer.writerows(run_rows)
    summary = {"status": "completed", "backend": args.backend, "algorithm": adapter.algorithm,
        "package": adapter.package, "package_version": package_version(adapter.package),
        "build_ns": build_ns, "tuning_ns": tuning_ns, "tuning_recall_at_10": tuning_recall,
        "evaluation_recall_at_10": sum(recalls) / len(recalls),
        "recall_target": target, "recall_target_passed": min(recalls) + 1e-12 >= target,
        "median_run_p95_ns": percentile(p95s, .5), "min_run_p95_ns": min(p95s),
        "max_run_p95_ns": max(p95s), "latency_boundary": "in-process single-query API completion",
        "filter_semantics": "no filter; reference fraction=1 includes all corpus rows",
        "dataset_sha256": digest(args.dataset), "reference_revision": config.get("git_revision"),
        "python": sys.version, "platform": platform.platform()}
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
