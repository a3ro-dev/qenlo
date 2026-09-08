# Alpha.5 routing evidence

Status: candidate rejected by held-out evidence.

The analyzed 31 compatible CPU/GPU pairs compare the alpha.4 fallback with a static `eligible_rows * dimension * batch` threshold of 1,000,000 work units.

| Policy | Wrong routes | Median regret | Maximum regret |
| --- | ---: | ---: | ---: |
| Alpha.4 | 3 | 0.0% | 178.6% |
| Candidate | 0 | 0.0% | 0.0% |

The candidate passes the development-data gate: **true**. The held-out release decision is **rejected**: held-out alpha.5 confirmation rejected the candidate. See `../alpha5-router-heldout/report.md`. The candidate is not part of alpha.5 runtime behavior.
