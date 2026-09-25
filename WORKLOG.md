# work log: alpha.8 → alpha.10

Durable record of decisions, evidence, and next steps for the alpha.8–alpha.10
push. Newest entries last. Release mechanics are in `RELEASING.md`.

## state at start (2026-09-26)

- alpha.7 released (GitHub prerelease + crates.io, PyPI, npm, Maven).
  `research/power-states-paper` merged into `main` as PR #4 (`f0135e8`).
- `portable correctness / research-evidence` red on `main` since alpha.6:
  `paper/audit/archive-inventory.json` was not regenerated after the
  power-state archives landed. Lint, clippy, and workspace tests were green.
- Uncommitted alpha.8 work from the previous session: TUI `? Functions` tab
  (10 hard-coded entries), TypeScript `bigint` range validation, docs home.

## alpha.8

- Decision: ship the existing work plus the CI fix as one small release;
  defer the full function browser to alpha.9.
- TypeScript bug: `BigUint64Array.from([-1n])` stores `2^64-1`, so a negative
  ID was written as a different record ID. Now `RangeError` before any native
  call. Test: `sdk/typescript/test/qenlo.test.ts` (6/6 pass locally).
- CI fix: regenerated the inventory (1654 → 1833 files; deterministic across
  two runs). `bump_alpha.py` now regenerates it on every version bump.
- Local checks: `cargo fmt --check`, `cargo clippy -p qenlo-browser
  --all-targets -D warnings`, `cargo test -p qenlo-browser`, TS typecheck +
  tests, full `research-evidence` job steps (17 unit tests, alpha.5 gate
  reproduction, processed-data diff), `check_docs`, `check_release_versions`.
- Docs home verified rendering in a browser at `docs/#/`.

## candidate work (ranked)

1. alpha.9 function browser: catalog every public `Collection` method, group by
   task, type-to-filter, example per entry, and a test that fails if a public
   method is missing (drift guard).
2. alpha.10 README / landing hero: lead with what Qenlo is, the evidence map,
   and honest results (wins and failed bets), with links to raw data.
3. Surgical perf/reliability items found during review (see below).
