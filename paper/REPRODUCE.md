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
```

The campaign verifier checks CSV/JSON equality, raw corrected timings and phases, lifecycle timers, source roles, all retained result archives/checksum manifests, and qualified pair counts. Expected results: 182 rows, eight result archives, nine checksum manifests, 1,053 checked entries, four self-manifest entries skipped, 12 selector pairs (5 wins/7 losses), seven qualified Chroma pairs, and one completed-unqualified USearch row. These are integrity and descriptive checks, not significance tests.

The reducer writes to `paper/audit/reduced/` and checks parsed CSV equality against retained originals. Android is reproduced from supplied aggregate records; unavailable raw distributions cannot be recreated. Historical endpoint and localization source revisions are kept distinct. Duplicate archive mirrors are counted once.

## Regenerate numerical tables, figures, and ledger

```powershell
python paper/scripts/generate_final_tables.py
python paper/scripts/generate_final_figures.py
python paper/scripts/assemble_claim_ledger.py
```

The numerical tables use the verified campaign matrix and regenerated historical CSVs. The figure script reuses historical plotting functions with output redirected into `paper/figures/final/`; the native crossover panel deliberately omits incompatible older endpoint revisions. Existing top-level figures are never overwritten. It writes PDF and reviewable PNG for all twelve current figures and records exact input paths/hashes in `paper/audit/figure-sources.json`.

The full claim ledger embeds historical, campaign, semantic-source, and citation components and all 182 matrix rows. Missing information is not silently filled with nearby cohort values. The CSV is a short index into the JSON.

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

`paper/output/pdf/qenlo-final-research-paper.pdf`

The PDF verifier extracts layout-preserving text, checks headline values and reference/figure/table markers, renders every page at 120 DPI, and records the PDF hash, page count, and pixel hashes. It does not certify visual quality automatically. Every rendered page must be inspected for clipping, labels, legends, tables, equations, references, and page breaks. After edits, rebuild and re-inspect changed rendered pages; identical pixel output can retain its recorded review. Final visual records are tied to the delivered PDF and per-page render hashes.

## Scope and archival cautions

- Do not invoke the older `scripts/generate_small_paper_tables.py` in this final workflow: it writes the superseded six-claim CSV. It remains unchanged as historical authoring code.
- Do not overwrite source archives, raw samples, historical plots, or earlier PDFs.
- There is no script here to run cloud workloads or publish an artifact.
- USD 0.9400778694252952 is captured daily account spend; billing lag and unrelated usage prevent clean campaign-only attribution.
- Completed-call, device phases, process RSS, and owned accelerator/tensor allocations have different scopes. No efficiency score combines them.
- Current mobile packaging, physical iOS/MPS performance, final-selector corrected-cell latency, calibrated routing, ANN frontiers, and energy/concurrency remain untested.
