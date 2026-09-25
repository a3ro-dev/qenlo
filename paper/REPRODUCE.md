# Reproducing the definitive Qenlo paper

Run these commands from the repository root. They read retained artifacts and write only paper authoring or verification outputs. Do not rerun a paid experiment or a product benchmark to reproduce this manuscript.

## Dependencies

The audited local build uses Python 3.14 with NumPy, pandas, matplotlib, Pillow, and pypdf; MiKTeX pdfLaTeX/BibTeX; and Poppler `pdftoppm`, `pdftotext`, and `pdfinfo`. Standard LaTeX packages include geometry, amsmath, amssymb, booktabs, float, graphicx, microtype, array, longtable, url/xurl, xcolor, hyperref, and natbib. The bundled desktop Python is an alternative only if these plotting dependencies are installed there. The scripts are analysis/authoring tools, not library performance tests.

## Audit retained evidence

```powershell
python paper/scripts/verify_campaign_claims.py
python paper/audit/verify_historical_raw.py
python paper/scripts/reduce_final_evidence.py
python paper/scripts/inventory_evidence.py
python research/scripts/analyze_full_archive.py
```

The campaign verifier checks CSV/JSON equality, raw corrected timings and phases, lifecycle timers, source roles, all retained result archives/checksum manifests, and qualified pair counts. Expected results: 182 rows, eight result archives, nine checksum manifests, 1,053 checked entries, four self-manifest entries skipped, 12 selector pairs (5 wins/7 losses), seven qualified Chroma pairs, and one completed-unqualified USearch row. These are integrity and descriptive checks, not significance tests.

The reducer writes to `paper/audit/reduced/` and checks parsed CSV equality against retained originals. Android is reproduced from supplied aggregate records; unavailable raw distributions cannot be recreated. Historical endpoint and localization source revisions are kept distinct. Duplicate archive mirrors are counted once.

The archive reanalysis verifies the SHA-256 identities of the Phase 0, Phase 1, Phase 2, and alpha.5 held-out archives; parses them in place; and writes only to `research/data/processed/archive-reanalysis/`. It inventories tracked and selected archive-internal sample, raw-sample, and lifecycle CSVs, canonicalizes line endings for duplicate detection, and counts sample rows by schema without treating unlike rows as independent trials. It regenerates the held-out router failure, cross-environment winner contradiction, and sparse/dense CPU optimization reductions.

## Audit the partial E0/E2 campaign

```powershell
python research/scripts/audit_e0_e2_mechanisms.py
python -c "import tarfile; tarfile.open('research/data/raw/runpod-e0-e2-partial-20260924.tar.gz').extractall('paper/tmp/e0e2', filter='data')"
python research/data/processed/runpod-e0-e2-analysis-20260924/analyze.py --extracted-root paper/tmp/e0e2/e0-e2
python research/data/processed/runpod-e0-e2-analysis-20260924/render_report.py
```

The first command verifies SHA-256 `e4ca1336757cf55a5f5150a8bcc601ab3538cb7e9e9c2270c137e5a72fd51ed0` and reads the tarball in place. Expected results: 227 of 260 summaries, no completion marker, one summary-less destination (`e2/16/30000/1/cpu`), one nonzero exit with a summary (`e2/1/100/2/gpu-mask`: 101), and recall@10 values of 1 (207) and 0.99998 (20). Use Python `tarfile` for extraction: GNU tar stops at the archive's 6,575 trailing non-gzip bytes. Extraction goes to the ignored `paper/tmp/`; the retained archive is never modified. Bootstrap endpoints depend on the seed in the third decimal, so compare against the retained CSVs rather than re-deriving them with another seed.

## Regenerate numerical tables, figures, and ledger

```powershell
python paper/scripts/generate_final_tables.py
python paper/scripts/generate_e0e2_tables.py
python paper/scripts/generate_final_figures.py
python paper/scripts/assemble_claim_ledger.py
```

