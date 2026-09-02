# Qenlo paper

This directory contains the evidence-audit manuscript. `paper.tex` is the
canonical source; `main.tex` is a compatibility entry point that inputs it.
The current PDF is an author draft, not an anonymous conference-formatted
submission. Apply the target venue's current style and anonymity rules only
when preparing the actual submission package.

## Build

From this directory:

```powershell
python ../research/scripts/analyze_results.py
python ../research/scripts/generate_plots.py
pdflatex -interaction=nonstopmode -halt-on-error paper.tex
bibtex paper
pdflatex -interaction=nonstopmode -halt-on-error paper.tex
pdflatex -interaction=nonstopmode -halt-on-error paper.tex
```

The compiled, visually verified PDF is copied to
`output/pdf/qenlo-filtered-search-phase-map.pdf` after validation.

## Evidence boundaries

- The shipped Rust/WGSL backend owns the 100k x 384 Windows and Intel Arc
  observations.
- Every 1M x 768 CUDA prototype row is exploratory PyTorch code, not a shipped
  Qenlo backend.
- FAISS leads that strict cell. The paper does not claim equivalence or a
  universal performance advantage.
- Raw samples, failures, checksums, and environment captures remain in
  `benchmark-results/`; failed systems are not silently removed.
