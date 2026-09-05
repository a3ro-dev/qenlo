# Small-collection paper

`paper.tex` is the final September 2026 manuscript. It replaces the earlier routing-centered draft with the verified 1k--100k campaign, the corrected 100k-by-768 supplement, resource accounting, mutation behavior, failures, and deployment limits.

The numerical source is `research/artifacts/runpod-small-2026-09-05/report/performance-matrix.csv`. Run `py -3 scripts/generate_small_paper_tables.py` after regenerating the matrix. The paper does not infer mobile performance from Runpod and does not claim a universally fastest database.

Build and verification commands are in [REPRODUCE.md](REPRODUCE.md).
