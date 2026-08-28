# Native Windows Chroma 1.5.9 replay

Exploratory measurements on the same RTX 4050 laptop as `../gpu-tuning`.
These are synthetic uniform vectors and synthetic independent metadata, not
real embeddings, the full benchmark protocol, or the 1m x 768 gate.

| Cell | Chroma configuration | Tuning / evaluation recall@10 | Median run P95 |
|---|---|---|---|
| 2,000 x 32, user 0 AND bounded time range, 20 eligible | ef=128 at creation | 1 / 1 | 2.5812 ms |
| 100,000 x 384, all eligible | ef=8192 at creation | 1 / 0.99 | 21.5418 ms |

The 100k cell has only **4 tuning queries, 20 evaluation queries, 4 warmups and
2 repetitions**. Its two P95s are 27.0829 and 21.5418 ms; the lower-middle median
is used. The identical GPU reference's P95 is 3.0908 ms, and the CPU exact
baseline's is 34.1576 ms. This is a noisy exploratory sample, not a production
tail estimate or an algorithm superiority claim. Chroma includes Python API
validation, bindings, serialization and local database execution, with no HTTP;
Qenlo uses its Rust API. Chroma's internal exact fallback is not instrumented.

All accepted runs returned the expected number of unique eligible IDs and passed
independent f64 cosine-score checks. Recall uses the exported Qenlo exhaustive
eligible-subset oracle. The 100k Chroma replay uses exactly the query order,
vectors and filter of `../gpu-tuning/gpu-parallel-large-100k384-all`.

The compound smoke has 8 tuning queries, 32 evaluation queries, 8 warmups,
3 repetitions, and bounded timestamp filter `user_id=0 AND -1000<=timestamp<-980`.
Its CPU reference returned recall 1 and P95 0.002 ms; the tiny corpus emphasizes
different API overheads and says nothing about GPU acceleration.

## Failed configuration experiment retained

`invalid-runtime-ef-sweep` is **not an accepted baseline**. We tried setting
`collection.modify(configuration={"hnsw": {"ef_search": ...}})` after the
index was built. The configuration reported every new value, but all ten
settings from 128 to 100000 returned identical tuning recall 0.25 and similar
latency; held-out recall was 0.155. This suggests the warm native index did not
apply those updates. We removed that tuning path and rebuilt with a fixed
creation-time ef=8192, which returned 0.99 held-out recall. The latter was chosen
after inspecting only the unsuccessful tuning recalls; no search over successful
held-out results was performed. It is not established as the fastest qualifying
Chroma setting. All failed samples and configuration are retained for diagnosis.

## Reproduce and provenance

See `../../../scripts/chroma-replay.md` for pinned installation and replay.
The 100k vector file is reproduced with `qenlo-bench prepare --rows 100k
--dimensions 384 --tuning 4 --evaluation 20 --seed 42`; CRC32 is `8b058757`.
The smoke is prepared with `--rows 2000 --dimensions 32 --tuning 8
--evaluation 32 --seed 42`; CRC32 is `46092fce`.

Dataset and database payloads are not committed. Reference metadata are exported
by the Rust benchmark; the 100k metadata are deterministic from the supplied
seed and distribution, and their SHA-256 is recorded in Chroma's configuration.
The compound smoke reference includes its small metadata/truth exports.

Chroma configuration JSON records full installed package versions, the native
63 MB binary SHA-256, input-file SHA-256, prepared-dataset SHA-256, Python version,
and actual HNSW configuration. `git_revision` inside it refers to the **Qenlo
reference run**, not the adapter source. The fixed-8192 adapter is commit
`daf2116`; its source hash is also recorded. The failed runtime sweep is adapter
commit `59b028a`. The first smoke was run during development of `78c6c5c`.
No physical VRAM or process RSS measurement is claimed.
