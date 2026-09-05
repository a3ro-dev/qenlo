# Reproducing the small-collection paper

## Verify retained artifacts

The campaign root is `research/artifacts/runpod-small-2026-09-05`. The corrected 768-dimensional archive is:

```text
deep768/rtx4090-eu-ro-secure-deep768-admission-fix/artifacts.tar.gz
SHA-256 9e322d615c0bbed44616faa831493ca030dd49bbce7fe0c4386f49e3b8532fc0
```

Its 74 non-self manifest entries were verified locally. The controller ledger ends with pod deletion, a zero-pod API result, and known daily spend of USD 0.9400778694252952. Billing may settle later.

## Rebuild matrix and tables

```powershell
py -3 scripts/analyze_small_campaign.py `
  --campaign research/artifacts/runpod-small-2026-09-05 `
  --output research/artifacts/runpod-small-2026-09-05/report
py -3 scripts/generate_small_paper_tables.py
```

Expected status counts are 131 completed, 42 failed-or-unavailable, seven failed, and two invalid-harness rows.

## Run bounded local checks

```powershell
$env:CARGO_BUILD_JOBS = '1'
$env:RAYON_NUM_THREADS = '2'
cargo test -p qenlo --lib -- --test-threads=1
cargo test -p qenlo-bench --no-default-features -- --test-threads=1
```

Use new dataset and result paths for any benchmark rerun. Do not overwrite retained campaign artifacts.

## Compile and render

```powershell
Set-Location paper
pdflatex -interaction=nonstopmode paper.tex
bibtex paper
pdflatex -interaction=nonstopmode paper.tex
pdflatex -interaction=nonstopmode paper.tex
Copy-Item paper.pdf output/pdf/qenlo-small-collection-vector-search.pdf
pdftoppm -png -r 150 output/pdf/qenlo-small-collection-vector-search.pdf tmp/pdfs/final
```

Inspect every rendered page for clipping, overlap, broken tables, missing citations, and unreadable text. `pdfinfo` and text extraction are supplementary checks, not visual verification.
