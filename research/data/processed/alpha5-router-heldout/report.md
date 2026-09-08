# Alpha.5 held-out routing gate

Status: **rejected**.

The preregistered RTX 4050 / DX12 suite measured 16 workloads after the 1,000,000-work-unit rule was selected from older evidence. Recall and filter correctness passed: **true**.

| Result | Value | Release limit |
| --- | ---: | ---: |
| Wrong routes | 8 / 16 | descriptive |
| Median routing regret | 0.4% | <= 10% |
| Maximum routing regret | 235.7% | <= 25% |

The candidate is not shipped. The held-out data shows that a single `eligible_rows * dimension * batch` threshold does not generalize across query shapes and `k`; Qenlo retains the alpha.4 static fallback and its per-device tuning-profile override. Raw summaries, samples, run order, diagnostics, and the deterministic dataset recipe are retained in `research/data/raw/alpha5-router-heldout-rtx4050.tar.gz`.
