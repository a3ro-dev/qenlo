# Qenlo prioritized roadmap

Status: active planning document. Evidence reviewed September 7, 2026.

## Objective

Make Qenlo the credible default for embedded, durable, inspectable vector search. The next milestone is not another broad performance claim. It is a release that a developer outside this repository can discover, integrate, verify, and keep using.

Priorities are ordered by expected credibility and adoption impact, then by effort, dependencies, and reversibility. A lower tier does not start merely because it is interesting; its entry gate must be satisfied.

## What changed since the original assessment

Several items that looked open in the earlier positioning report are already complete in the current repository and should not consume another roadmap cycle.

| Earlier action | Current evidence | Disposition |
| --- | --- | --- |
| Add a telemetry opt-out | The [feature matrix](feature-matrix.md) says core and SDKs make no automatic network requests. [Device-lab upload](device-lab.md) is explicit and host-controlled; the collector is a separate service. | Complete. Preserve this privacy boundary with regression tests and release review. |
| Publish the paper/writeup | A [verified 22-page PDF](../QENLO-RESEARCH-PAPER.pdf), source, figures, claim ledger, and [reproduction instructions](../paper/REPRODUCE.md) are committed. | Repository publication complete. A DOI, arXiv entry, or external engineering article remains optional distribution work. |
| Benchmark LanceDB | The retained [100K x 384 replay](../benchmarks/runpod/2026-09-02-bb51987/README.md) includes LanceDB Flat with raw artifacts and disclosed timing boundaries. | Complete for one historical cohort. Do not imply broad coverage. |
| Benchmark sqlite-vec | No retained sqlite-vec adapter or result was found. | Open. This is the remaining closest-competitor gap. |
| Close the 1M x 768 gate | The [strict A6000 experiment](reports/2026-09-02-strict-topk-research-gate.md) ran and failed the pre-registered 2x CPU gate; Qenlo's prototype also lost to FAISS GPU Flat in that cell. | Complete as a negative result. Publish the verdict; do not rerun until a named implementation change creates a falsifiable new hypothesis. |
| State a v1 format | [`.qn` v1](qn-format-v1.md) is implemented, documented, and covered by round-trip, version, checksum, tombstone, and non-overwrite tests. | Technical format complete. Add a user-facing compatibility promise before calling it stable. |

## Priority summary

| Rank | Outcome | Impact | Effort | Main dependency |
| ---: | --- | --- | --- | --- |
| 1 | Ship one useful Python ecosystem integration and recruit an external design partner | Very high | Medium | Stable Python collection API and a narrow metadata contract |
| 2 | Add sqlite-vec to the controlled benchmark harness and publish the result | High | Medium | Comparable corpus, filter semantics, timing boundary, and oracle |
| 3 | Publish the compatibility policy for `.qn` v1 and prerelease APIs | High | Low | A deliberate promise the project can maintain |
| 4 | Turn the paper and failed scale gate into a concise public evidence page | High | Low/medium | Claims linked to retained artifacts |
| 5 | Produce installable, signed Android and Apple test releases and obtain current-revision device runs | High | High/external | Signing identities, store/test distribution, physical devices |
| 6 | Run a small, gated launch after ranks 1–5 have visible proof | High | Medium | Install path, integration example, competitive result, device evidence |
| 7 | Add at-rest encryption only from a written threat model and user demand | Medium | High | Key ownership, recovery, migration, and performance requirements |
| 8 | Execute NPU research only when devices, SDK access, and energy measurement exist | Uncertain | High | ANE/QNN/NeuroPilot access and a preregistered gate |
| 9 | Build a flagship local-first application only after an integration has real users | Medium | High | A validated workflow worth demonstrating |

## Tier 1: adoption and trust release

These items can proceed in parallel, but the integration is the release's organizing deliverable.

### 1. Ship one external integration

Start with one Python adapter, not four shallow integrations. Qenlo already has a published Python package, and current vector-store frameworks expect adapters to cover document insertion, deletion, similarity search, IDs, metadata, and score semantics. Implement the smallest honest mapping first; do not pretend Qenlo supports arbitrary metadata when its canonical filter model is user ID plus timestamp.

Choose between LangChain and LlamaIndex using one criterion: a real design partner agrees to run the example in their application. If neither has a user attached, publish a framework-neutral `qenlo-retriever` example first and use it to recruit one.

Deliverables:

- an optional integration package or extra that does not add framework dependencies to core Qenlo;
- add, delete, reopen, similarity-search, and filtered-search examples;
- explicit distance/score conversion and metadata limitations;
- conformance tests against Qenlo's independent exact oracle;
- a clean-environment install test from published artifacts; and
- one named external repository, application, or design partner using it.

Done means the integration is installable and an external user has completed the documented workflow. A merged adapter with no outside run is distribution progress, not adoption proof.

### 2. Close the sqlite-vec comparison gap

Add sqlite-vec to the existing OSS replay rather than creating a separate promotional benchmark. Compare equivalent exact search first. If metadata filtering cannot be expressed with the same semantics and timing boundary, publish separate end-to-end and prefiltered rows instead of collapsing them.

