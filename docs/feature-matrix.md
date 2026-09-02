# Feature Comparison Matrix

A technical evaluation of **Qenlo** against primary alternatives in the vector database and search ecosystem:
* **pgvector (PostgreSQL)** — Relational database extension integrating vector operations into enterprise ACID SQL.
* **Milvus / Pinecone** — Dedicated distributed / managed cloud vector databases engineered for high-throughput, multi-tenant, billion-scale clustering.
* **Chroma / SQLite-VSS** — Embedded, lightweight vector stores focused on developer ergonomics and local prototyping.

---

## 1. Architecture, Operations & Deployment

| Feature / Dimension | **Qenlo** | **pgvector (PostgreSQL)** | **Milvus / Pinecone** | **Chroma / SQLite-VSS** |
| :--- | :--- | :--- | :--- | :--- |
| **Primary Deployment Model** | **In-Process Embedded** (Rust core + native C-ABI bindings) | Client-Server Relational Database (SQL engine) | Distributed multi-node cluster (Milvus) / Managed Cloud SaaS (Pinecone) | In-Process / Local Daemon (Python/JS wrappers) |
| **Operational Overhead** | **Zero** (no daemon, no socket, no background service) | **High** (Postgres daemon, connection pools, VACUUM tuning, migrations) | **High** (Etcd, MinIO, Pulsar/Kafka, K8s) or SaaS vendor lock-in | **Minimal to Low** (Embedded SQLite or local server) |
| **Storage Architecture** | Single-file container (`.qn` / snapshot + append WAL) | Relational heap tables + WAL + Postgres Index pages | Distributed segment files on Object Storage (S3/MinIO) + LSM | SQLite `.db` file or parquet/duckdb directory |
| **Query Interface** | Native SDK APIs (Rust, Python, TS, Go, Kotlin, Swift) | SQL (`SELECT ... ORDER BY vector <-> query LIMIT k`) | gRPC / REST API / Vector Query DSL | Python / JS SDK (`collection.query(...)`) |
| **Native Edge / Mobile Support** | **Tier 1 First-Class** (iOS XCFramework, Android JNI, macOS ARM64, Linux, Windows) | **None** (Server-only) | **None** (Cloud / Server-only) | **Partial / Fragile** (SQLite-VSS custom C-exts; Chroma mobile limited) |

---

## 2. Indexing, Hardware Acceleration & Performance

| Capability | **Qenlo** | **pgvector (PostgreSQL)** | **Milvus / Pinecone** | **Chroma / SQLite-VSS** |
| :--- | :--- | :--- | :--- | :--- |
| **Exact (Brute-Force) Engine** | AVX2 / NEON runtime SIMD auto-vectorized + independent float64 oracle | Sequential scan with CPU SIMD distance functions | Brute-force execution on query nodes | CPU loop / Faiss flat index |
| **ANN Index Types** | USearch HNSW adapter + GPU IVF-Flat & IVF-SQ8 | HNSW, IVFFlat, HNSW-PQ | HNSW, IVF-PQ, SCaNN, DiskANN, GPU-IVF | HNSWlib (Chroma), Faiss Flat/IVF (SQLite-VSS) |
| **GPU Acceleration Backends** | **Portable WGPU** (Vulkan, DX12, Metal) + CUDA kernels | **None** (CPU-only execution) | **Dedicated CUDA / cuVS / TensorRT** (Milvus GPU nodes) | **None / Very Limited** (CPU bound) |
| **Quantization Support** | GPU IVF-SQ8 (8-bit scalar quantization + FP32 rerank) | Scalar / Product Quantization (newer pgvector) | SQ8, PQ, BFloat16, FP16, Binary, DiskANN compression | Limited (HNSWlib default FP32, SQLite-VSS Faiss PQ) |
| **Batch Vector Processing** | Native multi-query GPU batching (up to 128 queries per dispatch) | Query-level Postgres parallel workers | High-throughput distributed batch pipelines | Query-level looping |

---

## 3. Metadata Filtering & Correctness Guarantees

