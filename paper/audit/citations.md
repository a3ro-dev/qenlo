# Citation audit

Audit date: 2026-09-05.  The audit compared `paper/references.bib` with the
current manuscript and the committed bibliography at `HEAD`, then checked
publisher, proceedings, repository, or standards metadata.  A bibliography
key is retained when it is used by an earlier manuscript even if its suffix
does not match the authoritative publication year.

## Current citation use

The current `paper/paper.tex` cites ten keys: `johnson2019billion`,
`douze2024faiss`, `malkov2020hnsw`, `subramanya2019diskann`, `chen2021spann`,
`gollapudi2023filtered`, `patel2024acorn`, `iff2026fanns`, `shi2025unified`,
and `amanbayev2026systems`.  `paper/appendix.tex` contains no citations in
the current revision.  `li2025attribute` was added as a materially relevant
filtered-ANN study, but is intentionally marked optional until the revised
manuscript cites a claim it supports.  The older manuscript's AG News,
TopK Bench, and WebGPU references remain in the bibliography for historical
and reproducibility use.

## Verified entries

| Key | Primary source | Verification and scope |
|---|---|---|
| `johnson2019billion` | [IEEE DOI](https://doi.org/10.1109/TBDATA.2019.2921572) | Johnson, Douze, and Jégou; *IEEE Transactions on Big Data* 7(3), 535–547 (2021). Supports GPU similarity-search/k-selection context. The key's `2019` suffix reflects the DOI/publication history and is retained for compatibility; the journal publication year is 2021. |
| `douze2024faiss` | [arXiv:2401.08281](https://arxiv.org/abs/2401.08281) | Douze et al.; *The Faiss library* (2024 preprint, revised versions exist). Supports Faiss design and benchmark context. Added `eprint`, `archivePrefix`, and `primaryClass`. |
| `malkov2020hnsw` | [IEEE DOI](https://doi.org/10.1109/TPAMI.2018.2889473) | Malkov and Yashunin; *IEEE TPAMI* 42(4), 824–836 (2020). Supports HNSW as approximate graph search and its recall/latency trade-off. |
| `subramanya2019diskann` | [NeurIPS proceedings](https://papers.nips.cc/paper_files/paper/2019/hash/09853c7fb1d3f8ee67a61b6bf4a7f8e6-Abstract.html) | Subramanya, Devvrit, Simhadri, Krishnaswamy, and Kadekodi; NeurIPS 32 (2019). Corrected the historical misspelling `Krishnawamy` to `Krishnaswamy`. Supports storage-aware billion-scale ANN context, not Qenlo's exact-search claims. |
| `chen2021spann` | [NeurIPS proceedings](https://papers.nips.cc/paper/2021/hash/299dc35e747eb77177d9cea10a802da2-Abstract.html) | Chen et al.; NeurIPS 34 (2021), 5199–5212. Corrected the title from “nearest neighborhood” to the canonical “nearest neighbor.” Supports memory/disk hybrid ANN context. |
| `gollapudi2023filtered` | [ACM DOI](https://doi.org/10.1145/3543507.3583552) | Gollapudi et al.; WWW 2023, 3406–3416. Restored all twelve authors and added publisher/URL. Supports filtered-ANN graph context; it does not establish Qenlo's exact or portable-GPU results. |
| `patel2024acorn` | [ACM DOI](https://doi.org/10.1145/3654923) | Patel, Kraft, Guestrin, and Zaharia; *Proceedings of the ACM on Management of Data* 2(3), Article 120, 120:1–120:27 (2024). Added article pages, publisher, and URL. Supports predicate-agnostic hybrid ANN comparison context. |
| `webgpu` | [W3C WebGPU](https://www.w3.org/TR/webgpu/) | W3C GPU for the Web Working Group, Candidate Recommendation Draft dated 2026-08-20; accessed 2026-09-05. The citation is a moving standards document and should be read as an API/semantic reference, not a performance result. |
| `wgsl` | [W3C WGSL](https://www.w3.org/TR/WGSL/) | W3C GPU for the Web Working Group, Candidate Recommendation Draft dated 2026-08-17; accessed 2026-09-05. Scope is shader-language semantics and portability; it does not validate Qenlo measurements. |
| `topkbench` | [TopK Bench repository](https://github.com/topk-io/bench/tree/398ce7d72dea7ad765ada54c5daa014c498efb52) | Reproducibility fixture pinned to commit `398ce7d72dea7ad765ada54c5daa014c498efb52`. Treat as software/data provenance, not a peer-reviewed benchmark claim. |
| `nielsen2022agnews` | [DTU Data DOI](https://doi.org/10.11583/DTU.21286923.v1) | Nielsen, *Pretrained sentence BERT models AG News embeddings*, Technical University of Denmark, version 1, issued 2023. Corrected `year` from 2022 to the DataCite `publicationYear` 2023 and added the DOI URL. The old key suffix is retained for source compatibility. |
| `iff2026fanns` | [ACM DOI](https://doi.org/10.1145/3805712.3809731) and [DBLP record](https://dblp.org/rec/conf/sigir/IffBCKBH26.html) | Iff, Brügger, Chrapek, Kochergin, Besta, and Hoefler; SIGIR 2026, 610–622. The current revision uses the published 2026 record. An earlier commit used the arXiv-only `iff2025fanns` record; it was superseded when the DOI and proceedings metadata became available. |
| `li2025attribute` | [ACM DOI](https://doi.org/10.1145/3769763) | Li, Yan, Lu, Zhang, Cheng, and Ma; *Proceedings of the ACM on Management of Data* 3(6), Article 298, 298:1–298:26 (2025). Added as a current, primary filtered-ANN experimental study covering attribute types, selectivity, and component costs. It is currently uncited in the manuscript and should be cited only if the revised related-work argument uses those findings. |
| `shi2025unified` | [arXiv:2509.07789](https://arxiv.org/abs/2509.07789) | Shi, Cai, and Zheng; *Filtered Approximate Nearest Neighbor Search: A Unified Benchmark and Systematic Experimental Study* (2025 preprint). Supports workload-, selectivity-, and dataset-dependent FANNS comparisons. It is an arXiv preprint, not a peer-reviewed venue claim. |
| `amanbayev2026systems` | [arXiv:2602.11443](https://arxiv.org/abs/2602.11443) | Amanbayev, Tsan, Dang, and Rusu; *Filtered Approximate Nearest Neighbor Search in Vector Databases: System Design and Performance Analysis* (2026 preprint). Supports system-boundary and optimizer/filtering context. It is a preprint and should not be cited as settled consensus. |

## Historical comparison

`git show 3faf32e:paper/references.bib` contains the same core references but
uses `iff2025fanns` for arXiv:2507.21989.  The current published SIGIR record
is a legitimate update, not a change to an experimental result.  The
bibliography at `f10f71a` is otherwise materially identical.  The dataset key
`nielsen2022agnews` predates the DataCite record's issued year correction;
renaming it would force unnecessary changes to historical source text, so the
key is stable while its metadata is corrected.

## Citation boundaries

The cited ANN papers establish prior methods, design trade-offs, and broader
filtered-ANN empirical context.  They do not validate Qenlo timings, recall,
resource accounting, mobile support, or routing decisions.  Those claims must
remain tied to repository artifacts and their claim-to-artifact ledger.  The
W3C entries document API and language status only.  The TopK and DTU entries
document data/software provenance only.

## Audit artifacts

`paper/audit/references-before.bib` is the byte-preserved bibliography before
this audit.  Machine-readable source, status, and discrepancy records are in
`paper/audit/citation-ledger.json`.

## Final integrated manuscript usage

The final paper cites all 15 verified bibliography keys, including dataset provenance, W3C standards, and `li2025attribute` for workload-dependent filtered-ANN context. The earlier ten-key count above describes the pre-revision draft. The final list is in `citation-ledger.json`. No cited paper validates a Qenlo measurement.
