# Citation audit: vector-search, database and CPU/GPU-placement references in "Power-State Variance in One WGPU Vector-Search Kernel" (Qenlo), plus omitted related work

Scope: the 13 references cited in Section 9 ("Filtered vector search and CPU/GPU placement") and Section 3 (Workload) of /home/claude/QENLO-RESEARCH-PAPER.pdf, and a survey of omitted work. Audit date 2026-10-25.

Method note: shell egress to arxiv export API, Crossref, dblp and Semantic Scholar was blocked by the session proxy (HTTP 403 on CONNECT), so every record below was checked by opening the arXiv abstract page, the ACM DL / IEEE / computer.org landing page, the figshare API record or the official docs page with a web-fetch tool. The Qenlo GitHub API was also blocked for this session; I read the repo's dataset record through raw.githubusercontent.com.

The manuscript passage under audit (Section 9, p. 7), verbatim: "Filtered nearest-neighbor search is an active area (Gollapudi et al., 2023; Patel et al., 2024; Iff et al., 2026; Shi et al., 2025; Li et al., 2025; Amanbayev et al., 2026), including GPU systems (Johnson et al., 2021; Xi et al., 2025). Whether to run vector search on a GPU at all depends on data movement and batch size (Gregg and Hazelwood, 2011; Mageirakos et al., 2026), and cost-model-based CPU/GPU operator placement is known to be fragile (Breß et al., 2016). The choice between an eligible-id list and a full-width filter mirrors selection vectors versus bitmaps in vectorized databases (Ngom et al., 2021). We add that the measured margins between such alternatives can depend on the GPU's power state, which the alternatives themselves influence."

Section 3 (p. 3): "100,000 AG News sentence embeddings (all-MiniLM-L12-v2, D = 384; Nielsen, 2023) form the corpus; 5,000 disjoint held-out embeddings are the queries."

---

## Q1. Does each citation exist, are its metadata correct, and does it support the sentence it is attached to?

### Takeaway
All 13 works exist. None is fabricated, and the substantive metadata (authors, venue, volume, pages, DOI) are correct for 10 of them. There are four metadata defects. (1) VecFlow (Xi et al.) is cited only as a 2025 arXiv preprint, although it appeared in PACMMOD 3(4) in September 2025 (SIGMOD 2026). (2) The DTU dataset's title is wrong. (3) Ngom et al. lacks its article number. (4) Shi et al.'s full title is truncated. On support: Johnson et al. 2021 is cited as a filtered-search GPU system but does not address filtering. Breß et al. 2016 supports only a narrower version of the "fragile" claim. Gregg and Hazelwood 2011 supports data movement but not batch size and is not about vector search.

### Cited Findings

Classification table:

| # | Citation as given | Exists / status | Metadata problems | Support for attached sentence | Classification |
|---|---|---|---|---|---|
| 1 | Amanbayev, Tsan, Dang, Rusu 2026, arXiv:2602.11443 | arXiv v1, 11 Feb 2026. I found no peer-reviewed version. | None. Title and authors match. | Supports "active area". Background only for an exact GPU engine. | Verified and directly supportive of the weak "active area" claim |
| 2 | Breß, Funke, Teubner, SIGMOD 2016, pp. 1891–1906, doi 10.1145/2882903.2882936 | Published 26 June 2016 | None | Supports only a narrower claim (see Q2) | Verified but only partially supportive |
| 3 | Gollapudi et al., Filtered-DiskANN, WWW 2023, pp. 3406–3416, doi 10.1145/3543507.3583552 | Published 30 Apr 2023 | None | Supports "active area" | Verified and directly supportive (weak claim) |
| 4 | Gregg and Hazelwood, ISPASS 2011, pp. 134–144, doi 10.1109/ISPASS.2011.5762730 | Exists (IEEE Xplore document 5762730; ACM DL mirror 10.5555/2015039.2015535) | Venue string is set in odd all-caps (cosmetic only) | Supports "data movement"; does not address batch size or vector search | Verified but only partially supportive |
| 5 | Iff, Brügger, Chrapek, Kochergin, Besta, Hoefler, SIGIR 2026, pp. 610–622, doi 10.1145/3805712.3809731 | Published 19 July 2026 in SIGIR '26 proceedings | None | Supports "active area" | Verified and directly supportive (weak claim) |
| 6 | Johnson, Douze, Jégou, IEEE TBD 7(3):535–547, 2021, doi 10.1109/TBDATA.2019.2921572 | Exists. Issue dated 1 July 2021; first online 10 June 2019. | None | Cited as a GPU system for filtered search. The paper covers brute-force, IVF and PQ GPU search and a new k-selection; the abstract does not mention attribute filtering. | Misattributed for "filtered … including GPU systems". It is relevant background for GPU exact search and GPU k-selection. |
| 7 | Li, Yan, Lu, Zhang, Cheng, Ma, PACMMOD 3(6):298:1–26, 2025, doi 10.1145/3769763 | Published 5 Dec 2025 | None | Supports "active area" | Verified and directly supportive (weak claim) |
| 8 | Mageirakos, André, Kabić, Wu, Chronis, Alonso 2026, arXiv:2605.15957 | arXiv v1, 15 May 2026, comment "preprint". Not peer-reviewed as far as I found. | arXiv title capitalization is "To GPU or Not to GPU: Vector Search in Relational Engines". Should be labelled as a preprint. | Directly addresses data movement and batch size for GPU vector search (see Q4) | Verified and directly supportive. Note that it studies GB-scale indexes and SQL+VS queries, not batch-1 resident 100k×384 data. |
| 9 | Ngom, Menon, Butrovich, Ma, Lim, Mowry, Pavlo, DaMoN 2021, pp. 1–7, doi 10.1145/3465998.3466009 | Published 20 June 2021 | Article No. 6 missing (pages 1–7 are within the article). Proceedings: "DAMON '21: Proc. 17th Int. Workshop on Data Management on New Hardware". | Directly supports the selection-vector vs bitmap dichotomy (see Q3) | Verified and directly supportive. The analogy to GPU is the author's. |
| 10 | Nielsen 2023, "DTU pretrained sentence BERT AG News embeddings", doi 10.11583/DTU.21286923.v1 | Dataset exists, posted 29 Sep 2023, CC BY 4.0 | Title is wrong. The actual title is "Pretrained sentence BERT models AG News embeddings", published by the Technical University of Denmark. | Contents match the use (see Q5) | Incorrect or incomplete bibliographic metadata; the substance is verified |
| 11 | Patel, Kraft, Guestrin, Zaharia, ACORN, PACMMOD 2(3):120:1–27, 2024, doi 10.1145/3654923 | Published 30 May 2024 | None | Supports "active area" | Verified and directly supportive (weak claim) |
| 12 | Shi, Cai, Zheng 2025, arXiv:2509.07789 | arXiv v1, 9 Sep 2025. I found no peer-reviewed version. | The full title carries the suffix "[Experiment, Analysis & Benchmark]" | Supports "active area" | Verified and directly supportive (weak claim). Publication status: preprint. |
| 13 | Xi, Mo, Karsin, Chirkin, Li, Zhang 2025, VecFlow, arXiv:2506.00812 | Since published: PACMMOD 3(4), Article 271, pp. 1–27, 23 Sep 2025, doi 10.1145/3749189 (SIGMOD 2026 paper) | Cites the preprint only. Should cite the PACMMOD version. ACM lists the author as "Ben Karsin". | Directly supports "filtered search … GPU systems" | Incorrect or incomplete metadata (outdated); content directly supportive |

