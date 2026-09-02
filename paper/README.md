# Qenlo paper

This directory contains the review manuscript in the official ICLR 2027
format. `main.tex` includes Akshat Singh Kushwaha's author block, but the
conference style suppresses it in review mode. Do not add `\iclrfinalcopy`
before acceptance: that switch both reveals the author and labels the paper as
published.

## Build

From this directory:

```powershell
pdflatex -interaction=nonstopmode -halt-on-error main.tex
bibtex main
pdflatex -interaction=nonstopmode -halt-on-error main.tex
pdflatex -interaction=nonstopmode -halt-on-error main.tex
```

The submission source is `main.tex` plus `references.bib` and the four official
ICLR style files. The compiled review PDF is copied to
`output/pdf/qenlo-iclr-2027-review.pdf` after validation.

## Evidence boundaries

- The shipped Rust/WGSL backend owns the 100k x 384 Windows and Intel Arc
  observations.
- The strict 1M x 768 A6000 row is a PyTorch CUDA research prototype, not a
  shipped Qenlo backend.
- FAISS leads that strict cell. The paper does not claim equivalence or a
  universal performance advantage.
- Raw samples, failures, checksums, and environment captures remain in
  `benchmark-results/`; failed systems are not silently removed.
