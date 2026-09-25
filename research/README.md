# Research evidence

Qenlo keeps raw measurements, failures, manifests, and processed reductions in
the repository. Product behavior changes only when a stated gate passes.

## Alpha.5 automatic-routing evaluation

1. `python scripts/analyze_routing_evidence.py` reduces compatible retained
   CPU/GPU pairs and emits the development hypothesis in
   `data/processed/alpha5-routing/`.
2. `scripts/run_alpha5_router_gate.py` records the preregistered local suite and
   retains every summary, run, sample, truth file, and dataset recipe. The
   completed directory is archived as
   `data/raw/alpha5-router-heldout-rtx4050.tar.gz`. It now refuses to run against the
   released source because the measured candidate was intentionally reverted;
   this prevents accidentally relabeling alpha.4 routing as the candidate.
3. `python scripts/analyze_alpha5_router_gate.py` checks completion, recall,
   stable routing, median regret, and maximum regret. A nonzero exit is an
   expected rejected hypothesis, not missing evidence.
4. `python paper/scripts/inventory_evidence.py` rebuilds the archive inventory,
   including SHA-256 identities and exact-duplicate relationships.

The alpha.5 candidate failed its held-out maximum-regret limit and was reverted.
See `data/processed/alpha5-router-heldout/report.md`.

## Full archive reanalysis

Run `python research/scripts/analyze_full_archive.py` to verify and reduce the
previously contextual Phase 0 and Phase 2 archives together with the alpha.5
held-out router suite. The script does not rerun benchmarks or pool incompatible
latencies. It emits `data/processed/archive-reanalysis/`, including a hash-based
sample-series inventory and exact-duplicate disposition ledger.

The resulting conclusion is narrower and stronger than a search for one better
threshold: the retained cells falsify both a universal eligible-count threshold
and the preregistered `eligible_rows * dimension * batch` rule. A future router
must keep the workload factors and environment identity separate and pass a new
end-to-end held-out gate.

## Partial E0/E2 campaign (2026-09-24)

1. Verify `data/raw/runpod-e0-e2-partial-20260924.tar.gz` against SHA-256
   `e4ca1336757cf55a5f5150a8bcc601ab3538cb7e9e9c2270c137e5a72fd51ed0`.
2. `python scripts/audit_e0_e2_mechanisms.py` re-verifies coverage and exit
   codes from the tarball in place. It writes diagnostics, clock associations,
   and the ordering witness to `data/processed/runpod-e0-e2-mechanisms-20260924/`.
3. Latency statistics are in `data/processed/runpod-e0-e2-analysis-20260924/`.
   To regenerate them, extract the tarball with Python `tarfile` (GNU tar
   rejects its trailing bytes), then run `analyze.py --extracted-root <dir>/e0-e2`
   and `render_report.py`.

The campaign is partial and single-host. It motivates E1/E3/E4 (see
`direction-memo-2026-09-24.md`) and is not a basis for a routing threshold.
The audit fixes future measurement integrity (batch-total accounting and
zero-exit resume validation) and removes unused host row-list materialization
from required shader-predicate batches. These source changes do not rewrite the
archive and have no campaign-host speedup claim.
