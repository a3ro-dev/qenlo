# Go Driver

CGO-free and FFI-backed Go package for embedding Qenlo inside Go microservices and CLI tools.

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

    resp, err := db.Search([]float32{0.1, 0.7, 0.5}, qenlo.Filter{UserID: 42}, 5)
    if err != nil {
        log.Fatal(err)
    }

    fmt.Printf("Matches found: %d\n", len(resp.Results))
}
```