| Filtering Characteristic | **Qenlo** | **pgvector (PostgreSQL)** | **Milvus / Pinecone** | **Chroma / SQLite-VSS** |
| :--- | :--- | :--- | :--- | :--- |
| **Filter Model** | Compound pre-filter bitmask (`user_id` + `i64` timestamp range + 64-bit flags) | Arbitrary SQL expressions (`WHERE user_id = 42 AND created_at > ...`) | Boolean scalar expressions / inverted index tags | Metadata dict expressions (`{"$and": [...]}`) |
| **Recall Under High Selectivity (<1%)** | **100% Deterministic Recall** (Pre-filtering scans exact eligible bitmask) | Can suffer index scan failure / fallback to slow sequential scan unless filtered indexes exist | Filtered graph traversals may suffer recall loss or require fallback bitsets | Post-filtering drops candidate count, causing recall collapse unless exact scan is triggered |
| **Tombstones & Deletions** | Canonical tombstones in generation snapshot; ANN cannot revive deleted rows | Vacuum-based dead tuple reclamation | Dynamic bitset soft-delete + compaction pipelines | In-memory ID tracking + periodic graph rebuild |
| **Correctness Authority** | Continuous validation against independent float64 CPU oracle | PostgreSQL test suite | Distributed integration test suite | Basic unit test assertions |

---

## 4. Durability, Concurrency & Memory Model

| Characteristic | **Qenlo** | **pgvector (PostgreSQL)** | **Milvus / Pinecone** | **Chroma / SQLite-VSS** |
| :--- | :--- | :--- | :--- | :--- |
| **ACID & Durability** | Atomic generation snapshot + checksummed WAL + publication watermark | Full enterprise ACID MVCC transactions | Eventual / Tunable consistency (Bounded, Strong, Session) | Atomic SQLite WAL or single-thread file write |
| **Concurrency Model** | Single-handle `Arc<Collection>` read-write lock + OS-level directory exclusivity | Multi-process connection pool with row/table level locks | Stateless distributed query nodes + separated write/log brokers | In-memory mutex / SQLite single-writer lock |
| **Memory Footprint & Cold Start** | **Extremely Low (~10-50MB base)**; zero-copy mmap decoded row slices; 512MB default admission budget | **Moderate to High** (PostgreSQL daemon `shared_buffers` + process memory) | **Very High** (Multiple GBs required for cluster runtime, JVM/Go/C++ services) | **Moderate** (Python runtime RSS + HNSW graph memory) |
| **Disaster Recovery** | Auto-validation on restart; corrupt/partial staging files rejected cleanly | Postgres WAL replay and point-in-time recovery (PITR) | Distributed segment recovery from MinIO / cloud storage | File replacement / SQLite database recovery |

---

## 5. Decision & Architectural Fit

### Choose Qenlo If:
* **In-process & Edge AI:** You need sub-millisecond cold starts on client devices (iOS, Android, macOS, Windows) or local daemons without deploying database containers.
* **Strict Filtered Retrieval:** Your workload requires selective partition filtering (e.g., per-user multi-tenancy, time-series windows, flag bitmasks) where graph-based ANN databases suffer severe recall collapse.
* **Heterogeneous GPU/SIMD Acceleration:** You want out-of-the-box Vulkan, DX12, Metal, and AVX2/NEON acceleration across desktop and mobile GPUs.
* **Single-File Artifact Portability:** You want simple backups, cloud sync, or air-gapped deployments where your entire vector database is a standalone `.qn` file.

### Choose pgvector If:
* **Unified Relational Workloads:** Your vectors are an attribute of existing relational entities, and queries require multi-table SQL `JOIN`s, foreign keys, and complex aggregate expressions.
* **Established PostgreSQL Infrastructure:** Your team already runs and manages production PostgreSQL clusters and wants to avoid introducing new storage infrastructure.

### Choose Milvus / Pinecone If:
* **Billion-Scale Multi-Node Sharding:** Your dataset spans 500M–10B+ vectors requiring horizontal partitioning across distributed Kubernetes nodes.
* **Fully Managed Cloud SaaS:** You require a hosted API with zero infrastructure maintenance, automatic scaling, and multi-region replication.

### Choose Chroma / SQLite-VSS If:
* **Rapid Python/RAG Prototyping:** You are developing proof-of-concept LangChain/LlamaIndex pipelines and prioritize ease of installation (`pip install chroma`) over strict durability and native mobile binaries.
