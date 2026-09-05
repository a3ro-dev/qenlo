# Go SDK

cgo bindings for embedding Qenlo in Go services and command-line tools. The
package links the shared native ABI; it is not CGO-free.

## Installation

```bash
go get github.com/a3ro-dev/qenlo/sdk/go
```

## Quick Example

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

CPU is the default. Desktop artifacts compiled with portable GPU support accept
`NewWithOptions`, `CreateWithOptions`, `OpenWithOptions`, and
`ImportQNWithOptions`. Use `Automatic` to permit a reported fallback or
`GPURequired` to fail if the GPU route is unavailable. The allocation budget is
set with `GPUAllocationBudgetBytes`.