Required evidence:

- pinned sqlite-vec and binding versions;
- identical vectors, query set, `k`, and distance semantics;
- independent-oracle recall and filter-violation checks;
- build/load time, database size, peak RSS, and P50/P95/P99 completed-call latency;
- raw samples, environment, seeds, hashes, failure records, and adapter source; and
- a table that retains competitor wins and labels non-equivalent API boundaries.

The existing LanceDB result is historical evidence from one 100K x 384 cohort. Re-run LanceDB only if the sqlite-vec campaign can include it at little incremental cost and the revision/timing boundary is identical.

### 3. Publish a compatibility policy

Separate three promises that alpha software often conflates:

1. `.qn` portable interchange compatibility;
2. live directory/WAL storage compatibility; and
3. Rust, C ABI, and language-SDK API compatibility.

The first can be stronger than the others. State how long v1 readers and writers will be supported, what an unknown version does, whether migration tools will be provided, and which prerelease APIs may still break. Add a fixture produced by the released version to CI and require future versions to import it.

Done means a user can answer, from one page, whether upgrading Qenlo can strand a `.qn` file or a durable collection. Merely documenting the byte layout is not the full stability promise.

### 4. Publish the evidence page

Create one short public entry point that links to the paper, strict-gate addendum, retained competitor artifacts, device-lab status, and known limitations. Lead with the negative 1M x 768 result: the gate was run, exactness held, and the speed hypothesis failed on the tested cohort.

Do not submit to arXiv only to acquire a badge. Use arXiv or a DOI-bearing archive if the manuscript fits its scope and metadata requirements; otherwise publish a durable engineering article that makes every number traceable to the repository.

Done means a reader can verify each headline claim without navigating the internal research tree.

## Tier 2: mobile proof

Begin after the Tier 1 release candidate is usable. Mobile work should prove the portable product story, not merely produce CI artifacts.

### 5. Signed packages and physical-device runs

Ship the smallest trustworthy distribution path for each platform:

- Android: stable release signing, an installable tester or library artifact, and at least one current-revision Snapdragon and one MediaTek run;
- Apple: signed Apple-silicon macOS and iOS builds, one simulator check, and at least one physical iPhone/iPad run; and
- all reports: package hash, Qenlo revision, OS, SoC, backend, thermals, correctness result, fallback status, and retained raw samples.

Signing secrets remain outside the repository. Built, signed, installed, and physically validated are separate statuses and must stay separate in the feature matrix.

Done means an external tester can install a signed artifact and reproduce the canonical add, filter, exact-search, delete, close, and reopen workflow on both Android and iOS.

## Tier 3: launch

### 6. Spend the launch moment only after proof exists

The launch gate is conjunctive:

- one externally exercised integration;
- one controlled sqlite-vec result;
- a published compatibility policy;
- a concise evidence page with the failed scale gate visible; and
- signed Android and Apple artifacts with current-revision device evidence.

When all five are true, prepare one reproducible five-minute demo and one message: embedded, durable, exact retrieval whose execution path and evidence are inspectable. Avoid “fastest,” “production-ready,” “SQLite replacement,” and universal cross-platform performance claims.

Measure launch success by clean installs, completed example runs, integration users, issue quality, and retained users after 30 days—not impressions or repository stars alone.

## Tier 4: demand-gated differentiators

### 7. At-rest encryption

Do not start with “wrap the snapshot in AES.” First specify protected files, key ownership, nonce strategy, WAL and temporary-file behavior, crash recovery, rotation, backup/restore, metadata leakage, and migration from plaintext. Require at least two prospective users with a shared threat model before implementation.

### 8. NPU execution research

Run only with physical devices, vendor SDK access, and credible energy measurement. Pre-register accuracy, P95 latency, energy, memory, and integration-cost gates. A compile success or vendor detection is not a result.

### 9. Flagship local-first application

Use the first integration's real workflow as the product brief. The application should demonstrate persistence, deletion, offline operation, inspectable routing, and recovery. Do not create a generic chat shell merely to display Qenlo's name.

## Explicit non-goals

Do not build replication, sharding, multi-tenancy, a hosted control plane, arbitrary SQL, or a managed service without repeated external demand. These features would change the product, threat model, durability protocol, and operating model rather than deepen the current embedded niche.

Do not invest in native CUDA/HIP/Metal/Vulkan kernels merely because the portable backend loses a benchmark. Require a controlled result that identifies a material portable-runtime ceiling and a target workload valuable to actual users.

Do not reopen the failed 1M x 768 gate without a named implementation change, predicted effect, frozen baseline, and preregistered decision rule.

## Operating cadence

Keep one public ledger with `planned`, `in progress`, `blocked`, `complete`, or `rejected` status. Every completed item links to code, tests, an installable artifact, or retained measurements. Review priorities after each external user run; do not use calendar dates as evidence of readiness.

The next three concrete moves are:

1. recruit one Python design partner and select the integration from that user's stack;
2. add a sqlite-vec adapter and protocol-compliant replay cell; and
3. write the compatibility policy plus a released `.qn` fixture test.
