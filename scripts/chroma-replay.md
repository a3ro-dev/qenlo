# Native Chroma replay

`chroma_replay.py` compares Chroma **1.5.9 PersistentClient on native Windows**
with a completed `qenlo-bench` run. It reads the same CRC-checked QNLOB001 vectors,
the Rust-generated synthetic metadata, exhaustive f64 oracle IDs, and each run's
exact shuffled query order. There is no embedding model or HTTP transport.

## Install and run

From the repository root (the D: paths keep packages and caches off C:):

```powershell
$env:UV_CACHE_DIR='D:\qenloDB\target\uv-cache'
$env:UV_PYTHON_INSTALL_DIR='D:\qenloDB\target\uv-python'
uv venv --python 3.12 target/chroma-venv
uv pip install --python target/chroma-venv/Scripts/python.exe -r scripts/chroma-requirements.txt
cargo build --release -p qenlo-bench --features usearch,gpu-wgpu
target/release/qenlo-bench.exe prepare --dataset target/example.qnb --rows 2000 --dimensions 32 --tuning 8 --evaluation 32
target/release/qenlo-bench.exe run --dataset target/example.qnb --dimensions 32 --output target/example-cpu --backend cpu --user-id 0 --fraction 1
target/chroma-venv/Scripts/python.exe scripts/chroma_replay.py --reference target/example-cpu --output target/example-chroma --ef-search 128
```

Every output directory and dataset path must be new. Chroma's output directory
contains its persistent database; don't commit that database. Keep the JSON and
CSV artifacts, which include all samples, per-run nearest-rank percentiles, recall,
actual HNSW configuration, full package versions, native library SHA-256, and
input artifact SHA-256. The dependency lock records the smoke environment;
other platforms may need a different supported Python/package combination.

`--user-id 0` adds user equality AND inclusive-lower/exclusive-upper timestamps.
With this option `--fraction` is relative to that user's population, **not** the
whole corpus; the actual eligible count/fraction is recorded. On independent
metadata, user 0 with fraction 1 selects 1% of a corpus divisible by 100. Without
`--user-id`, fraction applies to the whole corpus, as before. Metadata is synthetic
even when vector embeddings were imported from a real dataset.

## Real-data and scale commands

First prepare the selected source with `qenlo-bench prepare --input RAW_F32_LE
--expect-crc32 HEX`, preserving the publisher checksum and disjoint split recipe.
Then run each backend independently with identical options:

```powershell
target/release/qenlo-bench.exe run --dataset data/ag-news/ag-news-100k-384.qnb --dimensions 384 --output target/ag-cpu --backend cpu --user-id 0 --fraction 1 --warmups 200 --repetitions 5
target/release/qenlo-bench.exe run --dataset data/ag-news/ag-news-100k-384.qnb --dimensions 384 --output target/ag-usearch --backend usearch --user-id 0 --fraction 1 --warmups 200 --repetitions 5 --tune-expansion-search 128,256,512,1024,2048,4096,8192
target/release/qenlo-bench.exe run --dataset data/ag-news/ag-news-100k-384.qnb --dimensions 384 --output target/ag-gpu --backend gpu-rows --user-id 0 --fraction 1 --warmups 200 --repetitions 5
target/chroma-venv/Scripts/python.exe scripts/chroma_replay.py --reference target/ag-cpu --output target/ag-chroma --ef-search 128
```

Use a prepared dataset containing 1,000 tuning and 5,000 evaluation queries for
the full protocol; the adapter uses the reference's splits and timing protocol.
Repeat GPU runs with `gpu-mask` and `gpu-predicate`; use `--batch 8` or `--batch 32`
for shared-filter batch measurements. Distinct-filter batches are not implemented.
Chroma uses fixed `--ef-search` at index creation, with tuning-query recall retained
in `tuning.csv`. A failed tuning target writes `tuning-failure.json` and aborts
before any held-out query; retry with a fresh index and a larger fixed expansion.
An attempted runtime parameter sweep in native 1.5.9 reported
updated configuration but unchanged query results across expansions 128..100000;
do not rely on modifying a warm index for tuning. Use independent index builds
and tuning experiments to select Chroma expansion before held-out evaluation.
Qenlo's `--tune-expansion-search` sorts/deduplicates the supplied USearch grid,
selects the smallest expansion reaching tuning recall, records each attempt in
`tuning.csv` and `expansion_search_effective` in the manifest, then evaluates
held-out queries once. No passing candidate aborts before held-out evaluation.
GPU runs also record the selected adapter, API and negotiated limits. Recall must
pass in every cell. Repeat at `--recall-target 0.99`; failed runs retain samples.

On the 2026-08-28 hybrid Windows host, the available adapters were Intel UHD
Graphics (integrated) and NVIDIA GeForce RTX 4050 Laptop GPU (discrete). The
GPU measurements requested the high-performance adapter and must be interpreted
from the manifest's `gpu_adapter`, `gpu_device_type`, and `gpu_api` fields rather
than from the presence of a GPU feature flag alone. The retained real-data
[result record](../docs/results-2026-08-28.md) uses the NVIDIA adapter.

The predeclared gate remains **1,000,000 x 768, batch 1, 1% eligible, k=10**: these commands
do not establish it; the current real-data cells are 100,000 x 384. For that run prepare a separate 1m768 dataset and set explicit
vector/GPU budgets only if hardware memory can accommodate the measured allocation.

## Interpretation and checks

For repeated Rust backends on the same cell, `--oracle-reference EXACT_CPU_DIR`
reuses a completed exact-CPU run's independent truth. The importer checks dataset
CRC, shape and query splits, metadata distribution and every metadata row, actual
user/time filter, oracle query coverage, result counts, uniqueness and eligibility.
It requires source recall 1 and zero violations. Source truth content is trusted
as a prior exact computation, not recomputed; its CRC and path are recorded.

Recall threshold comparisons allow `1e-12` solely for floating-point accumulation
roundoff (for example 0.9899999999999999 versus 0.99); raw recalls are not rounded.

Query latency includes the completed API call. Chroma includes Python input
validation, bindings, serialization, local database execution and result conversion;
Qenlo's Rust API has a different boundary. A large difference on a tiny corpus can
be dominated by those costs and is **not evidence of a better ANN algorithm or GPU**.
The retained 100k × 384 native replay and the matching Qenlo CPU/GPU cells are
summarized in [the 2026-08-28 result record](../docs/results-2026-08-28.md).
Both measured QPS windows also include driver correctness validation and CSV writes.
Build time, readiness/tuning, and warmups are excluded from steady-state query
latency. Host RSS and physical VRAM are not measured by this adapter.

Validation rejects duplicate, out-of-range and filter-violating IDs, nonfinite or
unordered distances, and cosine scores differing from independent f64 scoring by
more than `1e-5`. Recall uses the exported exhaustive full-eligible-subset truth.
Equal-distance Chroma IDs need not be ordered numerically; ties may affect recall.
The adapter supports metadata representable as Chroma signed integers; the
generated benchmark IDs/timestamps satisfy this.

```powershell
cargo test -p qenlo-bench --bin qenlo-bench
target/chroma-venv/Scripts/python.exe -m unittest discover -s scripts -p test_chroma_replay.py
```

API documentation: [collection configuration](https://docs.trychroma.com/docs/collections/configure),
[metadata filters](https://docs.trychroma.com/docs/querying-collections/metadata-filtering),
and [Python collection API](https://docs.trychroma.com/reference/python/collection).
Context7 documentation was checked and the API was exercised against pinned 1.5.9.
