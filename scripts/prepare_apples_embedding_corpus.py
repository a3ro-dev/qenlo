#!/usr/bin/env python3
"""Generate one reproducible 1M-row local-embedding corpus and disjoint queries.

AG News has fewer than one million train rows.  This generator therefore makes
the expansion explicit by appending a deterministic row token before local
embedding.  It is suitable for controlled systems measurements, not a claim
about a naturally occurring million-document corpus.
"""
import argparse, hashlib, json, platform, sys, time, zlib
from pathlib import Path

import numpy as np


def digest(path):
    sha, crc = hashlib.sha256(), 0
    with open(path, "rb") as f:
        while b := f.read(1 << 20):
            sha.update(b); crc = zlib.crc32(b, crc)
    return {"sha256": sha.hexdigest(), "crc32": f"{crc:08x}"}


def encode_to(model, texts, path, batch):
    sha, crc, n = hashlib.sha256(), 0, 0
    with open(path, "xb") as f:
        for start in range(0, len(texts), batch):
            x = np.asarray(model.encode(texts[start:start + batch], batch_size=batch,
                convert_to_numpy=True, normalize_embeddings=True, show_progress_bar=False), dtype="<f4")
            if x.ndim != 2 or x.shape[1] != 768 or not np.isfinite(x).all():
                raise ValueError(f"invalid embedding chunk {x.shape}")
            b = x.tobytes(order="C"); f.write(b); sha.update(b); crc = zlib.crc32(b, crc); n += len(x)
    return n, {"sha256": sha.hexdigest(), "crc32": f"{crc:08x}"}


def main():
    p = argparse.ArgumentParser(); p.add_argument("--directory", type=Path, required=True)
    p.add_argument("--rows", type=int, default=1_000_000); p.add_argument("--queries", type=int, default=100)
    p.add_argument("--batch-size", type=int, default=256); p.add_argument("--seed", type=int, default=42)
    a = p.parse_args(); a.directory.mkdir(parents=True, exist_ok=False)
    if a.rows < 1 or a.queries < 1: raise ValueError("rows and queries must be positive")
    from datasets import load_dataset
    from huggingface_hub import model_info
    from sentence_transformers import SentenceTransformer
    import torch
    model_name = "sentence-transformers/all-mpnet-base-v2"
    revision = model_info(model_name).sha
    train = load_dataset("fancyzhx/ag_news", split="train", revision="eb185aade064a813bc0b7f42de02595523103ca4")
    test = load_dataset("fancyzhx/ag_news", split="test", revision="eb185aade064a813bc0b7f42de02595523103ca4")
    model = SentenceTransformer(model_name, revision=revision, device="cuda")
    started = time.time(); base = train["text"]
    corpus_text = [f"{base[i % len(base)]}\n[qenlo-row:{i}]" for i in range(a.rows)]
    query_text = [f"{t}\n[qenlo-query:{i}]" for i, t in enumerate(test["text"][:a.queries])]
    corpus = a.directory / "corpus.f32"; queries = a.directory / "queries.f32"
    count, c_hash = encode_to(model, corpus_text, corpus, a.batch_size)
    q_count, q_hash = encode_to(model, query_text, queries, a.batch_size)
    manifest = {"format":"qenlo-local-embedding-apples-v1", "model":model_name, "model_revision":revision,
      "dataset":"fancyzhx/ag_news", "dataset_revision":"eb185aade064a813bc0b7f42de02595523103ca4",
      "corpus_source":"train; deterministically augmented with row token because AG News has 120000 rows",
      "query_source":"disjoint test split; deterministically augmented with query token", "rows":count, "queries":q_count,
      "dimensions":768, "batch_size":a.batch_size, "seed":a.seed, "inference":"local CUDA only; normalized FP32",
      "corpus": {"path":str(corpus), "hashes":c_hash}, "queries_file":{"path":str(queries),"hashes":q_hash},
      "elapsed_seconds":time.time()-started,"python":sys.version,"platform":platform.platform(),"torch":torch.__version__,
      "cuda":torch.version.cuda,"gpu":torch.cuda.get_device_name(0)}
    (a.directory / "manifest.json").write_text(json.dumps(manifest,indent=2)+"\n")
    print(json.dumps(manifest, indent=2))

if __name__ == "__main__": main()
