#!/usr/bin/env bash
# Build paper/v2/main.pdf with MiKTeX pdfLaTeX + BibTeX (on PATH).
set -euo pipefail
cd "$(dirname "$0")"
PDF=(pdflatex -interaction=nonstopmode -halt-on-error main.tex)
"${PDF[@]}" >/dev/null
bibtex main
"${PDF[@]}" >/dev/null
"${PDF[@]}" >/dev/null
if grep -Eq "undefined|Citation .* undefined|There were undefined references" main.log; then
  grep -E "undefined" main.log; exit 1
fi
cp main.pdf ../../QENLO-RESEARCH-PAPER.pdf
echo "OK: main.pdf (copied to QENLO-RESEARCH-PAPER.pdf)"
