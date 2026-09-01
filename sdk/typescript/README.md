# `@qenlo/qenlo`

Type-safe Node.js bindings for Qenlo's embedded, durable vector database.

```ts
import { Collection } from "@qenlo/qenlo";

using db = Collection.memory(3);
db.add({ id: 1n, userId: 7n, timestamp: 10n, vector: [1, 0, 0] });
const response = db.search([1, 0, 0], { userId: 7n });
console.log(response.results[0]?.id); // 1n
```

IDs and timestamps use `bigint`, preventing JavaScript's 53-bit integer limit
from corrupting Qenlo's 64-bit values. The npm release carries a native library
for each supported OS/architecture. A source checkout can set
`QENLO_LIBRARY_PATH` to a local build.
