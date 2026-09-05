# TypeScript / Node.js SDK

High-performance native Node.js FFI bindings for Qenlo with TypeScript type safety and explicit resource management (`using`).

## Installation

```bash
pnpm add @a3ro.dev/qenlo
# or npm install @a3ro.dev/qenlo
```

## Quick Example

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

Construction defaults to exhaustive CPU search. Desktop native artifacts built
with portable GPU support accept `{ backend: "automatic" }` or
`{ backend: "gpu-required" }`, plus `gpuFilterMode` and
`gpuAllocationBudgetBytes`. Automatic mode exposes the actual route and fallback
in the returned execution report; required mode fails instead of silently using
CPU.
