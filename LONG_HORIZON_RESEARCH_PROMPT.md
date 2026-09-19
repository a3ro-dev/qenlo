# Qenlo long-horizon research and documentation mission

Work autonomously in this repository for roughly 3–5 hours. Use the available time continuously: inspect, research, verify, edit, run checks, and iterate. Do not stop after producing a plan or an audit. Leave the repository materially closer to a defensible finished research project and paper.

## Mission

Determine the strongest claims Qenlo can honestly make, identify the highest-value changes needed to improve the system and its evidence, finish every research or writing task that can be completed from the current repository and available public information, and update the relevant documentation and paper sources so they agree.

The desired result is rigorous rather than promotional. Treat “prove SOTA” as a hypothesis to test and narrowly define, not a conclusion to assume. If Qenlo is not state of the art under a fair comparison, establish that clearly, identify the exact gap, and turn it into the smallest credible research plan or implementation change. A negative result is useful. Unsupported superlatives are failure.

## Current repository context

Start by reading the repository rather than relying on this summary. At minimum inspect:

- `README.md`, `PRODUCT.md`, `DESIGN.md`, and `IDEAS.md`
- `docs/benchmark-protocol.md`, `docs/prioritized-roadmap.md`, `docs/implementation-status.md`, `docs/verification.md`, `docs/feature-matrix.md`, and `docs/trade-offs.md`
- `research/README.md`, `research/completion-audit.md`, `research/claim-artifact-ledger.md`, `research/experiment-plan.md`, `research/methodology-audit.md`, and `research/analysis.md`
- `paper/paper.tex`, `paper/appendix.tex`, `paper/references.bib`, `paper/README.md`, `paper/REPRODUCE.md`, and the paper audit files
- implementation, benchmarks, raw evidence, processed evidence, scripts, and tests referenced by those files
- `git status`, the current diff, recent relevant history, and any repository instructions

The repository currently contains uncommitted paper and research work. Preserve it. Do not reset, discard, overwrite, or casually reformat existing changes. Understand the diff before editing overlapping files.

Existing evidence appears to support a useful negative result: neither eligible count nor `E*D*B` is a universal routing scalar on the retained cells, and the held-out scalar router failed. It does not yet establish a validated multidimensional production router or a universal performance lead. Verify this yourself.

## Non-negotiable research rules

1. Every empirical claim must point to a retained artifact, reproducible command, primary source, or explicitly identified new measurement.
2. Keep incompatible cohorts separate. Do not pool results across different hardware, source revisions, APIs, timing boundaries, datasets, or correctness requirements.
3. Distinguish algorithmic exhaustiveness, numerical agreement, recall, deterministic ordering, storage durability, and production readiness.
4. Compare systems only when data, filters, `k`, timing boundaries, correctness gates, and resource scopes are genuinely comparable. Otherwise report separate rows and explain the mismatch.
5. Preserve failed, unavailable, and negative results. Never select only favorable cells.
6. Do not infer causality from cross-environment comparisons.
7. Do not invent citations, benchmark values, experiment runs, user adoption, hardware validation, or implementation status.
8. Prefer primary sources: papers, official documentation, standards, source repositories, release notes, and original benchmark artifacts. Record URLs, access dates when useful, versions or commits, and what each source actually supports.
9. Treat blogs, vendor claims, and benchmark charts as leads. Do not use them as independent proof without checking the underlying method.
10. Do not spend money, provision cloud hardware, publish externally, push, release, or contact people. If a decisive experiment requires unavailable hardware or paid infrastructure, specify the exact command, predicted result, decision rule, estimated cost, and artifact schema instead of pretending it ran.

## What “SOTA” must mean here

Before searching for supportive evidence, write a falsifiable SOTA claim matrix. Candidate axes include:

- embedded/local versus client-server deployment;
- exact filtered vector search versus ANN;
- durable canonical storage versus an in-memory or derived index;
- completed-call latency versus kernel-only latency;
- small and medium local collections versus million-scale workloads;
- metadata-filter selectivity, dimension, batch size, `k`, mutation state, memory, build time, and energy;
- correctness, recoverability, portability, inspectability, and ease of integration.

