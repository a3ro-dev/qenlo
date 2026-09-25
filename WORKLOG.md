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

## alpha.8 shipped

- CI root cause #2: after regenerating the inventory, Linux still differed in
  45 places. `glob()` follows filesystem order (NTFS sorted, ext4 hashed), so
  `manifest_references` reordered. Fixed by sorting on the name string (Windows
  `Path` ordering is case-insensitive, POSIX is not). `portable correctness`
  fully green on `d1ed007`, first time since alpha.5. Tagged alpha.8 there.

## alpha.9

- Function browser (delegated to Sonnet, reviewed): 33/33 public `Collection`
  methods, `/` filter, drift test that re-parses `crates/qenlo/src/lib.rs`.
  Review found the 80x24 layout hid the summary and example entirely (8-row
  shortcut panel) and truncated names; fixed with an adaptive one-line panel and
  a `TestBackend` render test at 80x24. Every example identifier was checked
  against the source (`Filter` = `Predicate`, derives `Default`, etc.).
- Reliability scan (Sonnet, read-only) → four fixes, each with a test:
  closed collections served stale `get_record`/`scan_records`; FFI free ran
  `close()` outside `catch_unwind` and dropped its error; Python `__del__` was
  silent (now `ResourceWarning`, stdlib convention); TypeScript leaked handles
  and the directory lock without `close()` (now `FinalizationRegistry`).
- Deliberately not done: `scan_records(filter)` materializes every matching
  slot per page (O(matches) per page). Candidate for alpha.10 if measured.

## alpha.9 shipped

- Found while verifying alpha.8 assets: `SHA256SUMS` hashed itself mid-write,
  so `sha256sum -c` always failed one line. Fixed in `sdk-release.yml` before
  tagging alpha.9 (`5300b6c`). The alpha.8 asset digests all match GitHub's
  recorded digests; only the self-line is bogus.
- Line endings: Python `write_text` on Windows turned two LF `.rs` files into
  CRLF in `212eb52`; restored in `5300b6c`. Use byte writes from now on.

## alpha.10

- Landing page: the three hero cards had rendered ~2px wide on the live site
  since alpha.8. `.cards` is `container-type: size` and sizes cards from
  `100cqh`; with `flex: 1 1 auto`, the research section added below it took
  the height and `100cqh` resolved to 0 (a `min-height` alone did not fix it;
  a definite flex basis did). Verified 426px desktop, 361px tablet, 347px
  mobile in a browser. The dot-matrix glyph set lacked 2/3/4/8 (Sonnet caught
  it), so 4.6 would have drawn as 0.0.
- Hero numbers replaced: prototype 1M / 0.17 ms / 100% (unshipped PyTorch CUDA
  prototype that lost to FAISS in its own gate) → 4.6x, 235.7%, 1,833, each
  traced to a file. README restructured (install, examples, research table);
  wording fixed where the draft implied causation ("speed tracks power state")
  or overstated ("learned rule"). All 1,833 inventory entries are hashed, so
  "every file" is accurate.
- Web/desktop function browser: `GET /api/functions` from the same catalog;
  checked in a browser (`/` focuses search; "batch" → 3 of 33).
- Deferred with reason: sqlite-vec comparison (roadmap #2). Needs a release
  build of the bench and matched filter semantics; too heavy for a modest
  overnight run. Next step: add an `SqliteVec` backend to
  `scripts/oss_replay.py` (~25 lines) and run the 100K x 384 cell.

## GTM

- X and LinkedIn were unavailable until the user signed in mid-session.
  Posted on X (voice skill + short style): CI glob-order story and alpha.8.
  Drafts and a posted log live in `.claude/gtm-drafts.md` (untracked).