The numerical tables use the verified campaign matrix and regenerated historical CSVs. The figure script reuses historical plotting functions with output redirected into `paper/figures/final/`; the native crossover panel deliberately omits incompatible older endpoint revisions. Existing top-level figures are never overwritten. It writes PDF and reviewable PNG for all fourteen current figures, including the E0/E2 noise and representation panel, and records exact input paths/hashes in `paper/audit/figure-sources.json`.

The full claim ledger embeds historical, campaign, semantic-source, and citation components and all 182 matrix rows. Missing information is not silently filled with nearby cohort values. The CSV is a short index into the JSON.

## Verify campaign-motivated code changes

```powershell
cargo test -p qenlo-core
cargo test -p qenlo --features gpu-wgpu
cargo test -p qenlo-bench --features gpu-wgpu
```

These tests cover exact count/materialization equivalence across deleted rows and
timestamp extremes, both timestamp materialization paths, required shader
predicate diagnostics with no host row list, and batch-total counter handling.
They are correctness and accounting checks, not performance evidence. The local
GPU suite skips device-dependent tests only when no adapter is available.

## Source and result identity

| Role | Retained SHA-256 |
|---|---|
| S0 frozen source | `f37e744c7ee4054a74c4b4181f2dde61dade4d677bbc5493b9a36b61d165a45d` |
| S1 experimental source | `730c050065e6786972a7be4f9bcb619168becdc943ce1122d1583c61f66409e3` |
| S2 corrected source | `bb195fedb6519c26598c6c103e7e182982c2eddd0a06d9c2cf54eb5cb5ceeb19` |
| S2 corrected results | `9e322d615c0bbed44616faa831493ca030dd49bbce7fe0c4386f49e3b8532fc0` |
| H2 eligibility ablation | `26e0ccc057c59c0fb4687f8d85a923e88b7bb25257e4e551e22cffe13c1b821f` |

S1 is preserved by its hash-qualified source filename beneath `research/artifacts/runpod-small-2026-09-05/`. S2 results are under `deep768/rtx4090-eu-ro-secure-deep768-admission-fix/artifacts.tar.gz` and have 74 non-self manifest entries. Both S1/S2 contain the selector candidate; the local WGSL equals the simpler S0 shader. The dirty Git HEAD alone cannot identify these executable archives.

## Clean build and render

```powershell
python paper/scripts/build_final_paper.py
python paper/scripts/verify_final_pdf.py
```

The builder creates a new timestamped directory under `paper/tmp/`, copies only source/bibliography/figure/table inputs, and executes pdfLaTeX, BibTeX, and three resolving pdfLaTeX passes. It cannot consume stale auxiliary files from `paper/`. It rejects unresolved citations/references, missing files, duplicate labels, overfull horizontal/vertical boxes, and ignored TeX errors. Only a passing build is copied to:

`QENLO-RESEARCH-PAPER.pdf`

The PDF verifier extracts layout-preserving text, checks headline values and reference/figure/table markers, renders every page at 120 DPI, and records the PDF hash, page count, and pixel hashes. It does not certify visual quality automatically. Every rendered page must be inspected for clipping, labels, legends, tables, equations, references, and page breaks. After edits, rebuild and re-inspect changed rendered pages; identical pixel output can retain its recorded review. Final visual records are tied to the delivered PDF and per-page render hashes.

## Scope and archival cautions

- Do not invoke the older `scripts/generate_small_paper_tables.py` in this final workflow: it writes the superseded six-claim CSV. It remains unchanged as historical authoring code.
- Do not overwrite source archives, raw samples, or historical plots. Superseded paper PDFs live only in git history.
- There is no script here to run cloud workloads or publish an artifact.
- USD 0.9400778694252952 is captured daily account spend; billing lag and unrelated usage prevent clean campaign-only attribution.
- Completed-call, device phases, process RSS, and owned accelerator/tensor allocations have different scopes. No efficiency score combines them.
- Retained batch-greater-than-one upload/readback/lock-wait fields written before run format v4 are B times the per-call value.
- Current mobile packaging, physical iOS/MPS performance, final-selector corrected-cell latency, calibrated routing, ANN frontiers, and energy/concurrency remain untested.
