# Experiment plan

Primary question: when metadata predicates reduce the eligible search set, when is exact search preferable to ANN?

H1: selective predicate-aware exact GPU search can beat exact CPU and approach or beat high-recall HNSW while retaining recall 1.0. H2: post-filter eligible count dominates total corpus size. H3: CPU wins below a GPU fixed-cost crossover. H4: batching moves that crossover downward. H5: HNSW's advantage shrinks at high recall and high selectivity. H6: an eligible-count-aware router can reduce fixed-policy regret.

The canonical matrix is the repository protocol (100k/1M rows; 384/768 dimensions; 100, 10, 1, 0.1, 0.01 percent eligibility; empty/fewer; batches 1/8/32; independent and correlated metadata). Because the $6 campaign budget and unavailable Vulkan ICD prevented the full matrix, this campaign prioritizes: retained real 100k×384 CPU/WGPU/USearch cells, retained real 1M×768 TopK CPU/FAISS exact cells, and fresh-seed synthetic 1M×768 CUDA-prototype/FAISS cells at E={1k,10k,100k,1M}.

Optimization is separated from evaluation: seed 20260902 is development/tuning; seed 20260903 is held out. Headline Rust comparisons use five complete repetitions and the repository's whole-run bootstrap. The CUDA matrix uses 50 warmups, five repetitions, shuffled queries, 500 samples per cell, and host-wall timing through synchronized ID readback.