For each candidate claim, state:

- the comparison set and why it is the right one;
- the metric and timing boundary;
- the correctness and statistical gate;
- the workloads on which the claim applies;
- the evidence already present;
- the strongest counterevidence;
- the missing experiment that could falsify it; and
- one verdict: `supported`, `promising`, `unsupported`, `falsified`, or `not comparable`.

Do not use “SOTA,” “fastest,” “best,” “production-ready,” or equivalent language in public documentation unless a claim is `supported` under a defined comparison set and reproducible protocol. Prefer a narrower claim that survives scrutiny. Qenlo may be most defensible as an embedded, durable, exact, inspectable system rather than the fastest vector engine; test that thesis rather than assuming it.

## Work loop

Maintain a concise working ledger in `research/long-horizon-ledger.md`. Each entry should contain the question, evidence inspected, result, confidence, affected claims, files changed, and next action. Update it as work proceeds so another researcher can resume without reconstructing your reasoning.

Repeat this loop until the time budget is nearly exhausted:

1. Select the unresolved question with the greatest expected effect on paper validity, SOTA credibility, or product direction.
2. Inspect the relevant code, artifacts, and existing prose end to end.
3. Search current literature and competitor documentation. Use primary sources and verify versions and dates.
4. Try to disprove the proposed conclusion. Search for stronger baselines, mismatched boundaries, missing negative cells, data leakage, post-hoc tuning, invalid statistics, and claims that exceed the evidence.
5. Run the smallest existing local analysis or verification command that resolves the question. Reuse repository scripts before writing code. Add code only when a small, necessary analysis cannot be expressed with what exists.
6. Record the evidence and verdict in the ledger and claim-to-artifact machinery.
7. Immediately update affected paper and documentation text. Do not defer all writing to the end.
8. Re-run the narrowest relevant check, inspect the diff, and continue.

Spend no more than about 20 minutes on passive reading without producing a ledger entry, a verified finding, an edit, or a runnable next step. Do not repeatedly summarize the same files.

## Priority questions

Work in evidence-value order, adjusting when the repository reveals a better sequence.

### 1. Claim and paper integrity

- Audit every headline number and broad conclusion in the abstract, introduction, conclusion, README, and evidence pages against the claim ledger and retained artifacts.
- Resolve contradictions among the paper, appendix, README, completion audit, implementation status, roadmap, and generated PDF.
- Check whether the manuscript clearly separates retained implementation measurements, rejected candidates, historical revisions, external engines, and unavailable cells.
- Check citations against the source they are claimed to support. Add or correct references only after verification.
- Decide whether the paper’s actual contribution is sufficiently novel and clearly stated: a systems characterization and held-out falsification of scalar routing rules, with conditional execution factors and evidence boundaries.

### 2. Current field and closest comparisons

- Build a current competitor and related-work map for embedded/local vector search, exact filtered search, filtered ANN, CPU/GPU routing, query optimization, and persistent vector storage.
- Include the closest credible systems and libraries, not merely famous ones. Investigate at least sqlite-vec and the relevant modes of FAISS, USearch, hnswlib, LanceDB, Chroma, DuckDB vector search/VSS, and other directly comparable current systems found during research.
- Separate database behavior from raw index or tensor-library behavior.
- Identify which comparisons can be made from retained evidence, which require a local adapter/run, and which are structurally not comparable.
- Check whether newer work invalidates the novelty framing or supplies a stronger baseline, routing method, cost model, filtering technique, or benchmark protocol.

### 3. Highest-value missing evidence

Rank gaps by information gain divided by cost. The repository already points to likely gaps; verify them:

