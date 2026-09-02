# `@a3ro-dev/qenlo`

Type-safe TypeScript and Node.js bindings for **Qenlo** — the embedded, durable vector database written in Rust.

Qenlo delivers exact filtered vector search with native C ABI performance, explicit resource disposal (`using`), 64-bit integer safety via `bigint`, write-ahead logging (WAL), and transparent execution reports.

## Installation

```bash
pnpm add @a3ro-dev/qenlo
# or: npm install @a3ro-dev/qenlo
```

Platform binaries are packaged directly in the module for:
- Linux (`x64`)
- macOS (`Apple Silicon arm64`)
- Windows (`x64`)

---

## Quickstart

### In-Memory Collection with Explicit Resource Management

```typescript
import { Collection } from "@a3ro-dev/qenlo";

// Uses TypeScript 5.2+ / ECMAScript Explicit Resource Management (`using`)
using db = Collection.memory(3);

// Add records using native bigint for IDs and timestamps
db.add({
  id: 1n,
  userId: 42n,
  timestamp: 100n,
  vector: [1.0, 0.0, 0.0],
});

db.add({
  id: 2n,
  userId: 42n,
  timestamp: 200n,
  vector: [0.0, 1.0, 0.0],
});

// Search with combined metadata filters
const response = db.search([1.0, 0.0, 0.0], {
  userId: 42n,
  timestampLower: 50n,
  timestampUpper: 150n,
}, 5);

for (const hit of response.results) {
  console.log(`Matched ID: ${hit.id}, Distance: ${hit.distance.toFixed(4)}`);
}

// Execution report with telemetry
console.log(`Backend: ${response.report.actualBackend}`);
console.log(`Duration: ${response.report.totalDurationNs}ns`);
```

---

## Durable Storage Across Restarts

```typescript
import { Collection } from "@qenlo/qenlo";

const dir = "./my_vectors.qenlo";

// 1. Create a persistent collection
{
  using db = Collection.create(dir, 128);
  db.add({ id: 100n, userId: 7n, timestamp: 10n, vector: myVector });
  db.flush();
}

// 2. Open across restarts
{
  using db = Collection.open(dir, 128);
  const results = db.search(myQuery, { userId: 7n });
  console.log(`Found ${results.results.length} matches`);
}
```

---

## Portable `.qn` Interchange Files

```typescript
// Export current collection to an immutable snapshot
db.exportQn("./backup.qn");

// Import snapshot into a new in-memory collection
using snapshotDb = Collection.importQn("./backup.qn", 128);
const stats = snapshotDb.stats();
console.log(`Loaded ${stats.liveRows} live rows (Gen ${stats.generation})`);
```

---

## Batch Operations

```typescript
// High-throughput atomic insertion
db.addBatch([
  { id: 10n, userId: 1n, timestamp: 1000n, vector: [0.1, 0.2, 0.3] },
  { id: 11n, userId: 1n, timestamp: 1001n, vector: [0.4, 0.5, 0.6] },
  { id: 12n, userId: 2n, timestamp: 1002n, vector: [0.7, 0.8, 0.9] },
]);

// Batch deletion
db.deleteBatch([10n, 11n]);
```

---

## 64-Bit Integer Safety

JavaScript's `Number` type cannot safely represent integers above `2^53 - 1` (9,007,199,254,740,991). Because Qenlo uses native 64-bit unsigned IDs and 64-bit signed timestamps, `@qenlo/qenlo` uses `bigint` everywhere to guarantee zero integer truncation or ID collision.

---

## API & Type Definitions

```typescript
interface RecordInput {
  readonly id: bigint;
  readonly userId: bigint;
  readonly timestamp: bigint;
  readonly vector: readonly number[];
}

interface Filter {
  readonly userId?: bigint;
  readonly timestampLower?: bigint;
  readonly timestampUpper?: bigint;
}

interface SearchResponse {
  readonly results: readonly SearchResult[];
  readonly report: ExecutionReport;
}

interface CollectionStats {
  readonly dimension: number;
  readonly rows: number;
  readonly liveRows: number;
  readonly generation: bigint;
  readonly preparedGeneration: bigint | null;
  readonly durableGeneration: bigint | null;
  readonly recoveredInterruptedWrite: boolean;
  readonly closed: boolean;
}
```

---

## Anonymous Telemetry Notice

Qenlo collects anonymous installation, execution, and hardware environment telemetry (OS platform, CPU architecture, SDK version, and search duration metrics) transmitted securely to `https://api.gobitsnbytes.org/qenlo/telemetry`. This telemetry is strictly anonymous, privacy-preserving, and mandatory across all SDK installations (there is no opt-out) in order to monitor stability, diagnose GPU driver regressions, and optimize embedded vector routing algorithms.

---

## License

Dual-licensed under **MIT** or **Apache-2.0** at your option.