Evidence for each row:
- Amanbayev et al.: arXiv abstract page, "Submitted on 11 Feb 2026", authors Amanbayev, Tsan, Dang, Rusu; it evaluates filtering strategies in FAISS, Milvus and pgvector and introduces MoReVec and the GLS metric — [arXiv:2602.11443](https://arxiv.org/abs/2602.11443)
- Amanbayev et al. findings that bear on Qenlo's exact-search framing: "pgvector's cost-based query optimizer frequently selects suboptimal execution plans, favoring approximate index scans even when exact sequential scans would yield perfect recall at comparable latency"; "Milvus achieves superior recall stability through hybrid approximate/exact execution" — [arXiv:2602.11443](https://arxiv.org/abs/2602.11443). Milvus bypasses HNSW and runs a brute-force scan when the filtered-out ratio exceeds a threshold "set to 93% in the codebase" — [arXiv HTML 2602.11443](https://arxiv.org/html/2602.11443)
- Breß et al.: ACM DL, SIGMOD '16, pages 1891–1906, published 26 June 2016 — [ACM DL](https://dl.acm.org/doi/10.1145/2882903.2882936)
- Filtered-DiskANN: WWW '23, pages 3406–3416, published 30 April 2023 — [ACM DL](https://dl.acm.org/doi/10.1145/3543507.3583552)
- Gregg and Hazelwood: IEEE Xplore document 5762730 is titled "Where is the data? Why you cannot debate CPU vs. GPU performance without the answer" — [IEEE Xplore](https://ieeexplore.ieee.org/document/5762730). ACM DL mirror lists pages 134–144, 10 April 2011, and the abstract finding that "when memory transfer times are included, it can easily take between 2 to 50 [times] longer to run a kernel than the GPU processing time alone" — [ACM DL](https://dl.acm.org/doi/10.5555/2015039.2015535). The authors' PDF says "the faster the GPU, the more important it is to consider the data transfer overhead" — [author PDF](https://web.stanford.edu/~cgregg/chris-gregg/pubs/WhereIsTheData.pdf)
- Iff et al.: SIGIR '26, pages 610–622, published 19 July 2026, authors Iff, Brügger, Chrapek, Kochergin, Besta, Hoefler — [ACM DL](https://dl.acm.org/doi/10.1145/3805712.3809731)
- Johnson et al.: IEEE TBD vol. 7, issue 3 (1 July 2021), pp. 535–547, first online 10 June 2019. The abstract covers k-selection, brute-force, approximate and PQ search and contains no mention of attribute filtering — [IEEE Xplore](https://ieeexplore.ieee.org/document/8733051)
- Li et al.: PACMMOD vol. 3, issue 6, Article 298, pp. 1–26, published 05 Dec 2025 — [ACM DL](https://dl.acm.org/doi/10.1145/3769763)
- Mageirakos et al.: "Submitted on 15 May 2026", Comments: "preprint" — [arXiv:2605.15957](https://arxiv.org/abs/2605.15957)
- Ngom et al.: DAMON '21, Article No. 6, pages 1–7, published 20 June 2021, seven authors as cited — [ACM DL](https://dl.acm.org/doi/10.1145/3465998.3466009)
- Nielsen dataset: figshare record title "Pretrained sentence BERT models AG News embeddings"; its own citation string is "Nielsen, Beatrix Miranda Ginn (2023). Pretrained sentence BERT models AG News embeddings. Technical University of Denmark. Dataset." — [figshare API](https://api.figshare.com/v2/articles/21286923); [DTU landing page](https://data.dtu.dk/articles/dataset/Pretrained_sentence_BERT_models_AG_News_embeddings/21286923)
- ACORN: PACMMOD vol. 2, issue 3, Article 120, pp. 1–27, published 30 May 2024 — [ACM DL](https://dl.acm.org/doi/10.1145/3654923)
- Shi et al.: arXiv title "Filtered Approximate Nearest Neighbor Search: A Unified Benchmark and Systematic Experimental Study [Experiment, Analysis & Benchmark]", "Submitted on 9 Sep 2025" — [arXiv:2509.07789](https://arxiv.org/abs/2509.07789). Iff et al. (July 2026) still cite it as a 2025 arXiv preprint — [ACM DL](https://dl.acm.org/doi/10.1145/3805712.3809731)
- VecFlow: arXiv v1 1 Jun 2025 — [arXiv:2506.00812](https://arxiv.org/abs/2506.00812). Published version: PACMMOD vol. 3, issue 4, Article 271, pp. 1–27, published 23 September 2025, doi 10.1145/3749189 — [ACM DL](https://dl.acm.org/doi/10.1145/3749189). The repository README says "VecFlow accepted by SIGMOD 2026" — [GitHub](https://github.com/Supercomputing-System-AI-Lab/VecFlow)

Corrected bibliography entries (only where a change is needed):
- Breß, S., Funke, H., Teubner, J. Robust Query Processing in Co-Processor-accelerated Databases. SIGMOD '16, pp. 1891–1906. ACM, 2016. https://doi.org/10.1145/2882903.2882936 (unchanged; for the "fragile" claim add Karnagel et al. 2017 and Rosenfeld et al. 2022, listed under Q7)
- Ngom, A., Menon, P., Butrovich, M., Ma, L., Lim, W. S., Mowry, T. C., Pavlo, A. Filter Representation in Vectorized Query Execution. DaMoN '21, Article 6, pp. 1–7. ACM, 2021. https://doi.org/10.1145/3465998.3466009
- Nielsen, B. M. G. Pretrained sentence BERT models AG News embeddings (version 1). Technical University of Denmark, dataset, 2023. CC BY 4.0. https://doi.org/10.11583/DTU.21286923.v1 (optionally also cite the companion paper: Nielsen, B. M. G., Hansen, L. K. Hubness Reduction Improves Sentence-BERT Semantic Spaces. NLDL 2024, PMLR 233:181–204. https://proceedings.mlr.press/v233/nielsen24a.html — verified at [PMLR](https://proceedings.mlr.press/v233/nielsen24a.html))
- Xi, J., Mo, C., Karsin, B., Chirkin, A., Li, M., Zhang, M. VecFlow: A High-Performance Vector Data Management System for Filtered-Search on GPUs. Proc. ACM Manag. Data 3(4), Article 271, pp. 1–27, 2025. https://doi.org/10.1145/3749189
- Shi, J., Cai, Y., Zheng, W. Filtered Approximate Nearest Neighbor Search: A Unified Benchmark and Systematic Experimental Study [Experiment, Analysis & Benchmark]. arXiv:2509.07789, 2025 (preprint). https://arxiv.org/abs/2509.07789
- Mageirakos, V., André, J., Kabić, M., Wu, B., Chronis, Y., Alonso, G. To GPU or Not to GPU: Vector Search in Relational Engines. arXiv:2605.15957, 2026 (preprint). https://arxiv.org/abs/2605.15957
- Johnson, Douze, Jégou 2021: metadata unchanged. Move the citation from the "filtered … GPU systems" clause to a clause about GPU exact/brute-force search and k-selection. FAISS's filtering support is described in Douze et al., "The Faiss Library", IEEE TBD 12(2), 2026, doi 10.1109/TBDATA.2025.3618474 — [IEEE Xplore](https://ieeexplore.ieee.org/document/11202651); the computer.org page says "Faiss has basic support for vector filtering: the user can provide a predicate (IDSelector callback)" — [computer.org](https://www.computer.org/csdl/journal/bd/2026/02/11202651/2aMEA0PkUve)

### Inferences
- None of the cited filtered-search works studies exact (brute-force) filtered top-k on a GPU at batch 1, or power state. They justify "active area" but give no comparison point for Qenlo's light/heavy kernels. The closest cited work is VecFlow, which has an IVF-BFS path for low-specificity labels; that is an exact scan within a label's posting list on the GPU.
- The six filtered-ANN citations are interchangeable background. A reviewer is more likely to object to the omission of exact-filtered GPU baselines (cuVS brute_force with prefilter, Milvus GPU_BRUTE_FORCE) and GPU top-k primitives (Q7) than to anything in this list.
- Fix needed: "including GPU systems (Johnson et al., 2021; Xi et al., 2025)" should cite VecFlow (PACMMOD) for GPU filtered search, and should cite Johnson et al. for GPU exact search and k-selection instead.

### Gaps
- I could not open the IEEE DOI resolver page for Gregg and Hazelwood to read the printed DOI string. The document number (5762730) and title match, so the DOI 10.1109/ISPASS.2011.5762730 is consistent but was not seen printed.
- Absence of peer-reviewed versions of Amanbayev et al., Shi et al. and Mageirakos et al. rests on web search and arXiv pages showing only v1. The ACM DL, PVLDB and VLDB 2026 tables of contents were not exhaustively searched.

### URLs actually opened (fetched)
arxiv.org/abs/2602.11443; arxiv.org/abs/2605.15957; arxiv.org/html/2605.15957v1; arxiv.org/abs/2509.07789; arxiv.org/abs/2506.00812; arxiv.org/abs/2604.02344; arxiv.org/abs/2608.08730; doi.org/10.11583/DTU.21286923.v1; data.dtu.dk/articles/dataset/Pretrained_sentence_BERT_models_AG_News_embeddings/21286923; api.figshare.com/v2/articles/21286923; dl.acm.org/doi/10.1145/3805712.3809731; dl.acm.org/doi/10.1145/3769763; dl.acm.org/doi/10.1145/3654923; dl.acm.org/doi/10.1145/3543507.3583552; dl.acm.org/doi/10.1145/2882903.2882936; dl.acm.org/doi/10.1145/3465998.3466009; dl.acm.org/doi/10.1145/3183713.3183735; dl.acm.org/doi/10.1145/2133803.2345676; dl.acm.org/doi/10.14778/3749646.3749725; ieeexplore.ieee.org/document/5762730; ieeexplore.ieee.org/document/8733051; tu.berlin/.../p1891-breb_preprint.pdf; huggingface.co/sentence-transformers/all-MiniLM-L12-v2; proceedings.mlr.press/v233/nielsen24a.html; github.com/a3ro-dev/qenlo; raw.githubusercontent.com/a3ro-dev/qenlo/main/llms-full.txt; raw.githubusercontent.com/a3ro-dev/qenlo/main/benchmarks/2026-08-28/real/dataset/ag-news-384.json. Everything else cited below came from search-result excerpts of the named page, without a full fetch.

---

## Q2. Does Breß, Funke, Teubner (SIGMOD 2016) show that "cost-model-based CPU/GPU operator placement is known to be fragile"?

### Takeaway
Only in a narrower sense. Breß et al. show that compile-time, operator-driven placement degrades badly when co-processor memory is scarce. The two effects they name are cache thrashing (up to 24×) and heap contention (up to 6×). They argue that compile-time heuristics depend on hard-to-get cardinality estimates and on run-time load. However, they keep HyPE's learned cost models and fix the problem with run-time placement and query chopping. The paper is not a demonstration that cost models as such are fragile. A more accurate wording would be "compile-time CPU/GPU operator placement is fragile when estimates or co-processor resources are wrong (Breß et al., 2016; Karnagel et al., 2017)".

### Cited Findings
- Abstract: "Cache thrashing occurs when the working set of queries does not fit into the co-processor's data cache, resulting in performance degradations up to a factor of 24. Heap contention occurs when multiple operators run in parallel on a co-processor … slowing down query execution by up to a factor of six." The proposed fixes are data-driven operator placement and query chopping, validated in CoGaDB on the Star Schema Benchmark (SSB) and TPC-H — [ACM DL](https://dl.acm.org/doi/10.1145/2882903.2882936)
- Section 4 lists three drawbacks of compile-time placement: "1. It cannot predict error situations such as out of memory scenarios … 2. Compile-time heuristics need to rely on sufficiently accurate cardinality estimates to estimate the data transfer volume. However, it is still very difficult to provide cardinality estimations. 3. GPU code is particularly sensitive to environment parameters such as current load or usage of heap memory. Inherently, those parameters cannot be known before the actual execution time" — [preprint PDF](https://www.tu.berlin/fileadmin/www/10002226/Publications/Papers/p1891-breb_preprint.pdf)
- Cost models are kept, not abandoned. Chopping means "we always know the exact input cardinalities during the tactical optimization, which increases the accuracy of HyPE's cost models and, hence, its operator placement" — [preprint PDF](https://www.tu.berlin/fileadmin/www/10002226/Publications/Papers/p1891-breb_preprint.pdf)
- Run-time placement alone "is still more than two times slower than the optimal case"; Data-Driven Chopping "never performs worse than a CPU-only approach" — [preprint PDF](https://www.tu.berlin/fileadmin/www/10002226/Publications/Papers/p1891-breb_preprint.pdf)
- The same paper cites Karnagel et al., who "conclude that both approaches [compile-time and run-time placement] are similarly efficient, where run-time placement is easier to implement and global optimization achieves an overall more robust performance" — [preprint PDF](https://www.tu.berlin/fileadmin/www/10002226/Publications/Papers/p1891-breb_preprint.pdf)
- Karnagel, Habich, Lehner, "Adaptive Work Placement for Query Processing on Heterogeneous Computing Resources", PVLDB 10(7):733–744, 2017, doi 10.14778/3067421.3067423. Its first stated open challenge is "C1 - Inaccurate Cardinality Information" — [PVLDB PDF](https://www.vldb.org/pvldb/vol10/p733-karnagel.pdf); DOI as listed in [Rosenfeld et al. references](https://dl.acm.org/doi/abs/10.1145/3485126?sid=SCITRUS)

### Inferences
- Breß et al.'s failure modes (memory capacity, caching, cardinality) are not Qenlo's (GPU power state, per-process clock variation). The citation supports the idea that a placement decision fixed in advance can be wrong once run-time conditions differ. It does not support the stronger claim that cost models are inherently fragile.
- Appendix C of the manuscript describes a scalar router (E·D·B ≥ 10^6) that failed on held-out workloads. The better analogue is Karnagel et al. 2017 (adaptive run-time work placement) together with the Rosenfeld/Breß/Markl survey, which classifies operator-placement systems that use learned cost models.

### Gaps
- I read the preprint, not the camera-ready. The page numbers match the ACM record and the text is assumed identical.

---

## Q3. Does Ngom et al. (DaMoN 2021) discuss selection vectors versus bitmaps as the paper says?

### Takeaway
Yes, directly. The paper is a micro-benchmark study of exactly these two filter representations in vectorized CPU query execution. Bitmaps win for operations that SIMD can vectorize; selection vectors win otherwise because their iteration is cheaper. The analogy to Qenlo's GPU choice (uploaded eligible-id list vs in-kernel full-width predicate) is the author's own. Ngom et al. study CPUs only.

### Cited Findings
- "For each vector (batch) of tuples, the DBMS must track the set of valid (visible) tuples … existing systems employ one of two data structures, or filter representations: selection vectors or bitmaps." — [ACM DL abstract](https://dl.acm.org/doi/10.1145/3465998.3466009)
- "the optimal strategy is a function of many factors: the cost of iterating through tuples, the cost of the operation itself, and how amenable it is to SIMD vectorization. Our analysis shows that bitmaps perform better for operations that can be vectorized using SIMD instructions and that selection vectors perform better on all other operations due to cheaper iteration logic." — [ACM DL abstract](https://dl.acm.org/doi/10.1145/3465998.3466009)

### Inferences
- Qenlo's heavy path evaluates the predicate over all N rows, which corresponds to a full-width mask. Its light path compacts to E ids, which corresponds to a selection vector. The analogy holds, with a caveat: in Qenlo the heavy kernel also uses a k-pass single-workgroup selection, so the two GPU paths differ in more than filter representation. The manuscript should say so wherever it invokes this analogy.
- A GPU-side analogue that the paper omits: cuVS brute_force accepts a bitset (shared) or bitmap (per-query) prefilter (Q7). That is the production form of the full-width-mask approach on GPU.

### Gaps
- I read only the abstract and landing page, not the full PDF, so the paper's specific selectivity crossover numbers are not recorded here.

---

## Q4. Does Mageirakos et al. 2026 exist, and what does it conclude about GPU vector search, batch size and data movement?

### Takeaway
It exists as an arXiv preprint (v1, 15 May 2026, ETH Zurich group with Alonso). Its conclusions support the manuscript's sentence, in a different regime. When an index or embeddings must move to the GPU, data movement dominates and GPU ANN often loses to CPU even over NVLink. Batching amortizes movement: IVF copy-i beats CPU at 10–100 queries, while CAGRA copy-di needs more than about 10^3 queries. Exact (brute-force) search on the GPU is competitive with fast interconnects. The study concerns ~10 GB indexes in a SQL+VS engine. It does not study batch-1 latency with GPU-resident 100k×384 data, and it does not mention power state.

### Cited Findings
- "when the vector search involves moving data and indexes, using the GPU is not the best option, even with fast interconnects"; "the relational components benefit much more from running on the GPU than the vector search part" — [arXiv abstract](https://arxiv.org/abs/2605.15957)
- "large batch sizes are needed to amortize transfer costs (Johnson et al., 2021)". Distance computation is an N×d×M GEMM, so "batching is necessary for higher GPU throughput and utilization for VS" — [arXiv HTML](https://arxiv.org/html/2605.15957v1)
- "Even when using NVLink, with current data-owning vector indexes, running vector search on a GPU does not pay off unless the vector index is resident on the GPU" — [arXiv HTML](https://arxiv.org/html/2605.15957v1)
- For exact search (ENN) with copy-di: over NVLink it is "4×–19× faster than cpu"; over PCIe "the performance is a tie with cpu on most queries". ANN copy-di over PCIe is "orders of magnitude slower than cpu", and index transfer takes "over 95% of wall time on most queries" — [arXiv HTML](https://arxiv.org/html/2605.15957v1)
- Batch-size crossover: "copy-i becomes better than cpu between 10–100 queries" for IVF1024; for CAGRA "copy-i loses to cpu at every batch size, and only copy-di improves over cpu at above a 10^3-query batch" — [arXiv HTML](https://arxiv.org/html/2605.15957v1)
- Transfer is dominated by overhead, not bandwidth: on NVLink "moving ~10 GB of embeddings (ENN) takes 28.7 ms while a similarly sized IVF1024 index takes 1266 ms" — [arXiv HTML](https://arxiv.org/html/2605.15957v1)
- They report a FAISS GPU top-k cap: post-filter oversampling of 500×k "exceeds the GPU top-k cap enforced by FAISS", giving <50% recall on Q15 on the GPU — [arXiv HTML](https://arxiv.org/html/2605.15957v1)
- Their conclusion anticipates per-query routing: "the query optimizer will estimate data movement, operator runtimes, batch size, GPU memory budget, and recall target to identify the optimal per-query execution strategy" — [arXiv HTML](https://arxiv.org/html/2605.15957v1)
- It cites a GPU vector-search survey (Liu et al., 2026) reporting that PCIe data transfers take 40–97% of total latency — [arXiv HTML](https://arxiv.org/html/2605.15957v1). That survey appears to be "GPU-Accelerated Algorithms for Graph Vector Search: Taxonomy, Empirical Study, and Research Directions", Liu, Chen, Tian et al., arXiv:2602.16719 (10 Feb 2026) — [arXiv](https://arxiv.org/abs/2602.16719v1)

### Inferences
- The citation is appropriate for "depends on data movement and batch size". The manuscript should note that Mageirakos et al. study the transfer-dominated, large-batch regime, whereas Qenlo studies a resident-corpus, batch-1 regime where per-dispatch overhead and clock state dominate. These are complementary findings, not the same one.
- Their per-query optimizer proposal is the closest published framing to Qenlo's router. Since they do not model power state, Qenlo's point that a router's cost inputs can depend on GPU power state is an additive contribution relative to this work, as far as I found.

### Gaps
- I did not check whether Mageirakos et al. report run-to-run variance, clock locking or warm-up practice. The fetched excerpts contained no mention of DVFS or clocks.

---

## Q5. Does the DTU dataset match the manuscript's description (all-MiniLM-L12-v2, D = 384, 100,000 corpus + 5,000 held-out queries, license)?

### Takeaway
Yes, with three qualifications. The DTU record contains all-MiniLM-L12-v2 embeddings of AG News as separate train and test HDF5 files, under CC BY 4.0. Qenlo's own dataset record shows it read only the train file, shape 120,000×384. It used rows 0–100,000 as the corpus, 100,000–101,000 for tuning, and 101,000–106,000 as the 5,000 queries. The 120,000 rows are enough for the ≥106,000 needed. The qualifications: "held-out" means held out from the corpus within the AG News training split, not the 7,600-row AG News test split; rows were not deduplicated; and the CC BY 4.0 embedding license sits on top of an AG corpus whose originator restricts use to non-commercial purposes.

### Cited Findings
- The DTU record lists eight files. Two are all-MiniLM-L12-v2: `all-MiniLM-L12-v2_ag_news_train.h5` (170,800,305 bytes, md5 364dd01d4a383fd5a2628f9d4ea8279a) and `all-MiniLM-L12-v2_ag_news_test.h5` (10,837,079 bytes). Three other models are included: all-distilroberta-v1, all-mpnet-base-v2 and multi-qa-distilbert-cos-v1. License "CC BY 4.0"; published 2023-09-29; author Beatrix Miranda Ginn Nielsen; funding Danish Pioneer Centre for AI; related paper "Hubness Reduction Improves Sentence-BERT Semantic Spaces" — [figshare API](https://api.figshare.com/v2/articles/21286923)
- The description names the AG News source as the Kaggle dataset amananandrai/ag-news-classification-dataset — [DTU landing page](https://data.dtu.dk/articles/dataset/Pretrained_sentence_BERT_models_AG_News_embeddings/21286923)
- all-MiniLM-L12-v2 "maps sentences & paragraphs to a 384 dimensional dense vector space"; model license Apache-2.0 — [Hugging Face](https://huggingface.co/sentence-transformers/all-MiniLM-L12-v2)
- AG News: "The total number of training samples is 120,000 and testing 7,600"; the Kaggle license field shows "Unknown" — [Kaggle](https://www.kaggle.com/datasets/amananandrai/ag-news-classification-dataset); TFDS split table train 120,000 / test 7,600 — [TensorFlow Datasets](https://www.tensorflow.org/datasets/catalog/ag_news_subset)
- The original AG corpus is "provided by the academic comunity for research purposes … and any other non - commercial activity" — [Gulli AG corpus page](http://groups.di.unipi.it/~gulli/AG_corpus_of_news_articles.html)
- Qenlo's dataset record: source_url `https://ndownloader.figshare.com/files/37763697` (the train file), md5 matches the publisher's, `source_shape [120000, 384]`, float32, "first consecutive source training rows; no shuffle or normalization", `corpus_range [0, 100000]`, `tuning_range [100000, 101000]`, `evaluation_range [101000, 106000]`, "content_deduplication": "not performed; row intervals are disjoint", license CC BY 4.0 — [Qenlo dataset record](https://raw.githubusercontent.com/a3ro-dev/qenlo/main/benchmarks/2026-08-28/real/dataset/ag-news-384.json)
- Qenlo docs: "100,000 rows × 384 dimensions from the DTU AG News all-MiniLM-L12-v2 HDF5 source", and "The corpus, tuning queries, and evaluation queries are disjoint" — [Qenlo llms-full.txt](https://raw.githubusercontent.com/a3ro-dev/qenlo/main/llms-full.txt)

### Inferences
- Size check: 120,000 × 384 × 4 bytes is 184,320,000 bytes, larger than the 170,800,305-byte file. HDF5 compression would explain this. The test file shows the same ratio (~0.93: 7,600×384×4 = 11,673,600 vs 10,837,079 bytes). This is consistent with 120,000 rows, but I did not download the file to confirm its shape. The shape claim rests on Qenlo's own record.
- "No normalization": all-MiniLM-L12-v2's reference usage L2-normalizes outputs, but whether the DTU HDF5 vectors are unit-norm is not recorded anywhere I could see. For cosine search this matters only if Qenlo assumes unit norm. This should be cross-checked by whoever audits correctness.
- Because rows were not deduplicated, some queries may be near-duplicates of corpus rows. That could produce near-ties or exact ties affecting top-k tie-breaking. It does not affect the timing argument.
- Suggested manuscript wording: "100,000 embeddings from the AG News training split (rows 0–99,999 of the all-MiniLM-L12-v2 train file of Nielsen, 2023, CC BY 4.0); 5,000 later rows of the same split (101,000–105,999) serve as queries." Also mention the non-commercial terms of the underlying AG corpus.

### Gaps
- I could not open the HDF5 file to verify its shape, dtype or row norms.
- Whether the Kaggle AG News CSV is row-aligned with the HDF5 rows (relevant if text is ever shown) was not checked.

---

## Q6. Has prior work observed that GPU vector-search latency at batch 1 depends on GPU power state, or compared work-efficient vs naive filtered kernels on GPU?

### Takeaway
I found no prior work that ties GPU vector-search (ANN or exact) latency at batch 1 to GPU clock or power state, or that compares compacted-id against full-width-mask filtered kernels on a GPU under DVFS. The absence rests on about eight targeted searches, so treat the novelty as plausible, not proven. Adjacent work exists on four fronts. (a) Batch-1 WebGPU and wgpu dispatch overhead (Maczan 2026), directly relevant to a WGPU engine. (b) Frequency insensitivity of memory-bound LLM decode (Maliakel et al., already cited). (c) Production systems that switch between brute-force pre-filtering and index search by selectivity (Milvus, Weaviate, pgvector, OpenSearch). (d) cuVS brute-force with bitset or bitmap prefilters. None of these examines power state.

### Cited Findings
- Maczan, "Characterizing WebGPU Dispatch Overhead for LLM Inference Across Four GPU Vendors, Three Backends, and Three Browsers", arXiv:2604.02344 (9 Feb 2026). It studies batch size 1 across Dawn and wgpu-native; "The true per-dispatch cost of WebGPU API overhead alone is 24-36 µs on Vulkan and 32-71 µs on Metal"; naive single-operation benchmarks "overestimate dispatch cost by ~20×" — [arXiv](https://arxiv.org/abs/2604.02344)
- Maczan, "Measuring and Reducing WebGPU Dispatch Overhead for LLM Inference", arXiv:2608.08730 (v2 11 Aug 2026), "Accepted to The Fourth UK AI Conference 2026 … PMLR". Dispatch overhead, "not kernel quality, is the bottleneck at batch size 1"; naive measurements conflate "dispatch with synchronization" — [arXiv](https://arxiv.org/abs/2608.08730)
- Maliakel et al. (already cited by the manuscript) find LLM decode "largely insensitive to GPU frequency", with "42% energy savings with only 1–6% latency increase" from 2842 to 180 MHz — [arXiv HTML 2501.08219v4](https://arxiv.org/html/2501.08219v4). This is a contrast case: memory-bound work that does not slow at low clock.
- cuVS brute-force supports filtered search with bitmap or bitset prefilters; "if a filter removes 90%-99% of the vectors, brute-force may do much less work while still returning exact results within the allowed subset" — [cuVS brute-force guide](https://docs.nvidia.com/cuvs/user-guide/api-guides/indexing-guide/brute-force); prefilter "using a bitmap or bitset" — [cuVS Python API](https://docs.rapids.ai/api/cuvs/stable/python_api/neighbors_brute_force/)
- Caminal et al. (VLDB 2025) classify pre-filtering ("data is filtered and then KNN search is performed on the result of the filter. This method is preferred when the filter result is small"), post-filtering and inline-filtering, where filters may be "evaluated before the vector search starts (and stored in a bitmap)" — [PVLDB PDF](https://www.vldb.org/pvldb/vol18/p5488-caminal.pdf)
- Weaviate can "automatically switch to a flat (brute-force) vector search when a filter becomes too restrictive" — [Weaviate docs (GitHub)](https://github.com/weaviate/docs/blob/main/docs/weaviate/concepts/filtering.md). OpenSearch k-NN has a user-set "Exact Search filtered threshold" — [OpenSearch KNNWeight.java](https://github.com/opensearch-project/k-NN/blob/main/src/main/java/org/opensearch/knn/index/query/KNNWeight.java)
- A related energy paper, "The Energy Cost of Execution-Idle in GPU Clusters", arXiv:2604.04745, reports that inference workloads spend 12–13% of time in "execution-idle" states. It also evaluates frequency downscaling with a latency penalty (p95 2.31 s default vs 2.99 s SM-only vs 6.03 s SM+Mem) — [arXiv PDF](https://arxiv.org/pdf/2604.04745). I saw this only in a search excerpt; authors and venue are not verified.

### Inferences
- The manuscript's addition ("measured margins between such alternatives can depend on the GPU's power state, which the alternatives themselves influence") is not contradicted by anything I found. It should still be phrased as "we did not find prior reports of …" with the search scope stated, not as a priority claim.
- Maczan's batch-1 wgpu/Vulkan dispatch numbers (24–36 µs per dispatch) bear directly on Qenlo's light-path kernel times (0.03–0.05 ms at E = 1,000 in Table 1) and on end-to-end latencies of ~0.1–0.2 ms. Section 6 and Section 10 should cite them, along with the sequential-dispatch method that separates dispatch from synchronization cost.
- Comparing a work-efficient kernel against a deliberately naive one is a methodology, not a known benchmark. The production analogue of the heavy path is cuVS brute_force with a bitset prefilter, which still scans N but uses a tuned selection kernel. That is the natural "competitive full-width" control that Section 10 says is missing.

### Gaps
- I did not search non-English literature or patents. I did not search GPU-recommendation inference papers (e.g., embedding lookup at batch 1) for clock-state effects.
- I could not verify whether FAISS GPU indexes honor IDSelector filtering on the GPU (as opposed to CPU-only). This needs checking in the FAISS source.

---

## Q7. Which important related works does the manuscript omit? (verified metadata, one line on relevance each)

### Takeaway
The biggest omissions are GPU top-k/selection primitives and production GPU exact search (cuVS brute_force with prefilter, Milvus GPU_BRUTE_FORCE, FAISS GPU flat). These are the competitive baselines for Qenlo's "naive" selection kernel. Next come GPU ANN systems (CAGRA, GGNN, SONG, BANG), CPU/GPU placement work beyond Breß 2016 (HyPE, Karnagel 2017, Rosenfeld et al. 2022 survey, Breß et al. 2014 survey, TQP), the filtered-search survey and benchmarks, and WebGPU dispatch-overhead studies.

### Cited Findings

GPU top-k / selection primitives:
- Alabi, Blanchard, Gordon, Steinbach. Fast k-selection algorithms for graphics processing units. ACM J. Exp. Algorithmics 17, Article 4.2, pp. 4.1–4.29, 2012. doi 10.1145/2133803.2345676 — [ACM DL](https://dl.acm.org/doi/10.1145/2133803.2345676). Relevance: foundational GPU k-selection (radix/bucket select); the baseline lineage for any claim about selection cost.
- Shanbhag, Pirk, Madden. Efficient Top-K Query Processing on Massively Parallel Hardware. SIGMOD '18, pp. 1557–1570, 2018. doi 10.1145/3183713.3183735 — [ACM DL](https://dl.acm.org/doi/10.1145/3183713.3183735). Bitonic top-k is "up to a factor of 15x faster than sort and 4x faster than a variety of other possible implementations for values of k up to 256" — [Imperial Spiral](https://spiral.imperial.ac.uk/entities/publication/42de3f62-fb24-4adf-a956-4871a534e3d9); code at [github.com/anilshanbhag/gpu-topk](https://github.com/anilshanbhag/gpu-topk). Relevance: a well-studied replacement for the heavy path's k-pass selection at k = 10.
- Gaihre, Zheng, Weng, Ding, Li, et al. Dr. Top-k: Delegate-Centric Top-k on GPUs. SC '21, pp. 1–14, 2021. doi 10.1145/3458817.3476141 — [ACM DL](https://dl.acm.org/doi/10.1145/3458817.3476141); [computer.org](https://www.computer.org/csdl/proceedings-article/sc/2021/09910070/1HzBK1bZjt6). Relevance: delegate filtering that can "reduce the workload for top-k up to more than 99%", which is the work-efficiency argument applied to top-k itself. (The full author list was not fully captured; verify it before citing.)
- Zhang et al. (NVIDIA RAFT). Parallel Top-K Algorithms on GPU: A Comprehensive Study and New Methods (AIR Top-K, GridSelect). SC '23, pp. 1–13, 2023. doi 10.1145/3581784.3607062 — [ACM DL](https://dl.acm.org/doi/10.1145/3581784.3607062). For "batch sizes 1 and 100", AIR Top-K shows "1.98–21.48× and 8.01–574.78× speedup over previous radix top-K". Relevance: explicitly evaluates batch 1, and is the algorithm behind RAFT/cuVS select_k. (Author names were not captured in my fetch; verify before citing. The attribution to RAFT/cuVS select_k comes from the brief and was not independently verified.)
- Li, Zhou, Zhang, Wei, Li, Chen. RadiK: Scalable and Optimized GPU-Parallel Radix Top-K Selection. ICS '24, pp. 537–548, 2024. doi 10.1145/3650200.3656596 — [ACM DL](https://dl.acm.org/doi/abs/10.1145/3650200.3656596); arXiv:2501.14336. Relevance: "up to 2.5x speedup over the prior art for non-batch queries"; it addresses non-batched top-k directly.
- WarpSelect / BlockSelect in FAISS: described in Johnson et al. 2021 Sections 3–4 ("Fast k-Selection on the GPU") — [IEEE Xplore](https://ieeexplore.ieee.org/document/8733051). Relevance: the manuscript already cites the paper, but only for "filtered" search, not for its k-selection, which is the most relevant part.

GPU vector-search systems:
- Ootomo, Naruse, Nolet, Wang, Feher, Wang. CAGRA: Highly Parallel Graph Construction and Approximate Nearest Neighbor Search for GPUs. ICDE 2024, pp. 4236–4247. doi 10.1109/ICDE60146.2024.00323 — [computer.org](https://www.computer.org/csdl/proceedings-article/icde/2024/1.715E241/1YOtVakjDy0); arXiv:2308.15136 notes "For a single query, our method is 3.4~53x faster than HNSW at 95% recall" — [arXiv](https://arxiv.org/abs/2308.15136). Relevance: state-of-the-art GPU ANN with explicit single-query (batch-1) results.
- NVIDIA cuVS library (CAGRA, IVF, brute_force) — [github.com/NVIDIA/cuvs](https://github.com/NVIDIA/cuvs). Relevance: the "tuned library kernels" that Section 10 names but does not cite.
- Groh, Ruppert, Wieschollek, Lensch. GGNN: Graph-Based GPU Nearest Neighbor Search. IEEE TBD 9(1):267–279, Feb 2023. doi 10.1109/TBDATA.2022.3161156 — [computer.org](https://www.computer.org/csdl/journal/bd/2023/01/09739943/1BWZbLa9H2w). Relevance: GPU graph ANN baseline.
- Zhao, Tan, Li. SONG: Approximate Nearest Neighbor Search on GPU. ICDE 2020, pp. 1033–1044. doi 10.1109/ICDE48307.2020.00094 — [Semantic Scholar](https://www.semanticscholar.org/paper/SONG:-Approximate-Nearest-Neighbor-Search-on-GPU-Zhao-Tan/91474bbe46200313bef7516cfa9a78258770c134). Relevance: early GPU graph ANN.
- Karthik V., Khan, Singh, Simhadri, Vedurada. BANG: Billion-Scale Approximate Nearest Neighbour Search Using a Single GPU. IEEE TBD 11(6):3142–3157, Dec 2025. doi 10.1109/TBDATA.2025.3581085 — [computer.org](https://www.computer.org/csdl/journal/bd/2025/06/11045134/27FMimdf8UU); [GitHub](https://github.com/karthik86248/BANG-Billion-Scale-ANN). Relevance: CPU/GPU split execution with PCIe-aware overlap.
- Douze et al. The Faiss Library. IEEE TBD 12(2), April 2026. doi 10.1109/TBDATA.2025.3618474 — [IEEE Xplore](https://ieeexplore.ieee.org/document/11202651). Relevance: the current FAISS reference, including IDSelector filtering and pre-/post-filter discussion.
- Mo, Karsin, Adams, Zhang. VecFlow-Chamfer. PACMMOD 4(1), Article 92, pp. 1–26, 2026. doi 10.1145/3786706 — [ACM DL](https://dl.acm.org/doi/10.1145/3786706). Relevance: secondary; GPU multi-vector search on Grace Hopper.
- Liu, Chen, Tian, et al. GPU-Accelerated Algorithms for Graph Vector Search: Taxonomy, Empirical Study, and Research Directions. arXiv:2602.16719, 2026 — [arXiv](https://arxiv.org/abs/2602.16719v1). Relevance: a recent GPU-ANN survey that reports transfer-dominated latency.

Exact / brute-force search on GPU (production):
- cuVS brute_force with bitset or bitmap `prefilter` — [cuVS docs](https://docs.rapids.ai/api/cuvs/stable/python_api/neighbors_brute_force/); [guide](https://docs.nvidia.com/cuvs/user-guide/api-guides/indexing-guide/brute-force). Relevance: the exact filtered GPU baseline.
- Milvus GPU_BRUTE_FORCE ("While there is no set limit for limit on GPU_BRUTE_FORCE, it is recommended not to exceed 4096") — [Milvus docs](https://milvus.io/docs/index-with-gpu.md); [GPU index overview](https://milvus.io/docs/gpu-index-overview.md). Relevance: a production exact GPU path.
- FAISS IDSelectorBitmap ("One bit per element") — [FAISS source](https://github.com/facebookresearch/faiss/blob/main/faiss/impl/IDSelector.h). Relevance: the CPU-side full-width-mask representation.

Filtered search (beyond those cited):
- Caminal, Chronis, Wu, Hellerstein(?), Papakonstantinou, Ozcan et al. Filtered Vector Search: State-of-the-art and Research Opportunities. PVLDB 18(12):5488–, 2025. doi 10.14778/3750601.3750700 — [ACM DL](https://dl.acm.org/doi/10.14778/3750601.3750700); [PDF](https://www.vldb.org/pvldb/vol18/p5488-caminal.pdf); [Google Research](https://research.google/pubs/filtered-vector-search-state-of-the-art-and-research-opportunities). Relevance: the canonical taxonomy (pre/post/inline, bitmap) to frame light vs heavy paths. Only Yannis Papakonstantinou and Fatma Ozcan were confirmed as authors; the "(?)" marks the unverified list, which must be verified before citing.
- Wang, Lv, Xu, Wang, Yue, Ni. NHQ: An Efficient and Robust Framework for Approximate Nearest Neighbor Search with Attribute Constraint. NeurIPS 2023. doi 10.52202/075280-0692 — [NeurIPS](https://proceedings.neurips.cc/paper_files/paper/2023/hash/32e41d6b0a51a63a9a90697da19d235d-Abstract-Conference.html)
- Wu, He, Qiao, Fu, Liu, Yu. HQANN: Efficient and Robust Similarity Search for Hybrid Queries with Structured and Unstructured Constraints. CIKM '22. arXiv:2207.07940 — [arXiv](https://arxiv.org/abs/2207.07940v1); listed in [CIKM '22 proceedings](https://dl.acm.org/doi/proceedings/10.1145/3511808?tocHeading=heading4) (the DOI suffix 3557610 appears in the TOC excerpt but is unconfirmed for HQANN).
- Li, Huang, Ding, Park, Chen. SIEVE: Effective Filtered Vector Search with Collection of Indexes. PVLDB 18(11):4723–4736, 2025. doi 10.14778/3749646.3749725 — [ACM DL](https://dl.acm.org/doi/10.14778/3749646.3749725). Relevance: uses an analytical cost model to choose among indexes per query, a router analogue.
- Cai, Shi, Chen, Zheng. Navigating Labels and Vectors: A Unified Approach to Filtered ANN Search (UNG). PACMMOD 2(6), 2024. doi 10.1145/3698822 — [ACM DL](https://dl.acm.org/doi/abs/10.1145/3698822)
- Gupta et al. CAPS: A Practical Partition Index for Filtered Similarity Search. arXiv:2308.15014, 2023 — [arXiv](https://arxiv.org/abs/2308.15014)
- Xu, Gao, Gou, Long, Jensen. iRangeGraph: Improvising Range-dedicated Graphs for Range-filtering Nearest Neighbor Search. PACMMOD 2(6), 2024. doi 10.1145/3698814 — [ACM DL](https://dl.acm.org/doi/10.1145/3698814). Relevance: Qenlo's filter is a timestamp range, so range-filtered ANN is the closest problem class.
- Engels, Landrum, Yu, Dhulipala, Shun. Approximate Nearest Neighbor Search with Window Filters. ICML 2024, PMLR 235 — [PMLR](https://proceedings.mlr.press/v235/engels24a.html). Relevance: "image and document search with timestamp filters" is exactly Qenlo's workload shape. (The author list was not shown in the excerpt; verify it.)
- Wang, Zhang, Hu. WoW: A Window-to-Window Incremental Index for Range-Filtering ANN Search. PACMMOD 3(6), 2025. doi 10.1145/3769843 — [arXiv:2508.18617](https://arxiv.org/abs/2508.18617); DOI per [ACORN cited-by](https://dl.acm.org/doi/10.1145/3654923)
- Zuo, Qiao, Zhou, Li, Deng. SeRF: Segment Graph for Range-Filtering ANN Search. PACMMOD 2(1), Article 69, 2024 — as cited in [Iff et al. references](https://dl.acm.org/doi/10.1145/3805712.3809731)
- Production filtering: pgvector 0.8.0 iterative index scans (`hnsw.iterative_scan`, `ivfflat.iterative_scan`), released 2024-11-11 to prevent "overfiltering" — [PostgreSQL news](https://www.postgresql.org/about/news/pgvector-080-released-2952/); Weaviate `acorn` filter strategy based on Patel et al., default from v1.34 — [Weaviate docs](https://docs.weaviate.io/weaviate/concepts/filtering), [GitHub docs](https://github.com/weaviate/docs/blob/main/docs/weaviate/concepts/filtering.md)

CPU/GPU placement and adaptive routing in databases:
- Breß. Why it is time for a HyPE: A Hybrid Query Processing Engine for Efficient GPU Coprocessing in DBMS. VLDB 2013 PhD Workshop (PDF hosted under PVLDB vol. 6, p. 1398) — [PDF](https://www.vldb.org/pvldb/vol6/p1398-bress.pdf). Relevance: learned operator placement.
- Breß, Heimel, Siegmund, Bellatreche, Saake. GPU-accelerated Database Systems: Survey and Open Challenges. Trans. Large-Scale Data- and Knowledge-Centered Systems 15:1–35, 2014. doi 10.1007/978-3-662-45761-0_1 — DOI as listed in [Rosenfeld et al. references](https://dl.acm.org/doi/abs/10.1145/3485126?sid=SCITRUS); [PDF](https://www.se.cs.uni-saarland.de/publications/docs/BHS+14.pdf). (The co-author list beyond Breß is from memory and unverified.)
- Karnagel, Habich, Lehner. Adaptive Work Placement for Query Processing on Heterogeneous Computing Resources. PVLDB 10(7):733–744, 2017. doi 10.14778/3067421.3067423 — [PDF](https://www.vldb.org/pvldb/vol10/p733-karnagel.pdf). Relevance: run-time adaptive placement; names inaccurate cardinality as challenge C1.
- Rosenfeld, Breß, Markl. Query Processing on Heterogeneous CPU/GPU Systems. ACM Computing Surveys 55(1), Article 11, pp. 1–38, 2022. doi 10.1145/3485126 — [ACM DL](https://dl.acm.org/doi/abs/10.1145/3485126). Relevance: the standard survey of operator placement and cost models.
- He, Nakandala, Banda, Sen, Saur, Park, Curino, Camacho-Rodríguez, et al. Query Processing on Tensor Computation Runtimes (TQP). PVLDB 15(11):2811–2825, 2022. doi 10.14778/3551793.3551833 — [PDF](https://www.vldb.org/pvldb/vol15/p2811-he.pdf). Relevance: tensor-runtime GPU query processing.
- Raza et al. GPU-accelerated data management under the test of time. CIDR 2020 — [PDF](https://www.cidrdb.org/cidr2020/papers/p18-raza-cidr20.pdf). Relevance: retrospective on GPU DBMS data-transfer assumptions. (The author list beyond Raza was not verified.)
- Lutz et al. 2020 (NVLink vs PCIe for databases) and Kabić et al. 2025 (MaxBench) are cited by Mageirakos et al. as the interconnect studies — [arXiv HTML 2605.15957](https://arxiv.org/html/2605.15957v1). I did not verify their metadata independently.

WebGPU / WGPU compute performance:
- Maczan 2026, arXiv:2604.02344 and arXiv:2608.08730 (UK AI Conference 2026, PMLR), described in Q6 — [arXiv](https://arxiv.org/abs/2604.02344), [arXiv](https://arxiv.org/abs/2608.08730). Relevance: the only batch-1 wgpu-native/Vulkan dispatch-overhead characterization I found.
- Ruan et al. WebLLM: A High-Performance In-Browser LLM Inference Engine. arXiv:2412.15803, 2024; "can retain up to 80% native performance" — [arXiv HTML](https://arxiv.org/html/2412.15803). Relevance: WebGPU vs native gap. (The "Ruan" author name was not verified.)
- A wgpu-native compute experience report: a 1.65× slowdown vs a prior Vulkan implementation — [gfx-rs/wgpu discussion #6688](https://github.com/gfx-rs/wgpu/discussions/6688). Relevance: anecdotal, not peer-reviewed.

### Inferences
- Highest-priority additions: (1) AIR Top-K (SC'23) and RadiK (ICS'24), because both report batch-1 or non-batched top-k and show how far a tuned selection kernel is from the heavy path's k-pass single-workgroup design. (2) cuVS brute_force with prefilter as the realistic full-width-mask GPU baseline. (3) Caminal et al. VLDB 2025 for the pre/post/inline-filter taxonomy. (4) Karnagel 2017 and Rosenfeld et al. 2022 alongside Breß 2016 for placement. (5) Maczan 2026 for wgpu batch-1 dispatch overhead. (6) Window/range-filtered ANN (Engels ICML 2024, iRangeGraph, SeRF), because Qenlo's filter is a timestamp range.
- Several 2026 SIGMOD and PVLDB filtered-search papers appear in cited-by lists: CGIF (PVLDB 19(11)), FAVOR (SIGMOD 2026), VecBench (PACMMOD 4(3), doi 10.1145/3802125) and Curator (SIGMOD 2026). They show the area moved fast during 2026. They would strengthen "active area", but each needs its own verification before citing.

### Gaps
- Author lists not fully verified: AIR Top-K (SC'23), Dr. Top-k (beyond the first five names), Engels et al. (ICML 2024), Caminal et al. (VLDB 2025), Breß et al. 2014 co-authors, WebLLM.
- The HQANN DOI was not confirmed.
- I did not verify the "CAGRA filtered search" feature in cuVS docs, or any GGNN/SONG filtering support.
- I found no peer-reviewed WebGPU or wgpu general-compute benchmark study beyond the Maczan preprints and WebLLM. This may reflect search coverage rather than true absence.
