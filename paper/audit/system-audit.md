# System-semantics audit

Audit date: 2026-09-05.  The implementation checkout was `b22d3b5033cfb578508a4cb76a8022dd3e3e258b`.  The repository's implementation note identifies the source baseline as `3e2a4a9bd82e130192907fc29eee6321269fb374` plus benchmark-harness changes; this audit treats the current Rust and Python sources as authoritative for semantics and uses the retained manuscript/archive only to identify wording that needs qualification.  No product source was changed.

The audit is bounded to seven contracts requested by the paper review: canonical FP32 normalization, CPU arithmetic, result ordering and ties, eligibility planning and stale validation, required-GPU/fallback behavior, mutations and persistence, and the Python `TorchIndex` snapshot.  Each conclusion is recorded in the machine-readable [system-ledger.json](system-ledger.json).  Line references below are source locations in the audited checkout, not claims that a historical benchmark used this exact revision.

## Findings

### Canonical representation and normalization

`qenlo-core` owns records as `Vec<f32>` with live state and public IDs (`crates/qenlo-core/src/lib.rs:218-229` and `1145-1154`).  `normalize_vector` rejects wrong dimensions and non-finite or zero-norm input, computes the norm in `f64`, divides in `f64`, and casts each normalized component back to `f32` (`crates/qenlo-core/src/lib.rs:1156-1178`).  `CoreStore::add` uses this function before adding a row.  Query entry points also normalize the query before scoring (`crates/qenlo-core/src/lib.rs:629-690`).

Restoration validates the stored FP32 bytes instead of renormalizing them.  It checks dimension, finiteness, nonzero norm, and unit norm within `1e-5` while preserving the bytes (`crates/qenlo-core/src/lib.rs:1181-1200`).  The defensible paper statement is therefore “canonical vectors and queries are FP32, normalized with an FP64 norm accumulator at ingestion/query boundaries.”  It is not “the store retains FP64 vectors,” and restoration is not an independent re-normalization pass.

### CPU arithmetic and exhaustiveness

The reference dot path returns `f64` (`crates/qenlo-core/src/lib.rs:927-948`); the scalar and SIMD CPU distance variants are typed as `f64` accumulation (`crates/qenlo-core/src/lib.rs:869-882`, `977-1143`).  Optimized CPU search uses a certified FP32 candidate score only for bounded candidate generation and re-scores the selected candidates with the FP64 path; collections over the certified row limit fall back to reference scoring (`crates/qenlo-core/src/lib.rs:738-821`).  The implementation therefore supports “exhaustive over eligible canonical rows” and “FP64 accumulation at the final CPU ranking boundary.”  It does not support calling every CPU arithmetic operation FP64: the certified candidate stage is FP32.

This distinction matters for the independent oracle.  The retained manuscript correctly says that native CPU/WGPU are exhaustive FP32-storage engines but need not be bit-identical to an FP64 oracle (`paper/audit/before/appendix.tex:39`, `paper/audit/before/head-paper.tex:40`).  That qualification should remain adjacent to any “exact” label.

### Ordering and ties

The host CPU `RankedHit` ordering is ascending distance followed by ascending ID (`crates/qenlo-core/src/lib.rs:841-864`).  The native GPU host merge applies the same distance/ID ordering, while the WGSL comparator uses FP32 distance and a 64-bit ID comparison (`crates/qenlo/src/gpu.rs:809-817`, `crates/qenlo/src/gpu_exact.wgsl:108-110`).  `TorchIndex` stores IDs in ascending order and uses stable sorting when the `topk` cutoff is tied (`sdk/python/src/qenlo/torch.py:73-75`, `159-168`).  This supports a deterministic implementation ordering.

It does not establish that all engines select the same member at an FP64-oracle tie.  The GPU and tensor paths compare FP32 distances, and CPU candidate generation can use FP32 before FP64 boundary scoring.  The old appendix wording “tied cutoffs accept only the deterministic members selected by that order” (`paper/audit/before/appendix.tex:37`) is too strong if read as an oracle guarantee.  The paper should say that each implementation has a deterministic `(distance, id)` order and that oracle qualification compares returned IDs/distances under the declared tolerance and tie policy.  A cross-engine exact tie theorem is not evidenced.

### EligibilityPlan, generation, and validation

`EligibilityPlan::compile` captures the current generation and corpus size, traverses the canonical predicate once by default, and records eligible count, predicate kind, representation, transfer bytes, contiguous runs, materialization time, traversal count, and cache state (`crates/qenlo/src/lib.rs:448-566`).  The legacy two-pass mode is retained explicitly for controlled ablations (`520-528`).  Cached host-row plans are reused only when both generation and exact filter match (`495-517`).  Representations are empty, tiny/sorted host rows, dense masks, or shader predicates; shader plans still retain rows for an automatic CPU decision or GPU fallback (`532-560`).

The plan is an execution artifact, not an independent source of truth.  Exact row execution revalidates ordering, bounds, liveness, and predicate membership before scoring; malformed or stale rows return an error (`crates/qenlo-core/src/lib.rs:663-735`).  Mutation increments the canonical generation and clears the prepared generation before attempting a resident update (`crates/qenlo/src/lib.rs:1527-1541`).  The evidence supports “generation-bound plans fail closed or are rebuilt.”  It does not support claiming that a plan can remain valid across arbitrary concurrent mutation without synchronization; the search path must be described at its observed generation boundary.

