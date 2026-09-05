# Small-collection campaign audit

This audit covers `research/artifacts/runpod-small-2026-09-05` and the campaign reducer and replay scripts. It reads retained machine-readable and raw artifacts; it does not rerun a benchmark or alter a source archive.

The audit is reproducible with `python paper/scripts/verify_campaign_claims.py`. The verifier checks all 182 matrix rows against both CSV and JSON, all 8 retained result archives, 9 retained checksum manifests, and 1,053 checksum entries (4 deliberate empty-file self-check placeholders skipped). It also checks the corrected headline rows, lifecycle raw semantics, selector and Chroma pair counts, source bundle hashes, and baseline/candidate WGSL roles.

## Matrix accounting

The CSV and JSON matrices each contain 182 rows and agree on the checked fields. The verified status partition is 131 completed, 42 failed or unavailable, 7 failed, and 2 invalid-harness. There are 130 rows with `qualified=True`; the other 52 remain visible but are excluded from qualified timing comparisons.

The seven ordinary failures are three Chroma recall-gate failures and four 100k-by-768 Qenlo CPU failures from pre-fix or admission-retry archives. The two invalid-harness rows are the initial 768-dimensional CPU invocations in which internal workload labels were passed positionally to `qenlo-bench`; their logs say `expected option`, so no benchmark executed. The four pre-fix 100k-by-768 rows report collection-storage admission failure. Corrected rows are retained in a distinct source archive.

## Corrected 100k-by-768 supplement

The corrected archive is `deep768/rtx4090-eu-ro-secure-deep768-admission-fix/artifacts`. Its environment capture identifies an RTX 4090 on Vulkan, Linux x86_64, driver 580.159.04, CUDA 13.0, and an AMD Ryzen Threadripper 7960X host. It uses synthetic-uniform-v1 data from `r100000-d768.qnb` with dataset CRC32 `50b69280`.

For d4 (N=100000, D=768, B=1, k=1, unfiltered), the matrix reports P95 values in milliseconds of 9.409 (Qenlo CPU), 0.897 (Qenlo WGPU), 14.800 (FAISS Flat), 15.343 (NumPy), 18.695 (Torch CPU), and 0.671 (Torch CUDA). For d5 (same N and D, B=8, k=64, 10% corpus eligibility), the values are 28.599, 0.896, 3.055, 6.202, 2.936, and 0.384 ms respectively. All 12 corrected rows report recall 1.0 against their independent FP64 truth and zero native filter violations.

The native Qenlo completed-call boundary is `batch-call-completion`. Replay rows use `host query input through host-visible IDs and cosine distances`; they run over prefiltered canonical eligible rows, and their preparation cost is reported separately. This is a scoped exact-latency comparison, not an assertion that every engine performs the same predicate work.

For d5 WGPU, Qenlo-owned accelerator allocation is 314.909 MB and process RSS high-water mark is 1.174577 GB. The raw phase medians are 0.072 ms scoring, 0.613 ms selection, and 0.826 ms backend execution. For d4, allocation is 311.181 MB and process RSS high-water mark is 1.203675 GB. Qenlo allocation is attributable to Qenlo's allocator rather than physical VRAM; RSS is process-wide. They must remain separate in the paper.

## Selector comparison and source roles

Independent recomputation from matched matrix rows gives 12 qualified baseline-GPU/candidate pairs: 5 candidate wins and 7 losses. Ratios are baseline P95 divided by candidate P95, with the worst loss 0.617 on the RTX 4090 50k compound-filter cell and the largest win 1.295 on the A40. This rejects the lane-minimum candidate as an unconditional replacement.

The archive roles are easy to confuse. `baseline-source.tar.gz` has SHA-256 `f37e744c...` and contains the simple full-rescan WGSL selector. `current-source.tar.gz` has SHA-256 `bb195fed...` and contains the lane-minimum candidate used by the corrected/current experimental rows. The audited local `crates/qenlo/src/gpu_exact.wgsl` is byte-identical to the baseline WGSL. The paper should describe the candidate as a measured, rejected optimization and preserve the archive distinction instead of calling candidate measurements durable current behavior.

## Chroma comparison

There are seven qualified Qenlo-versus-Chroma pairs. Independent division of matrix P95 values gives Chroma/Qenlo ratios of 3.700, 25.018, 35.832, 305.944, 92.736, 446.397, and 3233.136. Three additional Chroma rows fail the predeclared recall gate: c3 and c6 at `ef_search=128`, and d2 at `ef_search=512`; they have no held-out timed result. Chroma's rows use a Python collection/query boundary that includes bindings and serialization, while Qenlo reports its native completed call. The comparison therefore supports the scoped sentence “Qenlo WGPU beat Chroma in these qualified paired cells,” and does not support a universal fastest-database claim.

## Lifecycle

The lifecycle raw file has three repetitions for each mutation phase and one reopen observation. Its declared boundary is “completed synchronous mutation then completed first search.” First-search P95 is 0.598 ms after add-one, 0.631 ms after delete-one, 0.740 ms after add-batch, and 0.819 ms after delete-batch. The separate mutation P95 fields are approximately 21.995, 22.003, 22.073, and 21.958 ms; these are not the search values. Every mutation row has `rebuilt=false`; all delete rows have `deleted_id_absent=true`. The retained harness times `Collection::open` separately at 52.169 ms, then starts a new timer for the first search, whose search-only value is 101.296 ms; the row records `rebuilt=true` once. The matrix lifecycle P95 is this search-only value, not an invented sum.

The only completed but unqualified row is USearch on d2 (`N=25000, D=384, B=64, k=10`): recall is 0.3453125 and P95 is 83.313304 ms. It remains visible but is excluded from qualified comparisons.

## Operational accounting

`campaign-summary.json` records final known account daily spend of `$0.9400778694252952`, starting from an account daily capture of `$0.9096019977878312`, with `billingDelayPossible=true`. The observed delta is `$0.030475871637464` and cannot be called a campaign-only cost because attribution and billing lag are not isolated. The campaign status is `finished`, with zero remaining campaign pods. The ledger contains 10 pod-created and 10 verified pod-deleted events, with no retained campaign volume. The configuration manifest has 15 entries: its first 10 are the primary offer target, while deep/reference/retry/fix entries are follow-ups. The paper's phrase “ten primary configurations were attempted” should be read in this scoped sense; it must not imply ten successful distinct GPU measurements. Unavailable offer records remain visible with their exact pod-create command and offer metadata.

## Claims to exclude or narrow

The campaign does not test Android or iOS devices, Metal or DX12 cloud hosts, power, concurrency, realistic metadata/vector correlation, or a held-out learned router. Three-repetition replay rows are too small for significance claims. Allocation and RSS fields cannot be combined into an efficiency score. The corrected summaries explicitly make no scale-performance claim. The completed but unqualified USearch row must remain visible as a negative correctness result. These limits are recorded in `paper/audit/campaign-ledger.json`.
