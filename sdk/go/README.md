# Qenlo Go

Type-safe cgo bindings for Qenlo's embedded, durable vector database.

```go
db, err := qenlo.New(3)
if err != nil { log.Fatal(err) }
defer db.Close()

err = db.Add(qenlo.Record{ID: 1, UserID: 7, Timestamp: 10, Vector: []float32{1, 0, 0}})
response, err := db.Search([]float32{1, 0, 0}, qenlo.Filter{UserID: qenlo.Uint64(7)}, 10)
```

Official release archives include the native library for Linux, macOS, and
Windows. Qenlo uses cgo; cross-compilation therefore needs a target C toolchain.
