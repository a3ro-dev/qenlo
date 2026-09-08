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
