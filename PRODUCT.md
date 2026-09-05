# Product

## Positioning

Qenlo is a local-first, embedded vector store for durable exact retrieval over metadata-filtered collections. It gives an application one canonical record store and several optional execution paths without requiring a database server.

Qenlo is research-grade alpha software. Product copy must not present benchmark observations as universal performance claims or imply production validation that has not happened.

## Primary users

- Developers building desktop, offline, or air-gapped semantic retrieval.
- Systems engineers evaluating exact vector search on heterogeneous hardware.
- Researchers who need reproducible, correctness-gated execution records.

## Core promise

Canonical records decide what exists. Search indexes and accelerator buffers are derived and replaceable. A route change must not resurrect deleted data, bypass filters, or silently change requested failure behavior.

## Product boundaries

Qenlo stores precomputed vectors and limited metadata. It does not provide embedding generation, SQL, access control, encryption, replication, sharding, or a hosted service. GPU and ANN paths are optional. The host application owns runtime setup, authorization, deployment, and telemetry export.

## Communication principles

1. Lead with embedded durability and exact filtered search.
2. Name hardware, source revision, workload, timing boundary, and recall when quoting performance.
3. Report failed and unavailable routes as evidence.
4. Distinguish exhaustive coverage from FP64 numerical identity.
5. Describe automatic routing as unvalidated until held-out regret is measured.
6. Never imply that Qenlo CPU represents optimized CPU performance.

## Brand personality

Precise, candid, and quietly technical. The interface should feel like a trustworthy local instrument: clear about what happened, explicit about unsupported states, and restrained about experimental results.

Avoid generic AI imagery, unexplained benchmark scores, universal speed claims, and decorative infrastructure metaphors. Do not hide fallbacks, missing measurements, device failures, or evidence from superseded revisions.

## Accessibility

Target WCAG 2.2 AA for the web viewer and platform accessibility conventions for native shells. Do not rely on color alone. Support keyboard navigation, visible focus, assistive technologies, 200% zoom, and reduced motion.