### Required GPU and automatic fallback

Backend construction distinguishes `CpuExact`, `WgpuRequired`, and `Automatic` (`crates/qenlo/src/lib.rs:60-68`, `1376-1404`).  Required WGPU initialization maps adapter/device failure to a preparation error.  Automatic initialization returns CPU plus a recorded fallback reason on the same failure.  Preparation binds resident state to the current generation and, in automatic mode, switches to CPU if GPU preparation fails (`crates/qenlo/src/lib.rs:1580-1634`).  Search errors similarly fall back only in automatic mode; required mode returns a search error (`crates/qenlo/src/lib.rs:1637-1732`).

The source supports the paper's scoped statement that required-GPU mode does not silently fall back and automatic mode reports fallback.  It does not imply that every device-loss path is transparent or recoverable: a search failure can leave the backend unhealthy and diagnostics unavailable (`crates/qenlo/src/lib.rs:1690-1717`).  Campaign unavailable and failed rows remain empirical evidence about the boundary, not counterexamples to the mode contract.

### Mutation, resident state, and durability scope

In-memory `add`/`delete` mutate the canonical store and then call `finish_mutation`; file-backed calls route through `commit` (`crates/qenlo/src/lib.rs:1407-1439`).  `commit` validates the complete ordered batch and performs storage admission before WAL append and canonical application (`1442-1511`).  The durable path writes a checksummed pending WAL, flushes and `sync_all`s it, renames it, syncs the directory where supported, and publishes a manifest (`crates/qenlo/src/storage.rs:260-333`).

The same `commit` API is used for in-memory collections, where there is no file-backed persistence or durable generation.  The implementation note explicitly limits the durability claim to process-crash recovery and declines universal power-loss durability (`research/implementation-notes.md:7-18`).  Thus the manuscript's “atomic batches publish only after validation and durable staging” (`paper/paper.tex:37`) must be scoped to file-backed/durable collections.  For an in-memory collection the validated application can be atomic with respect to the API operation, but it is not durable.  Likewise, “reopen reconstructs resident derived state from the durable canonical snapshot and WAL” is a durable-collection statement, not an in-memory property (`paper/audit/before/appendix.tex:42`).

`finish_mutation` first invalidates prepared-generation state, then attempts an exact in-place GPU update.  The update can append rows and change live masks subject to capacity/layout/chunk/budget limits; otherwise the next preparation rebuilds resident state (`crates/qenlo/src/gpu.rs:411-470`, `1139-1149`).  This supports the measured claim that the reported warm mutation cells avoided a rebuild, not a universal claim that mutations always update in place.

### TorchIndex snapshot and device scope

`TorchIndex` is an owned, immutable FP32 tensor view.  Its constructor accepts only device types `cpu`, `cuda`, and `mps`, checks CUDA/MPS availability, and copies/normalizes inputs on the selected device (`sdk/python/src/qenlo/torch.py:26-75`).  `from_collection` captures a filtered native snapshot and records its generation (`90-106`).  `search` checks the collection generation once before computation and raises `RuntimeError` after a canonical mutation (`124-168`).  The CPU conformance test covers tie ordering, ownership, stale rejection, validation, and an unavailable-CUDA branch (`sdk/python/tests/test_torch_index.py:1-74`).  The test file itself says GPU matrices run separately; no positive CUDA or MPS test is present in this checkout.

The supported-device claim is therefore code-level support for three PyTorch device names, not evidence of successful execution on every CUDA or MPS device.  The paper should distinguish “supports CPU/CUDA/MPS selection” from “tested on” the specific devices named in the campaign.  The source also does not implement a double-checked concurrent snapshot: it checks generation before the GEMM but does not hold the collection read lock or recheck after the computation.  Sequential stale rejection is supported; a guarantee that a concurrent mutation cannot overlap or invalidate an in-flight search is an overclaim.

## Prior-manuscript flags

1. **Durability scope (P1):** qualify `paper/paper.tex:37` and any inherited “durable batches” prose to file-backed collections.  Keep the process-crash/power-loss limitation from `research/implementation-notes.md:13`.
2. **Tie/oracle wording (P1):** qualify `paper/audit/before/appendix.tex:37`.  Deterministic engine ordering is supported; a universal FP64-oracle tie-membership guarantee is not.
3. **Concurrent snapshot wording (P1):** do not claim a double-checked or lock-spanning `TorchIndex` snapshot.  The implementation checks generation once at entry (`sdk/python/src/qenlo/torch.py:133-135`).
4. **Device evidence (P1):** keep the supported device-name list separate from positive hardware testing.  The current Python test suite is CPU-focused and only exercises unavailable CUDA behavior when CUDA is absent.
5. **Incremental mutation wording (P2):** phrase measured “zero rebuilds” as a result for the qualified cells.  Capacity, layout, chunk-count, budget, or device-health conditions can force a later full preparation.
6. **Exactness wording (P2):** use “exhaustive” for native CPU/WGPU candidate coverage and report oracle recall; avoid implying bit-identical FP64 arithmetic across CPU, WGPU, Torch, and external engines.

## Audit limits

This audit verifies source-level semantics and retained manuscript statements.  It does not re-run GPU backends, prove concurrent behavior, establish power-loss durability, or turn the Python device branches into positive platform evidence.  Those remain experiment or deployment questions and should stay in the paper's threats-to-validity section.
