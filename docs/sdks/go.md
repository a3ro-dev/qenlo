# Go SDK (preview)

cgo bindings for embedding Qenlo in Go services and command-line tools. The package links against the shared native ABI and requires cgo.

## Installation

```bash
go get github.com/a3ro-dev/qenlo/sdk/go
```

## Quick example

```go
package main

import (
    "fmt"
    "log"

    "github.com/a3ro-dev/qenlo/sdk/go"
)

func main() {
    db, err := qenlo.New(3)
    if err != nil {
        log.Fatal(err)
    }
    defer db.Close()

    err = db.Add(qenlo.Record{
        ID:        1,
        UserID:    42,
        Timestamp: 1700000000,
        Vector:    []float32{0.1, 0.8, 0.5},
    })
    if err != nil {
        log.Fatal(err)
    }

    resp, err := db.Search([]float32{0.1, 0.7, 0.5}, qenlo.Filter{UserID: qenlo.Uint64(42)}, 5)
    if err != nil {
        log.Fatal(err)
    }

    fmt.Printf("Matches found: %d\n", len(resp.Results))
}
```

Collections run on CPU by default. Desktop binaries built with portable GPU support expose `NewWithOptions`, `CreateWithOptions`, `OpenWithOptions`, and `ImportQNWithOptions`. Set `Automatic` to allow a fallback to CPU, or `GPURequired` to fail when GPU execution is unavailable. Configure memory limits with `GPUAllocationBudgetBytes`.
