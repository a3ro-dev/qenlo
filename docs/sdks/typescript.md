# TypeScript / Node.js SDK

Native Node.js FFI bindings for Qenlo with TypeScript type definitions and explicit resource management (`using`).

## Installation

```bash
pnpm add @a3ro.dev/qenlo
# or npm install @a3ro.dev/qenlo
```

## Quick example

```typescript
import { Collection } from "@a3ro.dev/qenlo";

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

Collections default to exhaustive CPU search. Desktop binaries compiled with portable GPU support also accept `{ backend: "automatic" }` or `{ backend: "gpu-required" }`, alongside `gpuFilterMode` and `gpuAllocationBudgetBytes`. Automatic mode returns the chosen execution route and any fallback in the execution report. Required mode throws an error instead of silently falling back to CPU.
