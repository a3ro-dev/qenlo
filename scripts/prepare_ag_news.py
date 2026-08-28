"""Verify a DTU AG News HDF5 source and stream a deterministic f32 subset.

Requires h5py and numpy. The output is consumed by qenlo-bench prepare --input.
No embeddings are generated, projected, or normalized by this preparation step.
"""

import argparse
import hashlib
import json
from pathlib import Path
import zlib

import h5py
import numpy as np


SOURCES = {
    384: (37763697, "364dd01d4a383fd5a2628f9d4ea8279a", "all-MiniLM-L12-v2"),
    768: (40409750, "2f9902dfc67cd60a34c59432eaa0f4af", "multi-qa-distilbert-cos-v1"),
}


def hashes(path):
    md5, sha256, crc = hashlib.md5(), hashlib.sha256(), 0
    with path.open("rb") as source:
        while chunk := source.read(1 << 20):
            md5.update(chunk)
            sha256.update(chunk)
            crc = zlib.crc32(chunk, crc)
    return {"md5": md5.hexdigest(), "sha256": sha256.hexdigest(), "crc32": f"{crc:08x}"}


def extract(source, output, dimensions, count):
    with h5py.File(source, "r") as archive:
        candidates = []
        archive.visititems(lambda name, value: candidates.append(name)
                           if isinstance(value, h5py.Dataset)
                           and len(value.shape) == 2
                           and value.shape[1] == dimensions
                           and value.shape[0] >= count else None)
        if len(candidates) != 1:
            raise ValueError(f"expected one embedding dataset, found {candidates}")
        vectors = archive[candidates[0]]
        with output.open("xb") as target:
            for start in range(0, count, 1024):
                chunk = np.asarray(vectors[start:min(start + 1024, count)], dtype="<f4")
                if not np.isfinite(chunk).all() or np.any(np.linalg.norm(chunk.astype("f8"), axis=1) == 0):
                    raise ValueError("invalid vector; partial output retained for diagnosis")
                target.write(chunk.tobytes(order="C"))
        return {"hdf5_dataset": candidates[0], "source_shape": list(vectors.shape),
                "source_dtype": str(vectors.dtype)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--dimensions", type=int, choices=SOURCES, default=384)
    parser.add_argument("--rows", type=int, default=100000)
    parser.add_argument("--tuning", type=int, default=1000)
    parser.add_argument("--evaluation", type=int, default=5000)
    args = parser.parse_args()
    if min(args.rows, args.tuning, args.evaluation) <= 0:
        parser.error("all split counts must be positive")
    manifest = args.output.with_suffix(args.output.suffix + ".json")
    if args.output.exists() or manifest.exists():
        parser.error("output or provenance already exists; choose fresh paths")
    file_id, expected_md5, model = SOURCES[args.dimensions]
    source_hashes = hashes(args.input)
    if source_hashes["md5"] != expected_md5:
        raise ValueError("source differs from publisher's pinned MD5; refusing import")
    selection = extract(args.input, args.output, args.dimensions,
                        args.rows + args.tuning + args.evaluation)
    report = {
        "dataset": "DTU pretrained sentence BERT AG News embeddings v1",
        "doi": "10.11583/DTU.21286923.v1", "license": "CC BY 4.0",
        "source_url": f"https://ndownloader.figshare.com/files/{file_id}",
        "model": model, "source_hashes": source_hashes,
        "verification": "publisher MD5 matched; SHA256 additionally recorded locally",
        "dimensions": args.dimensions, **selection,
        "selection": "first consecutive source training rows; no shuffle or normalization",
        "corpus_range": [0, args.rows],
        "tuning_range": [args.rows, args.rows + args.tuning],
        "evaluation_range": [args.rows + args.tuning, args.rows + args.tuning + args.evaluation],
        "content_deduplication": "not performed; row intervals are disjoint",
        "output": str(args.output), "output_hashes": hashes(args.output),
        "h5py_version": h5py.__version__, "numpy_version": np.__version__,
    }
    with manifest.open("x", encoding="utf-8") as target:
        json.dump(report, target, indent=2)
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
