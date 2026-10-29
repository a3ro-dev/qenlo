# Research

Everything behind Qenlo's performance claims lives here: the plans we wrote
before measuring, the raw files that came back (failures included), the scripts
that reduce them, and the notes we kept along the way. The paper is a summary of
this folder, not a substitute for it.

If you only read one thing, read the [paper](../QENLO-RESEARCH-PAPER.pdf)
(*The Efficient Kernel Runs Slow*, source in [`paper/v2/`](../paper/v2/)). If you
want to check a number, start from the map below.

## What we found

| Finding | Boundary |
| --- | --- |
| The compact-row GPU path ran in lower power states. Its device time varied up to 4.6x across five RTX 4090 pod hosts and 4.2x across fresh laptop processes. The heavier path stayed within 1 to 7%. | Device timestamps for specific query cells. Not end-to-end latency and not a hardware ranking. |
| A small separate GPU load removed most of the light path's slow tail. | The predicted 10% end-to-end gain failed on every GPU. The extra load adds contention, so it is a test control, not a setting we recommend. |
| An automatic CPU/GPU routing rule failed its held-out gate: 235.7% maximum regret against a 25% limit. | The rule was reverted. This work does not show that power state moves the CPU/GPU crossover. |
| Qenlo's exact search beats several engines in specific cells and loses in others. | See [highlights](../docs/highlights.md) for both lists. |

## Map

| Folder | What is in it |
| --- | --- |
| [`experiments/`](experiments/) | Protocols written and hashed before any data existed: `dvfs-d1` (laptop), `dvfs-d1r` (RTX 4090 replication), `dvfs-d2` (API and batch replication), `dvfs-h100`, `dvfs-lock` (locked clocks), `dvfs-split` (SM vs memory clock). |
| [`data/raw/`](data/raw/) | What the machines returned, as checksummed archives. Large ones are split into `.partNN` files; rejoin with `split_archive.py --join`. |
| [`data/processed/`](data/processed/) | Reduced tables and reports computed from the raw archives. Small, and safe to read directly. |
| [`artifacts/`](artifacts/) | Campaign bundles from the RunPod and local runs: manifests, per-run CSVs, logs, billing records. |
| [`scripts/`](scripts/) | Runners, analysis and packing scripts. Each experiment's protocol names the script that ran it. |
| [`notes/`](notes/) | Working notes, audits and ledgers. Listed below. |
| [`audit/`](audit/) | The external red-team review of the paper, kept as received. |
| [`specs/`](specs/) | Experiments planned but not yet run. |
| [`literature/`](literature/) | Related-work notes. |
| [`runpod/`](runpod/) | The GPU host configurations we asked for. |

## Notes

Written while the work was happening, so they read like working notes. Where a
later note disagrees with an earlier one, the later one wins.

| Note | What it covers |
| --- | --- |
| [`decision-memo-2026-09-25`](notes/decision-memo-2026-09-25.md) | Why the paper was rebuilt around GPU power states. Supersedes the direction memo. |
| [`direction-memo-2026-09-24`](notes/direction-memo-2026-09-24.md) | Where the partial E0/E2 campaign left things and what to run next. |
| [`analysis`](notes/analysis.md) | A one-page synthesis of what the retained evidence does and does not support. |
| [`sota-claim-matrix`](notes/sota-claim-matrix.md) | Each "fastest" or "state of the art" claim, with a verdict from retained evidence. |
| [`claim-artifact-ledger`](notes/claim-artifact-ledger.md) | Which file backs which claim in the first paper. |
| [`methodology-audit`](notes/methodology-audit.md) | What the harness checks well, and which runs are context only. |
| [`completion-audit`](notes/completion-audit.md) | Each promised deliverable and its status. |
| [`phase0-implementation-audit`](notes/phase0-implementation-audit.md) | Audit of the code at the frozen baseline revision. |
| [`phase1-eligibility-report`](notes/phase1-eligibility-report.md) | Measurements of the eligibility plan. |
| [`phase2-cpu-exact-progress`](notes/phase2-cpu-exact-progress.md) | CPU exact-search numerics and the FAISS baseline. |
| [`cpu-certified-any-e-and-threads`](notes/cpu-certified-any-e-and-threads.md) | The beta.2 change that removed the 4,096-row CPU cliff. |
| [`implementation-notes`](notes/implementation-notes.md) | What the code does, with its durability limits. |
| [`experiment-plan`](notes/experiment-plan.md), [`experiment-log`](notes/experiment-log.md) | The September campaign plan and a dated log of commands run. |
| [`long-horizon-ledger`](notes/long-horizon-ledger.md) | A running ledger of research steps from 2026-09-17. |

## Checking things

- Every retained file is hashed. CI regenerates the inventory in
  [`paper/audit/archive-inventory.json`](../paper/audit/archive-inventory.json)
  and fails if it differs.
- CI also reruns the alpha.5 routing analysis and checks that the held-out gate
  still rejects the rule.
- [`REPRODUCE.md`](REPRODUCE.md) has the commands for re-deriving the reduced
  tables from the raw archives.
- A failed run is kept next to the good ones and says why it failed. If you find
  a number with no file behind it, that is a bug; please open an issue.
