# Qenlo Go SDK

Type-safe cgo bindings for **Qenlo** — the embedded, durable vector database written in Rust.

Qenlo provides exact filtered cosine vector search with atomic commits, write-ahead logging (WAL), portable `.qn` snapshot files, and zero background services.

Collection construction defaults to exhaustive CPU search. Desktop native artifacts built with portable GPU support can request automatic routing or require the GPU explicitly:

```go
db, err := qenlo.NewWithOptions(384, qenlo.Options{
	Backend:                  qenlo.Automatic,
	GPUFilter:                qenlo.GPUFilterPredicate,
	GPUAllocationBudgetBytes: 512 * 1024 * 1024,
})
```

`Automatic` reports the route and fallback in each execution report. `GPURequired` fails during collection construction if the native artifact or host cannot provide the backend. Mobile and CPU-only packages do not acquire desktop GPU dependencies.

## Installation

```bash
go get github.com/a3ro-dev/qenlo/sdk/go
```

The Go binding uses cgo to communicate directly with Qenlo's native C ABI. Release bundles include native binaries for `linux-amd64`, `darwin-arm64`, and `windows-amd64`.

---

## Quickstart

### In-Memory Collection

```go
package main

import (
	"fmt"
	"log"

	"github.com/a3ro-dev/qenlo/sdk/go"
)

func main() {
	// Create an in-memory collection with 3-dimensional vectors
	db, err := qenlo.New(3)
	if err != nil {
		log.Fatalf("failed to initialize Qenlo: %v", err)
	}
	defer db.Close()

	// Insert records
	err = db.Add(qenlo.Record{
		ID:        1,
		UserID:    42,
		Timestamp: 100,
		Vector:    []float32{1.0, 0.0, 0.0},
	})
	if err != nil {
		log.Fatalf("failed to add record: %v", err)
	}

	// Search with optional metadata filters
	filter := qenlo.Filter{
		UserID:         qenlo.Uint64(42),
		TimestampLower: qenlo.Int64(50),
		TimestampUpper: qenlo.Int64(150),
	}

	response, err := db.Search([]float32{1.0, 0.0, 0.0}, filter, 10)
	if err != nil {
		log.Fatalf("search failed: %v", err)
	}

	for _, hit := range response.Results {
		fmt.Printf("Hit ID: %d, Distance: %.4f\n", hit.ID, hit.Distance)
	}

	fmt.Printf("Backend: %s, Duration: %dns\n",
		response.Report.ActualBackend,
		response.Report.TotalDurationNS,
	)
}
```

---

## Durable Storage Across Restarts

```go
// 1. Create a new persistent collection on disk
db, err := qenlo.Create("./my_vectors.qenlo", 128)
if err != nil {
	log.Fatal(err)
}
db.Add(myRecord)
db.Flush()
db.Close()

// 2. Reopen existing collection after restart
db, err = qenlo.Open("./my_vectors.qenlo", 128)
if err != nil {
	log.Fatal(err)
}
defer db.Close()
```

---

## Portable `.qn` Snapshots

```go
// Export to a standalone immutable .qn file
err := db.ExportQN("backup.qn")

// Import from .qn into an in-memory collection
snapshotDb, err := qenlo.ImportQN("backup.qn", 128)
if err != nil {
	log.Fatal(err)
}
defer snapshotDb.Close()
```

---

## Batch Operations

```go
records := []qenlo.Record{
	{ID: 10, UserID: 1, Timestamp: 1000, Vector: []float32{0.1, 0.2, 0.3}},
	{ID: 11, UserID: 1, Timestamp: 1001, Vector: []float32{0.4, 0.5, 0.6}},
}

// Atomic batch insert
if err := db.AddBatch(records); err != nil {
	log.Fatalf("batch insert failed: %v", err)
}

// Batch delete by ID
if err := db.DeleteBatch([]uint64{10, 11}); err != nil {
	log.Fatalf("batch delete failed: %v", err)
}
```

---

## Error Handling & Concurrency

Qenlo errors can be unpacked with `errors.As`:

```go
var qerr *qenlo.Error
if errors.As(err, &qerr) {
	fmt.Println("Qenlo error:", qerr.Message)
}
```

`qenlo.Collection` manages its own internal read-write locking and is safe for concurrent search calls across multiple goroutines.

---

## License

Dual-licensed under **MIT** or **Apache-2.0** at your option.
