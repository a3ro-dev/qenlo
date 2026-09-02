# When to Use Qenlo (Pros & Cons)

Qenlo is designed for **embedded, deterministic, filtered vector search**. Like SQLite, it prioritizes in-process speed, zero configuration, single-file portability, and compound filtering over distributed clustering.

To help you decide if Qenlo is the right fit for your architecture, here is an honest evaluation of its strengths, sweet spots, trade-offs, and limitations.

---

## When to Use Qenlo (Pros & Sweet Spots)

### 1. In-Process & Zero Infrastructure Overhead
- **No Daemons or Microservices**: Embeds directly into your Python, Node/TS, Go, Rust, Kotlin, or Swift application via a native shared C-ABI.
- **Sub-Millisecond Cold Start**: Opens `.qn` files instantly via memory-mapping without warming up remote database connections or paying network round-trip penalties.

### 2. Strict Filtered Vector Search
- **Zero Recall Penalty on Filters**: In standard vector databases, applying selective filters (e.g. `user_id = 42` matching only 0.1% of records) breaks graph-based ANN traversals, causing high recall loss. Qenlo’s pre-filtering engine scans filtered bitmasks and partitions with exact recall and SIMD vector acceleration.
- **Compound Bitmask Filtering**: Filter simultaneously by partition ID, time range (`timestamp_min` / `timestamp_max`), and arbitrary 64-bit user flags (`flags_all_set`, `flags_any_set`, `flags_none_set`).

### 3. Single-File Portability (`.qn` Container)
- **SQLite-like Storage**: Your entire vector database, WAL log, and metadata live in a single `.qn` file.
- **Easy Backups & Synchronization**: Easily copy, sync over S3/Cloud Storage, or transfer vector spaces between edge devices and cloud backends.

### 4. Edge, Mobile & Air-Gapped Deployments
- **Runs Everywhere**: Full support for iOS (Swift XCFramework), Android (Kotlin JNI), macOS Apple Silicon (ARM64), Linux (x86_64), and Windows (x64).
- **Air-Gapped Ready**: Requires zero internet connection or telemetry to operate.

---

## When NOT to Use Qenlo (Cons & Limitations)

### 1. Multi-Node Distributed Clustering (Billion-Scale)
- **Single-Node Embedded**: Qenlo is designed to run in-process on a single node or edge device. 
- **Recommendation**: If your dataset exceeds hundreds of millions of vectors requiring horizontal sharding across a 50-node Kubernetes cluster, use distributed solutions like **Milvus**, **Qdrant Cluster**, or **Pinecone**.

### 2. Complex Relational ACID Joins Across Multiple Tables
- **Focused Vector Store**: Qenlo stores vectors with structured metadata headers (`user_id`, `timestamp`, `flags`). It does not implement a full relational SQL engine with multi-table joins, foreign keys, or complex transactions.
- **Recommendation**: If your queries require complex SQL joins across relational tables with embedded vector columns, use **PostgreSQL with pgvector**.

### 3. Pure Unfiltered ANN Without Payload Storage
- **Filter-Optimized Architecture**: Qenlo's layout is engineered for filtered retrieval. If your workload consists purely of global nearest-neighbor lookups with zero metadata filters where you only care about maximum raw graph hops in RAM, a pure graph library like **USearch** or raw **HNSWLib** may suffice.

---

## Detailed Comparison Matrix

### 1. Architecture, Operations & Deployment

| Feature / Requirement | **Qenlo** | **pgvector (PostgreSQL)** | **Milvus / Pinecone** | **Chroma / SQLite-VSS** |
| :--- | :--- | :--- | :--- | :--- |
| **Deployment Model** | In-Process Embedded (Rust / C-ABI) | Client-Server SQL Database | Distributed Cluster / Cloud SaaS | In-Process / Local Daemon |
| **Operational Overhead** | **Zero** (no daemon, no socket, no background service) | **High** (Postgres daemon, connection pool, tuning) | **High** (Etcd, MinIO, Pulsar/K8s) or Vendor SaaS | **Low** (Embedded SQLite or local server) |
| **Storage Architecture** | Single-file container (`.qn` / snapshot + append WAL) | Relational heap tables + WAL + Postgres Index pages | Distributed segment files on Object Storage (S3/MinIO) | SQLite `.db` file or parquet/duckdb directory |
| **Query Interface** | Native SDK APIs (Rust, Python, TS, Go, Kotlin, Swift) | SQL (`SELECT ... ORDER BY vector <-> query LIMIT k`) | gRPC / REST API / Vector Query DSL | Python / JS SDK (`collection.query(...)`) |
| **Native Edge / Mobile Support** | **Tier 1 First-Class** (iOS XCFramework, Android JNI, macOS ARM64, Linux, Windows) | **None** (Server-only) | **None** (Cloud / Server-only) | **Partial / Fragile** (SQLite-VSS custom C-exts; Chroma mobile limited) |