- no controlled sqlite-vec result;
- no validated multidimensional end-to-end router;
- no same-host fixed-eligible-count/varying-total-count study;
- incomplete dimension, metadata-correlation, and distinct-filter-batch ablations;
- no ANN recall-latency frontier, only operating points;
- limited optimized CPU-library comparison;
- incomplete physical mobile/MPS and cross-vendor evidence;
- incomplete energy, concurrency, mutation-churn, and crash-schedule evidence;
- external integration, compatibility-policy, and public evidence-page gaps.

For the top gaps, determine whether existing artifacts permit a valid reduction now. If yes, perform it and retain outputs. If no, write a preregistered experiment specification with hypotheses, factors, frozen baselines, held-out split, correctness oracle, statistical analysis, failure retention, stop rule, resource estimate, commands, and expected artifact paths. Do not create a giant Cartesian matrix when a small discriminating experiment can answer the question.

### 4. Product and implementation improvement

- Trace measured bottlenecks to actual implementation paths before suggesting changes.
- Prefer changes that improve both product behavior and evidentiary clarity: observability, stable timing boundaries, reproducible adapters, hardware-conditioned profiles, safe fallback, and artifacts that identify source/device/runtime state.
- Distinguish ideas supported by profiling from speculation.
- Rank recommendations by expected impact, evidence strength, effort, risk, and the experiment that decides whether to ship them.
- Implement only small, clearly justified, locally verifiable fixes that unblock the research or correct a proven defect. Avoid speculative architecture and new dependencies.

## Required repository outputs

Create or update the smallest coherent set of files. At minimum, leave:

1. `research/long-horizon-ledger.md` — chronological, resumable research record.
2. `research/sota-claim-matrix.md` — scoped claims, comparison sets, evidence, counterevidence, and verdicts.
3. `research/gap-ranked-plan.md` — ranked research and product gaps with exact next experiments or implementation steps.
4. `research/related-work-current.md` — current primary-source landscape and its effect on novelty and baselines.
5. Updated claim-to-artifact records for any claim that changed.
6. Updated `paper/paper.tex`, `paper/appendix.tex`, and `paper/references.bib` where the evidence or current literature requires changes.
7. Updated public and internal prose wherever it would otherwise contradict the final conclusions, especially `README.md`, `docs/verification.md`, `docs/prioritized-roadmap.md`, `docs/implementation-status.md`, `research/README.md`, `research/completion-audit.md`, `paper/README.md`, and `paper/REPRODUCE.md`.

Do not edit a file merely to say it was reviewed. Avoid duplicated essays across the repository. Pick one canonical home for each detailed conclusion and link to it elsewhere.

## Validation

Use existing repository workflows. Start narrow, then run the complete offline paper workflow if relevant inputs changed:

```powershell
python paper/scripts/verify_campaign_claims.py
python paper/audit/verify_historical_raw.py
python paper/scripts/reduce_final_evidence.py
python paper/scripts/inventory_evidence.py
python research/scripts/analyze_full_archive.py
python paper/scripts/generate_final_tables.py
python paper/scripts/generate_final_figures.py
python paper/scripts/assemble_claim_ledger.py
python paper/scripts/build_final_paper.py
python paper/scripts/verify_final_pdf.py
```

Run code tests only for code you changed or claims they materially verify. If the final PDF changes, inspect every changed rendered page for clipping, table and figure legibility, broken references, misleading captions, and page-break regressions. Record commands, exit status, and material failures. A failed command is evidence; diagnose it and do not describe it as passing.

Before finishing:

- inspect `git diff --check`, `git status`, and the complete diff;
- confirm no raw artifacts or user changes were overwritten;
- search for stale statements contradicted by the final verdicts;
- verify all new citations and links;
- ensure generated files correspond to their sources;
- remove scratch files and unsupported prose; and
- leave explicit blockers only when they depend on unavailable hardware, money, credentials, external users, or publication access.

## Final response

Report completed work, the strongest defensible conclusion, the SOTA verdict by scope, decisive evidence, files changed, verification performed, unresolved blockers, and the next three actions ranked by information gain. State plainly what Qenlo still cannot claim.

Do not end with a generic roadmap or a request for further instructions. Finish everything locally achievable first.
