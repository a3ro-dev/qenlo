# Deliverable completion audit

This ledger prevents broad claims from being inferred from narrow evidence.

| Required deliverable | Authoritative evidence | Status |
|---|---|---|
| Research question and hypotheses | `experiment-plan.md`, paper §§1–2 | complete |
| Methodology audit | `methodology-audit.md`; passing core/benchmark/WGPU test output | complete, with USearch build limitation retained |
| Reproducible GPU benchmarks | raw A6000 tuning/held-out directories, checksums, driver script | complete for CUDA prototype; native A6000 WGPU failed explicitly |
| Android device lab | author-supplied schema-v1 quick/full/soak records; processed 21-cell CSV and provenance note | complete as externally supplied evidence; no independent rerun or raw per-sample series |
| CPU exact measurements | retained 100k×384 and strict TopK raw samples | complete |
| HNSW at controlled recall | Windows USearch tuning/evaluation artifacts, ef=128, held-out recall 0.99224 | complete for one 100k×384 cell; no curve |
| Crossover analysis | native six-point CPU/WGPU sweep brackets reversal to (2k,3k); CUDA/FAISS bracket (10k,100k) | complete as bounded, hardware-specific crossovers |
| Ablations | mask/rows/predicate, CPU/GPU, batch 1/8, IVF device-lab | partial; dimension and metadata correlation absent |
| Statistical uncertainty | retained whole-run 10k-draw bootstrap comparisons | complete for native headline ratios; CUDA table descriptive only |
| Figures | Five publication figures generated as vector PDF plus PNG: architecture, two-panel phase map, matched Windows strategies, matched Android panels, and descriptive Linux context | complete without pooled or invented curves |
| Tables | paper tables plus CSV supplements | complete |
| Verified bibliography | `references.bib`, primary papers/standards with DOI or official URL | complete to the scope cited |
| LaTeX paper | `paper/paper.tex`, `appendix.tex`, compiled `paper.pdf`, stable export under `paper/output/pdf/` | complete; all ten rendered pages visually inspected, with no overflow or unresolved-reference warnings |
| Reproduction instructions | `paper/REPRODUCE.md`, campaign/analyze/plot scripts | complete |
| Raw and processed evidence | `research/data/raw`, `research/data/processed`, retained benchmark trees | complete |
| Resource cleanup/budget | Runpod API: zero pods, total spend $0.9926765 | complete |

The requested full Cartesian experiment matrix is not complete and cannot be represented as such. Its absent cells are part of the scientific result: native WGPU was unavailable on the budgeted A6000 image, while rerunning all dimensions, distributions, batches, and HNSW expansions would exceed the bounded campaign and the available prepared datasets. The paper narrows every conclusion accordingly.
