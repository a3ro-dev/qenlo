# Qenlo research paper source

**Current paper:** [QENLO-RESEARCH-PAPER.pdf](../QENLO-RESEARCH-PAPER.pdf). Its source is in [`v2/`](v2/): run `bash paper/v2/build.sh`; tables and figures come from `paper/v2/scripts/`. The paper is *The Efficient Kernel Runs Slow: GPU Power States Decide the Latency of Light Vector-Search Queries*. Its thesis, evidence and rejected alternatives are in [`research/decision-memo-2026-09-25.md`](../research/decision-memo-2026-09-25.md). The next experiments, a locked-clock A/B and FAISS as a second engine, are specified in [`research/specs/2026-09-25-clock-lock-and-faiss.md`](../research/specs/2026-09-25-clock-lock-and-faiss.md).

The rest of this file describes the **preserved v1 evidence audit** ([`archive/qenlo-evidence-audit-v1.pdf`](archive/qenlo-evidence-audit-v1.pdf), source `paper.tex`/`appendix.tex`). It is kept for provenance; the new paper cites it for the held-out router result.

## v1 evidence audit

The v1 manuscript integrates the September 5 small-collection campaign with the historical routing, eligibility preparation, Android, Intel Arc, A6000, real-embedding, external-engine, Phase 0/2, and alpha.5 held-out routing evidence.

This directory contains its reproducible source and audit evidence, not alternative papers. Generated build directories live under the ignored `paper/tmp/` path. Superseded manuscript PDFs were removed; git history retains them if historical comparison is ever needed.

The September 24 revision adds the partial E0/E2 RTX 4090 campaign (227 of 260 planned block summaries; not complete) and corrects batch transfer counters that the benchmark harness overstated B-fold. The subtitle is now "Why Scalar Routing Rules Fail and What a Router Must Measure": representation, host preparation, and same-cell noise are part of the routing problem. See `research/direction-memo-2026-09-24.md`.

The thesis is conditional execution and a held-out negative result: the frozen `E*D*B=1,000,000` policy failed its regret gate, and equal eligible counts have opposite winners across named environments. A post-hoc scalar fit has 3.16% maximum regret; it is not held-out validation, but prevents claiming that every scalar policy fails practical limits. Separate workload/state factors motivate research, not a proven multidimensional production router. See the [scoped claim matrix](../research/sota-claim-matrix.md) and [current related work](../research/related-work-current.md).

## Important audit corrections

- `current-gpu` is a matrix role. The S1/S2 archives contain the lane-minimum candidate; the local shader retains the simpler selector. The corrected 0.897/0.896 ms results are experimental-archive measurements, not verified final-local-selector performance.
- There are 182 matrix rows: 131 completed, 42 failed or unavailable, seven failed, and two invalid-harness. Only 130 are qualified; the completed USearch D2 row fails recall.
- Lifecycle search latency excludes mutation. Reopen's 52.169 ms opening and 101.296 ms first search are separate timers.
- External flat/tensor replays prefilter their matrices outside query timing. Their build field also omits the initial metadata traversal.
- Final USD 0.9400778694252952 is captured account daily spend, not independently attributable campaign cost.
- Historical endpoint revisions differ. The controlled localization sweep remains separate.
- The S2 batch-eight upload/readback fields (517,120 / 65,792 bytes) are eight times the per-call transfer (64,640 / 8,224 bytes). The harness summed batch totals repeated on every response; it now records them once (run format v4).
- E0/E2 is partial: 227/260 summaries, no COMPLETE marker, 33 missing batch-16 slots, one included exit-101 block, one summary-less CPU destination, and 20 summaries at recall@10 exactly 0.99998.
- Post-audit code changes do not alter routing: batch counters are recorded once, broad timestamp ranges can use a bounded-probe scan, required shader predicates no longer materialize unused host rows, and resumes require a zero-exit run record. None of the reported E0/E2 latencies measure these changes.

## Contents

- `paper.tex`, `appendix.tex`, `references.bib`: canonical manuscript source, detailed evidence, and audited citations.
- `../research/scripts/analyze_full_archive.py`: verifies four retained archives, reduces previously underused Phase 0/2 and held-out router evidence, and inventories sample series with exact-duplicate handling.
- `tables/claim-to-artifact.json`: combined full ledger; CSV provides an index.
- `audit/`: historical, campaign, citation, and source-contract audits; verifier results and preserved pre-edit manuscripts.
- `figures/final/`: fourteen generated PDF/PNG figure pairs used by the paper. Original top-level figures are retained as research evidence.
- `scripts/`: paper-only reduction, verification, table/figure generation, ledger, and isolated-build helpers.
- `audit/figure-sources.json`: each generator's exact input paths and hashes.
- `audit/build-verification.json` and `audit/verification-final.json`: build, text, and visual verification records.

See [REPRODUCE.md](REPRODUCE.md) for the complete offline authoring workflow. It does not run benchmarks, modify product code, provision infrastructure, or overwrite retained research artifacts. Prior manuscript sources are preserved in `audit/before/`; prior PDFs and historical figures remain untouched.
