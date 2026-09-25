# Full archive reanalysis

This reduction verifies and reads retained evidence only. It does not rerun a benchmark. Incompatible cohorts remain separate, exact duplicate sample series are counted once, and sample-row counts describe archive coverage rather than statistical independence.

## Archive coverage

- 303 tracked or selected archive-internal sample files were inspected.
- 282 content-distinct sample series remain after removing 21 exact mirrors.
- Those unique files contain 1,826,130 CSV sample rows across 12 schemas.
- The four synthesis archives passed SHA-256 verification.

## New conclusions

1. **The preregistered scalar router failed held-out evaluation.** The 1,000,000-unit `E*D*B` threshold made no errors on 31 development pairs, then chose the wrong backend on 8 of 16 held-out workloads. Median choice regret was 0.4%, but maximum regret was 235.7%. The release gate correctly rejected it.
2. **Perfect monotone classification and acceptable regret are different claims.** At `k=10`, `d768-b16-e61-k10` favors GPU at 749,568 work units, while `d384-b1-e1953-k10` favors CPU at 749,952 units. That ordering contradicts every increasing threshold on the observed medians; near-equal work alone would not. All same-k witnesses depend on the former cell, whose CPU/GPU medians differ by only 3.16%. The post-hoc threshold 1,499,904.5 has one error and 3.16% maximum regret, below the original regret limits but fitted to the test set. This is not a proof that all scalar policies fail a practical gate, nor a causal identification of query-shape effects.
3. **Eligible count alone is also non-identifying across environments.** The RTX 4050 cohort reverses between 2K and 3K eligible rows; the Phase 0 RTX 4090 cohort reverses between 4K and 6K. At both 3K and 4K, the two cohorts have opposite winners. No universal monotone E-only threshold can classify all 14 retained matched cells. The best post-hoc minimax E threshold still incurs 17.4% maximum regret.
4. **CPU optimization must itself be routed.** The certified FP32 A40 path changes P95 by -17.0% at E=1K, -28.0% at E=4K, and 10.0% at E=100K. Dense single-thread FAISS CPU Flat is 26.9% lower-latency than the frozen Qenlo baseline while recording recall 0.99984 rather than 0.99998.
5. **Backend choice and realized automatic latency are different objects.** Across the held-out suite, automatic-route P95 is 0.412x to 5.393x the faster forced-route P95. Sequential run drift and differences in execution paths or state remain competing explanations; this range does not identify routing overhead.

## Resulting paper thesis

The retained campaign rejects the frozen E*D*B rule at its preregistered gate and contradicts perfect universal E-only classification across the named environments. Perfect increasing E*D*B classification also fails descriptively, but the small margin and post-hoc low-regret fit preclude a general impossibility claim. Separate E, D, B, k, total N, predicate/representation, source/device identity, residency and mutation state are candidate covariates, not individually established necessary predictors. A new frozen policy must be evaluated end to end on fresh held-out workloads; no positive calibrated-router advantage is validated.
