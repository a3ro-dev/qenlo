# TypeScript / Node.js SDK

High-performance native Node.js FFI bindings for Qenlo with TypeScript type safety and explicit resource management (`using`).

## Installation

```bash
pnpm add @qenlo/qenlo
# or npm install @qenlo/qenlo
```

## Quick Example

```typescript
import { Collection } from "@qenlo/qenlo";

// Open in-memory collection
using db = Collection.memory(3);

// Insert records
db.add({
  id: 1n,
  userId: 42n,
  timestamp: 1700000000n,
  vector: [0.1, 0.8, 0.5],
});

// Search
const response = db.search([0.1, 0.7, 0.5], { userId: 42n }, 5);
console.log(`Matched ID: ${response.results[0]?.id}`);
```
