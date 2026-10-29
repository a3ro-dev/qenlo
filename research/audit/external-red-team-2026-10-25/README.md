# External red-team audit of the paper, 2026-10-25

Notes from an independent review of the Qenlo paper and repository, run in
claude.ai on 2026-10-25. They are kept as received: critiques, citation checks,
recomputation scripts, and one proposed patch. Nothing here has been edited.

Applied in this repository:

- `6c5e7cd` planner 4N-bytes fix and heater duty log (the proposed patch)
- `810a4a0` SHA256SUMS for the DVFS archives; runner logs un-ignored
- `297417f` `busy_fraction.py`
- `b3fe43f`, `f569058`, `af37bac` paper v2 citation, number, scope and novelty fixes

Not done when the notes were applied: a stronger CPU baseline past E=4096
(later shipped in 0.1.0-beta.2), a fused WGSL top-k, new laptop runs (driver
power-mode arm, inter-query delay sweep, end-to-end under a clock lock), and CI
table regeneration.

Start with `repo_provenance_and_data.md` and `manuscript_numbers_stats_causal.md`.
