"""Replay a completed qenlo-bench cell against native embedded Chroma 1.5.9.

The Rust run exports canonical metadata and independent exhaustive f64 truth.
This adapter reads the same QNLOB001 vector file and replays samples.csv order.
No server, embedding function, network transport, or alternative truth dataset.
"""

import argparse
import csv
import hashlib
import importlib.metadata
import json
import math
import platform
import struct
import sys
import time
import zlib
from pathlib import Path


def properties(path):
    return dict(line.split("=", 1) for line in path.read_text().splitlines() if "=" in line)


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def percentile(values, fraction):
    return sorted(values)[max(0, math.ceil(len(values) * fraction) - 1)]


def where_filter(config):
    terms = []
    for name, key, op in [
        ("filter_user_id", "user_id", "$eq"),
        ("filter_timestamp_from", "timestamp_micros", "$gte"),
        ("filter_timestamp_to", "timestamp_micros", "$lt"),
    ]:
        if config.get(name, ""):
            terms.append({key: {op: int(config[name])}})
    return {"$and": terms} if len(terms) > 1 else terms[0] if terms else None


def load_dataset(path, config):
    import numpy as np

    with path.open("rb") as stream:
        header = stream.read(56)
        if len(header) != 56 or header[:8] != b"QNLOB001":
            raise ValueError("unsupported or incomplete QNLOB001 header")
        dimension, rows, tuning, evaluation, seed, source = struct.unpack("<6Q", header[8:])
        if min(dimension, rows, tuning, evaluation) == 0:
            raise ValueError("zero-sized dataset dimension or split")
        size = 56 + (rows + tuning + evaluation) * dimension * 4
        if path.stat().st_size != size + 4:
            raise ValueError("dataset length does not match header")
        crc = zlib.crc32(header)
        remaining = size - 56
        while remaining:
            block = stream.read(min(1 << 20, remaining))
            crc = zlib.crc32(block, crc)
            remaining -= len(block)
        expected = struct.unpack("<I", stream.read(4))[0]
        if crc != expected or f"{crc:08x}" != config["dataset_crc32"]:
            raise ValueError("dataset checksum differs from replay reference")
    for key, value in [("dimensions", dimension), ("rows", rows), ("seed", seed)]:
        if int(config[key]) != value:
            raise ValueError(f"dataset {key} differs from replay reference")
    if not (source == 0 or source >> 32 == 1):
        raise ValueError("invalid source kind")
    vectors = np.memmap(path, dtype="<f4", mode="r", offset=56,
                        shape=(rows + tuning + evaluation, dimension))
    # Validate in bounded blocks; do not materialize a second full corpus.
    for start in range(0, len(vectors), 4096):
        block = vectors[start:start + 4096]
        if not np.isfinite(block).all() or (np.linalg.norm(block.astype(np.float64), axis=1) == 0).any():
            raise ValueError("invalid prepared vectors")
    return vectors[:rows], vectors[rows:rows + tuning], vectors[rows + tuning:]


def read_metadata(path, rows):
    with path.open(newline="") as stream:
        metadata = [{key: int(value) for key, value in row.items()} for row in csv.DictReader(stream)]
    if len(metadata) != rows or any(row["id"] != index for index, row in enumerate(metadata)):
        raise ValueError("metadata IDs must equal corpus source row IDs")
    return metadata


def eligible(row, config):
    return (not config.get("filter_user_id") or row["user_id"] == int(config["filter_user_id"])) and (
        not config.get("filter_timestamp_from") or row["timestamp_micros"] >= int(config["filter_timestamp_from"])
    ) and (not config.get("filter_timestamp_to") or row["timestamp_micros"] < int(config["filter_timestamp_to"]))


def validate(result, queries, corpus, metadata, config, truths):
    import numpy as np

    recalls = []
    if len(truths) != len(queries) or len(result["ids"]) != len(queries) or len(result["distances"]) != len(queries):
        raise ValueError("wrong response count")
    for ids, distances, query, truth in zip(result["ids"], result["distances"], queries, truths):
        ids = [int(value) for value in ids]
        if len(ids) != len(distances) or len(ids) > 10 or len(set(ids)) != len(ids):
            raise ValueError("duplicate IDs or invalid result count")
        if any(value < 0 or value >= len(metadata) or not eligible(metadata[value], config) for value in ids):
            raise ValueError("out-of-range or filter-violating result ID")
        if any(not math.isfinite(value) for value in distances) or any(a > b for a, b in zip(distances, distances[1:])):
            raise ValueError("nonfinite or unordered cosine distances")
        if ids:
            vectors = corpus[ids].astype(np.float64)
            query64 = np.asarray(query, dtype=np.float64)
            expected = 1 - vectors @ query64 / (np.linalg.norm(vectors, axis=1) * np.linalg.norm(query64))
            if np.max(np.abs(expected - distances)) > 1e-5:
                raise ValueError("returned cosine distance differs from independent f64 score by >1e-5")
        recalls.append(len(set(ids) & set(truth)) / len(truth) if truth else float(not ids))
    return sum(recalls) / len(recalls)


