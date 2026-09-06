# Qenlo research paper source

There is exactly one published manuscript: [QENLO-RESEARCH-PAPER.pdf](../QENLO-RESEARCH-PAPER.pdf) in the repository root. It is the verified 22-page final paper and integrates the September 5 small-collection campaign with the historical routing, eligibility preparation, Android, Intel Arc, A6000, real-embedding, and external-engine evidence.

This directory contains its reproducible source and audit evidence, not alternative papers. Generated build directories live under the ignored `paper/tmp/` path. Superseded manuscript PDFs were removed; git history retains them if historical comparison is ever needed.

The thesis is conditional execution: eligible work, preparation, predicate representation, batching, selection, runtime ownership, and mutation/residency state matter alongside collection size and device class. The archive supports this engineering conclusion; it does not validate a universal threshold or an adaptive router.

## Important audit corrections

- `current-gpu` is a matrix role. The S1/S2 archives contain the lane-minimum candidate; the local shader retains the simpler selector. The corrected 0.897/0.896 ms results are experimental-archive measurements, not verified final-local-selector performance.
- There are 182 matrix rows: 131 completed, 42 failed or unavailable, seven failed, and two invalid-harness. Only 130 are qualified; the completed USearch D2 row fails recall.
- Lifecycle search latency excludes mutation. Reopen's 52.169 ms opening and 101.296 ms first search are separate timers.
- External flat/tensor replays prefilter their matrices outside query timing. Their build field also omits the initial metadata traversal.
- Final USD 0.9400778694252952 is captured account daily spend, not independently attributable campaign cost.
- Historical endpoint revisions differ. The controlled localization sweep remains separate.

## Contents

- `paper.tex`, `appendix.tex`, `references.bib`: canonical manuscript source, detailed evidence, and audited citations.
- `tables/claim-to-artifact.json`: combined full ledger; CSV provides an index.
- `audit/`: historical, campaign, citation, and source-contract audits; verifier results and preserved pre-edit manuscripts.
- `figures/final/`: twelve generated PDF/PNG figure pairs used by the paper. Original top-level figures are retained as research evidence.
- `scripts/`: paper-only reduction, verification, table/figure generation, ledger, and isolated-build helpers.
- `audit/figure-sources.json`: each generator's exact input paths and hashes.
- `audit/build-verification.json` and `audit/verification-final.json`: build, text, and visual verification records.

See [REPRODUCE.md](REPRODUCE.md) for the complete offline authoring workflow. It does not run benchmarks, modify product code, provision infrastructure, or overwrite retained research artifacts. Prior manuscript sources are preserved in `audit/before/`; prior PDFs and historical figures remain untouched.
