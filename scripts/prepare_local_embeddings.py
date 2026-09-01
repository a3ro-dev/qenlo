"""Create a pinned, locally inferred AG News embedding corpus as raw f32 rows."""

import argparse
import hashlib
import importlib.metadata
import json
import platform
import sys
import time
import zlib
from pathlib import Path

import numpy as np


def hashes(path: Path) -> dict[str, str]:
    sha256, crc = hashlib.sha256(), 0
    with path.open("rb") as source:
        while chunk := source.read(1 << 20):
            sha256.update(chunk)
            crc = zlib.crc32(chunk, crc)
    return {"sha256": sha256.hexdigest(), "crc32": f"{crc:08x}"}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--rows", type=int, default=100_000)
    parser.add_argument("--tuning", type=int, default=100)
    parser.add_argument("--evaluation", type=int, default=500)
    parser.add_argument("--batch-size", type=int, default=512)
    parser.add_argument("--model", default="sentence-transformers/all-MiniLM-L12-v2")
    parser.add_argument("--model-revision", default="a50ef00143b4d5391434df20ae11632588ac25be")
    parser.add_argument("--dataset", default="fancyzhx/ag_news")
    parser.add_argument("--dataset-revision", default="eb185aade064a813bc0b7f42de02595523103ca4")
    args = parser.parse_args()
    manifest = args.output.with_suffix(args.output.suffix + ".json")
    if args.output.exists() or manifest.exists():
        parser.error("output or manifest already exists")
    if min(args.rows, args.tuning, args.evaluation, args.batch_size) < 1:
        parser.error("split sizes and batch size must be positive")

    import torch
    from datasets import load_dataset
    from sentence_transformers import SentenceTransformer

    total = args.rows + args.tuning + args.evaluation
    started = time.time()
    dataset = load_dataset(args.dataset, split=f"train[:{total}]", revision=args.dataset_revision)
    if len(dataset) != total or "text" not in dataset.column_names:
        raise ValueError("pinned dataset did not provide the requested AG News text rows")
    model = SentenceTransformer(args.model, revision=args.model_revision, device="cuda")
    encoded_started = time.time()
    vectors = model.encode(
        dataset["text"], batch_size=args.batch_size, convert_to_numpy=True,
        normalize_embeddings=True, show_progress_bar=True,
    )
    encode_seconds = time.time() - encoded_started
    vectors = np.asarray(vectors, dtype="<f4", order="C")
    if vectors.shape != (total, 384) or not np.isfinite(vectors).all():
        raise ValueError(f"unexpected embedding matrix {vectors.shape}")
    norms = np.linalg.norm(vectors.astype(np.float64), axis=1)
    if np.any(norms == 0) or np.max(np.abs(norms - 1)) > 1e-4:
        raise ValueError("embeddings are not finite unit vectors")
    with args.output.open("xb") as target:
        target.write(vectors.tobytes(order="C"))
    report = {
        "format": "qenlo-local-embedding-source-v1",
        "dataset": args.dataset,
        "dataset_revision": args.dataset_revision,
        "dataset_fingerprint": getattr(dataset, "_fingerprint", None),
        "selection": f"train[0:{total}] in publisher order",
        "model": args.model,
        "model_revision": args.model_revision,
        "inference": "local CUDA; normalized float32 output; no remote embedding API",
        "rows": args.rows,
        "tuning": args.tuning,
        "evaluation": args.evaluation,
        "dimensions": 384,
        "batch_size": args.batch_size,
        "encode_seconds": encode_seconds,
        "total_seconds": time.time() - started,
        "output": str(args.output),
        "output_hashes": hashes(args.output),
        "python": sys.version,
        "platform": platform.platform(),
        "torch": torch.__version__,
        "cuda": torch.version.cuda,
        "gpu": torch.cuda.get_device_name(0),
        "packages": {name: importlib.metadata.version(name) for name in
                     ["datasets", "numpy", "sentence-transformers", "transformers"]},
    }
    manifest.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