### 2. Indexing, Hardware Acceleration & Performance

| Capability | **Qenlo** | **pgvector (PostgreSQL)** | **Milvus / Pinecone** | **Chroma / SQLite-VSS** |
| :--- | :--- | :--- | :--- | :--- |
| **Exact (Brute-Force) Engine** | AVX2 / NEON runtime SIMD + independent float64 oracle | Sequential scan with CPU SIMD distance functions | Brute-force execution on query nodes | CPU loop / Faiss flat index |
| **ANN Index Types** | USearch HNSW adapter + GPU IVF-Flat / IVF-SQ8 | HNSW, IVFFlat, HNSW-PQ | HNSW, IVF-PQ, SCaNN, DiskANN, GPU-IVF | HNSWlib (Chroma), Faiss Flat/IVF (SQLite-VSS) |
| **GPU Acceleration Backends** | **Portable WGPU** (Vulkan, DX12, Metal) + CUDA kernels | **None** (CPU-only execution) | **Dedicated CUDA / cuVS / TensorRT** (Milvus GPU nodes) | **None / Very Limited** (CPU bound) |
| **Quantization Support** | GPU IVF-SQ8 (8-bit scalar quantization + FP32 rerank) | Scalar / Product Quantization (newer versions) | SQ8, PQ, BFloat16, FP16, Binary, DiskANN compression | Limited (HNSWlib default FP32, SQLite-VSS Faiss PQ) |
| **Batch Vector Processing** | Native multi-query GPU batching (up to 128 queries) | Query-level Postgres parallel workers | High-throughput distributed batch pipelines | Query-level looping |

### 3. Metadata Filtering & Correctness Guarantees

| Filtering Characteristic | **Qenlo** | **pgvector (PostgreSQL)** | **Milvus / Pinecone** | **Chroma / SQLite-VSS** |
| :--- | :--- | :--- | :--- | :--- |
| **Filter Model** | Compound pre-filter bitmask (`user_id` + time ranges + 64-bit flags) | Arbitrary SQL expressions (`WHERE user_id = 42 AND ...`) | Boolean scalar expressions / inverted index tags | Metadata dict expressions (`{"$and": [...]}`) |
| **Recall Under High Selectivity (<1%)** | **100% Deterministic Recall** (Pre-filtering scans exact eligible bitmask) | Can suffer index scan failure / fallback to slow sequential scan unless filtered indexes exist | Filtered graph traversals may suffer recall loss or require fallback bitsets | Post-filtering drops candidate count, causing recall collapse unless exact scan is triggered |
| **Tombstone / Deletion Semantics** | Canonical tombstones in generation snapshot; ANN cannot revive deleted rows | Vacuum-based dead tuple reclamation | Dynamic bitset soft-delete + compaction pipelines | In-memory ID tracking + periodic graph rebuild |
| **Correctness Authority** | Continuous validation against independent float64 CPU oracle | PostgreSQL test suite | Distributed integration test suite | Basic unit test assertions |

### 4. Durability, Concurrency & Memory Model

| System Characteristic | **Qenlo** | **pgvector (PostgreSQL)** | **Milvus / Pinecone** | **Chroma / SQLite-VSS** |
| :--- | :--- | :--- | :--- | :--- |
| **ACID & Durability** | Atomic generation snapshot + checksummed WAL + publication watermark | Full enterprise ACID MVCC transactions | Eventual / Tunable consistency (Bounded, Strong, Session) | Atomic SQLite WAL or single-thread file write |
| **Concurrency Model** | Single-handle `Arc<Collection>` read-write lock + OS-level directory exclusivity | Multi-process connection pool with row/table level locks | Stateless distributed query nodes + separated write/log brokers | In-memory mutex / SQLite single-writer lock |
| **Memory Footprint & Cold Start** | **Extremely Low (~10-50MB base)**; zero-copy mmap decoded row slices; 512MB default admission budget | **Moderate to High** (PostgreSQL daemon `shared_buffers` + process memory) | **Very High** (Multiple GBs required for cluster runtime, JVM/Go/C++ services) | **Moderate** (Python runtime RSS + HNSW graph memory) |
| **Disaster Recovery** | Auto-validation on restart; corrupt/partial staging files rejected cleanly | Postgres WAL replay and point-in-time recovery (PITR) | Distributed segment recovery from MinIO / cloud storage | File replacement / SQLite database recovery |