def replay(args):
    import chromadb
    import chromadb_rust_bindings
    from chromadb.config import Settings

    if chromadb.__version__ != "1.5.9":
        raise ValueError(f"expected chromadb==1.5.9, got {chromadb.__version__}")
    if args.ef_search < 1 or args.threads < 1:
        raise ValueError("ef-search and threads must be positive")
    reference = args.reference.resolve()
    config = properties(reference / "configuration.txt")
    if properties(reference / "summary.txt")["status"] != "completed":
        raise ValueError("reference must be completed")
    if config.get("replay_format") != "qenlo-csv-v1" or config["filter_mode"] != "shared":
        raise ValueError("reference must export qenlo-csv-v1 with shared filters")
    dataset = (args.dataset or Path(config["dataset"])).resolve()
    corpus, tuning, evaluation = load_dataset(dataset, config)
    metadata = read_metadata(reference / "metadata.csv", len(corpus))
    eligibility = sum(eligible(row, config) for row in metadata)
    if eligibility != int(config["eligible_count"]):
        raise ValueError("exported filter and metadata eligibility disagree")
    with (reference / "truth.csv").open(newline="") as stream:
        truth = {(row["split"], int(row["query_index"])): [int(value) for value in row["ids"].split(";") if value]
                 for row in csv.DictReader(stream)}
    for split, queries in [("tuning", tuning), ("evaluation", evaluation)]:
        for index in range(len(queries)):
            ids = truth[(split, index)]
            if len(ids) != min(10, eligibility) or len(ids) != len(set(ids)) or any(
                value < 0 or value >= len(corpus) or not eligible(metadata[value], config) for value in ids
            ):
                raise ValueError("invalid reference ground truth")
    with (reference / "samples.csv").open(newline="") as stream:
        samples = list(csv.DictReader(stream))
    grouped = {}
    for row in samples:
        if len(row["query_indices"].split(";")) != int(row["query_count"]) or not 1 <= int(row["query_count"]) <= int(config["batch"]):
            raise ValueError("reference batch query count mismatch")
        grouped.setdefault(int(row["run"]), []).append(row)
    if len(grouped) != int(config["repetitions"]):
        raise ValueError("reference repetition count mismatch")
    for run in grouped.values():
        indices = [int(value) for row in run for value in row["query_indices"].split(";")]
        if sorted(indices) != list(range(len(evaluation))):
            raise ValueError("reference query IDs are not a permutation of held-out queries")
    args.output.mkdir(parents=True, exist_ok=False)
    config_out = dict(config, backend="chroma-persistent-native", package_version=chromadb.__version__,
                      reference=str(reference), dataset=str(dataset), platform=platform.platform(),
                      query_latency="python-collection-query-including-bindings-serialization",
                      diagnostics="Chroma-default-telemetry-disabled", transport="embedded-no-http",
                      ef_search=args.ef_search, num_threads=args.threads, recall_tuning="fixed-ef-no-adaptive-tuning",
                      host_rss_bytes="unavailable:no-process-measurement", scale_gate="untested-by-this-single-cell")
    config_out["algorithm"] = "Chroma-local-configured-HNSW; internal-exact-fallback-not-instrumented"
    config_out["input_sha256"] = {name: digest(reference / name) for name in
                                 ["configuration.txt", "metadata.csv", "truth.csv", "samples.csv"]}
    config_out["dataset_sha256"] = digest(dataset)
    config_out["python"] = sys.version
    native_dir = Path(chromadb_rust_bindings.__file__).parent
    config_out["native_binary_sha256"] = {path.name: digest(path) for path in native_dir.iterdir()
                                          if path.suffix in (".pyd", ".so", ".dll")}
    config_out["packages"] = {package.metadata["Name"]: package.version for package in importlib.metadata.distributions()}
    (args.output / "configuration.json").write_text(json.dumps(config_out, indent=2) + "\n")
    start = time.perf_counter_ns()
    client = chromadb.PersistentClient(path=str(args.output / "database"), settings=Settings(anonymized_telemetry=False))
    collection = client.create_collection("qenlo-replay", embedding_function=None,
        configuration={"hnsw": {"space": "cosine", "ef_construction": 200, "ef_search": args.ef_search,
                                "max_neighbors": 16, "num_threads": args.threads}})
    chunk = min(4096, client.get_max_batch_size())
    for start_row in range(0, len(corpus), chunk):
        end = min(start_row + chunk, len(corpus))
        collection.add(ids=[str(value) for value in range(start_row, end)], embeddings=corpus[start_row:end],
                       metadatas=[{key: row[key] for key in ("user_id", "timestamp_micros")} for row in metadata[start_row:end]])
    if collection.count() != len(corpus):
        raise ValueError("Chroma corpus row count mismatch")
    build_ns = time.perf_counter_ns() - start
    (args.output / "hnsw-configuration.json").write_text(json.dumps(collection.configuration, indent=2) + "\n")
    where = where_filter(config)
    def query(vectors):
        return collection.query(query_embeddings=vectors, where=where, n_results=10, include=["distances"])
    start = time.perf_counter_ns()
    tuning_recall = 0
    for index, vector in enumerate(tuning):
        tuning_recall += validate(query([vector]), [vector], corpus, metadata, config, [truth[("tuning", index)]])
    tuning_recall /= len(tuning)
    readiness_ns = time.perf_counter_ns() - start
    for index in range(int(config["warmup_queries"])):
        query([tuning[index % len(tuning)]])
    p95s, recalls, passed = [], [], tuning_recall >= float(config["recall_target"])
    with (args.output / "samples.csv").open("w", newline="") as sample_file, (args.output / "runs.csv").open("w", newline="") as run_file:
        sample_writer, run_writer = csv.writer(sample_file), csv.writer(run_file)
        sample_writer.writerow(["run", "batch_index", "query_indices", "query_count", "batch_latency_ns", "recall_at_10", "result_ids"])
        run_writer.writerow(["run", "batches", "queries", "p50_batch_ns", "p95_batch_ns", "p99_batch_ns", "wall_ns", "qps", "recall_at_10", "recall_target_passed"])
        for run, rows in grouped.items():
            latencies, total_recall = [], 0
            start = time.perf_counter_ns()
            for row in rows:
                indices = [int(value) for value in row["query_indices"].split(";")]
                vectors = evaluation[indices]
                call_started = time.perf_counter_ns()
                result = query(vectors)
                latency = time.perf_counter_ns() - call_started
                latencies.append(latency)
                recall = validate(result, vectors, corpus, metadata, config, [truth[("evaluation", index)] for index in indices])
                total_recall += recall * len(indices)
                sample_writer.writerow([run, row["batch_index"], row["query_indices"], len(indices), latency, recall, json.dumps(result["ids"])])
            sample_file.flush()
            wall = time.perf_counter_ns() - start
            recall = total_recall / len(evaluation)
            p95 = percentile(latencies, .95)
            p95s.append(p95)
            recalls.append(recall)
            passed &= recall >= float(config["recall_target"])
            run_writer.writerow([run, len(rows), len(evaluation), percentile(latencies, .5), p95,
                                 percentile(latencies, .99), wall, len(evaluation) * 1e9 / wall, recall,
                                 recall >= float(config["recall_target"])])
            run_file.flush()
    summary = dict(status="completed", build_ns=build_ns, readiness_and_tuning_ns=readiness_ns,
                   tuning_recall_at_10=tuning_recall, evaluation_recall_at_10=sum(recalls) / len(recalls),
                   recall_target_passed=passed, median_run_p95_batch_ns=percentile(p95s, .5),
                   min_run_p95_batch_ns=min(p95s), max_run_p95_batch_ns=max(p95s), filter_violations=0,
                   median_convention="lower-middle", scale_performance_claim="none")
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))
    if not passed:
        raise ValueError("recall target not met; raw measurements retained")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path, required=True, help="completed Qenlo cell directory")
    parser.add_argument("--output", type=Path, required=True, help="new directory; includes persistent database")
    parser.add_argument("--dataset", type=Path, help="override prepared dataset path without changing its checksum")
    parser.add_argument("--ef-search", type=int, default=128)
    parser.add_argument("--threads", type=int, default=1)
    replay(parser.parse_args())
