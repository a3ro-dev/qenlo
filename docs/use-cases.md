# Suggested & Example Use Cases

Qenlo is engineered as an embedded, single-binary, filtered vector search engine. It eliminates the overhead of running standalone vector database clusters (like Milvus, Qdrant cluster, or Pinecone) when you need sub-millisecond, partition-isolated search inside your application process.

---

## 1. Local-First & Desktop AI Applications

### Scenario
You are building an offline-capable AI desktop app (Electron, Tauri, or native macOS/Windows) such as an AI note-taker, local code search assistant, or personal document summarizer.

### Why Qenlo?
- **Zero Server Setup**: Ships inside your application binary via Python ctypes, Node/TS FFI, or Rust. No Docker or PostgreSQL daemon required on the user's laptop.
- **Single-File Container (`.qn`)**: Every collection lives in a single portable file on disk. Users can move, backup, or sync their knowledge base across machines like an SQLite `.db` file.
- **Instant Cold Boot**: Memory-mapped indexes open in `< 5ms` with zero index warm-up latency.

```python
import qenlo

# Opens an embedded collection directly on the user's filesystem
db = qenlo.open("user_notes.qn", dim=384)
db.add(id=101, user_id=1, timestamp=1700000000, vector=embedding)
```

---

## 2. Multi-Tenant SaaS with Strict Per-Tenant Isolation

### Scenario
You run a B2B SaaS platform where thousands of customers upload embeddings. Security and compliance require strict per-tenant data partitioning (`tenant_id = X`), and customers should never have their vectors mixed during index traversal.

### Why Qenlo?
- **Bitmask & User-ID Pre-Filtering**: Qenlo enforces pre-filtering at the core engine level. Filtered candidate sets are evaluated with zero recall penalty and zero risk of cross-tenant leakage.
- **Separate `.qn` Containers or Partition Filters**: Host one `.qn` file per tenant or query partitioned spaces using `Filter(user_id=tenant_id)`.

```typescript
import { Collection } from "@a3ro-dev/qenlo";

const db = Collection.open("saas_vectors.qn", 768);
const results = db.search(queryVector, { userId: tenantId }, 10);
```

---

## 3. Autonomous AI Agent Episodic & Working Memory

### Scenario
An autonomous agent needs long-term semantic memory of past tool calls, chat sessions, and retrieved facts, bounded by time windows or task scopes.

### Why Qenlo?
- **Compound Temporal Filtering**: Filter by timestamp range (`timestamp_min`, `timestamp_max`) and status bitmask (`flags_all_set`) simultaneously in a single SIMD-accelerated scan.
- **Atomic WAL & Crash Resilience**: Uncommitted steps during sudden agent crashes are recovered via the Append-Only Log with CRC32 checksum verification.

```rust
use qenlo_core::{Filter, QenloCollection, Record};

let filter = Filter {
    user_id: Some(agent_id),
    timestamp_min: Some(start_of_task),
    timestamp_max: Some(end_of_task),
    flags_all_set: 0b0001, // e.g. successful tool executions only
    ..Default::default()
};

let matches = db.search(&task_embedding, &filter, 5)?;
```

---

## 4. Mobile & Edge Devices (iOS & Android)

### Scenario
On-device semantic search for mobile photo galleries, offline voice assistants, or edge IoT sensor anomaly detection.

### Why Qenlo?
- **Cross-Platform C-ABI & Native Wrappers**: Official Kotlin (JNI) and Swift (XCFramework) SDKs.
- **Low Memory Footprint**: Minimal heap allocation with configurable vector quantization (f32, fp16, int8) and zero background daemon CPU consumption.

```kotlin
import dev.qenlo.QenloCollection
import dev.qenlo.Filter

QenloCollection.open("/data/user/0/app/files/photos.qn", dim = 512).use { db ->
    val results = db.search(queryVector, Filter(flagsAnySet = 0b0010u), topK = 10)
}
```

---

## 5. Air-Gapped & Secure Environments

### Scenario
Military, healthcare, or financial air-gapped workstations where outbound internet connections and complex microservice architectures are forbidden.

### Why Qenlo?
- **Zero Network Dependencies**: 100% in-memory or single-file storage engine with zero external service requirements.
- **Deterministic & Auditable**: Cryptographic checksums on every record, portable across Linux, macOS, and Windows.
