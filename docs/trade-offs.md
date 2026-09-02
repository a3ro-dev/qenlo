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

## Quick Comparison Matrix

| Feature / Requirement | Qenlo | pgvector (Postgres) | Milvus / Pinecone | Chroma / SQLite-VSS |
| :--- | :---: | :---: | :---: | :---: |
| **Deployment Mode** | In-Process Embedded | Client-Server (SQL) | Cloud / Distributed | In-Process Embedded |
| **Setup Complexity** | Zero (no server) | High (Postgres DB) | High (Cluster / SaaS) | Low |
| **Single-File Portable** | Yes (`.qn`) | No | No | Yes |
| **Filtered Search Guarantee** | Exact Pre-Filter | Index-dependent | Variable recall | Post-filter / Exact |
| **SIMD AVX-512 / NEON** | Yes | Optional | Yes | Partial |
| **Native Mobile (iOS / Android)**| Yes | No | No | Limited |
| **Billion-Scale Sharding** | No (Single-node) | No (Single-node) | Yes | No |
| **Full Relational SQL Joins** | No | Yes | No | Limited |
