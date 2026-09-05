# Use cases

Qenlo is for local applications that need durable vector records and exact search over a metadata-filtered candidate set. Its strongest product shape is closer to an embedded database library than a vector-search service.

## Desktop and offline retrieval

Use Qenlo for note search, document retrieval, code search, or local RAG when the index should live beside the application. There is no service to deploy, and search does not require network access. This fit is strongest when the host owns precomputed embeddings and exact results are preferable to ANN tuning.

## Application and agent memory

Atomic batches, tombstones, durable reopen, and time-range filtering support episodic or working-memory stores. A host can restrict retrieval by user and time without maintaining a second vector index for every partition.

Qenlo does not provide authentication or authorization. The application must enforce who may select a user or predicate.

## Per-user local collections

Applications can isolate users with separate collection directories or use the built-in user filter. Separate collections provide a clearer storage boundary; filters are convenient when one trusted process owns the dataset.

## Exact filtered retrieval

Qenlo evaluates eligible live rows and returns exhaustive top-k results within the selected exact engine. This is attractive when predicates are selective enough that ANN adds complexity without useful latency savings.

“Exact” means exhaustive candidate coverage. FP32 storage and backend arithmetic can still differ from an FP64 oracle near ties.

## Optional local acceleration

WGPU can accelerate dense or batched work on supported adapters. PyTorch is useful when the host already owns a tensor runtime. Both remain derived execution state around the canonical collection.

Do not choose a route from collection size alone. Eligibility, dimension, batch size, representation, preparation cost, selection, and device state all matter. Measure the retained build on the deployment host.

## Device and systems research

The benchmark harness, independent oracle, diagnostics, and failure-preserving reports make Qenlo useful for studying exact vector-search execution across devices. This is currently a stronger claim than universal performance portability.

## When Qenlo is the wrong tool

Choose another system when you need SQL and joins, multi-process writers, a remote shared service, replication, sharding, an operational SLA, billion-scale ANN, embedding generation, built-in encryption, or mature production support.

PostgreSQL with pgvector is a better fit for relational applications. A managed or distributed vector database is a better fit for shared services. FAISS, cuVS, or another search library may be simpler when persistence and mutation semantics are unnecessary.
