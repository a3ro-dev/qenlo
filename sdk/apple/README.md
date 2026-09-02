# Qenlo Swift SDK (macOS & iOS)

Type-safe Swift bindings for **Qenlo** — the embedded, durable vector database written in Rust.

The Swift package provides typed, `Sendable`-safe bindings for macOS and iOS applications with zero background servers, local storage persistence, atomic mutations, and fast exact filtered vector search.

## Installation

Add Qenlo to your `Package.swift` dependencies:

```swift
dependencies: [
    .package(url: "https://github.com/a3ro-dev/qenlo.git", from: "0.1.0")
]
```

Or in Xcode: **File → Add Package Dependencies...** and enter `https://github.com/a3ro-dev/qenlo.git`.

### Supported Platforms
- **macOS** 13.0+ (Apple Silicon `arm64`, Intel `x86_64`)
- **iOS** 16.0+ (`arm64`, Simulator `arm64`/`x86_64`)

---

## Quickstart

### In-Memory Collection

```swift
import Qenlo

do {
    // Initialize an in-memory collection with 3-dimensional vectors
    let db = try QenloCollection(memoryDimension: 3)
    defer { try? db.close() }

    // Insert records with typed UInt64 IDs
    try db.add(Record(
        id: 1,
        userID: 42,
        timestamp: 100,
        vector: [1.0, 0.0, 0.0]
    ))

    try db.add(Record(
        id: 2,
        userID: 42,
        timestamp: 200,
        vector: [0.0, 1.0, 0.0]
    ))

    // Search with optional filters
    let response = try db.search(
        query: [1.0, 0.0, 0.0],
        filter: Filter(userID: 42, timestampLower: 50, timestampUpper: 150),
        k: 5
    )

    for hit in response.results {
        print("Hit ID: \(hit.id), Cosine Distance: \(hit.distance)")
    }

    print("Executed on: \(response.report.actualBackend) in \(response.report.totalDurationNs)ns")
} catch {
    print("Qenlo error: \(error)")
}
```

---

## Durable Storage Across Restarts

```swift
import Qenlo

let url = URL(fileURLWithPath: "/path/to/my_vectors.qenlo")

// 1. Create a persistent collection
let db = try QenloCollection(createPath: url.path, dimension: 128)
try db.add(myRecord)
try db.flush()
try db.close()

// 2. Reopen after restart
let reopened = try QenloCollection(openPath: url.path, dimension: 128)
defer { try? reopened.close() }
let results = try reopened.search(query: myQuery, filter: Filter(userID: 7), k: 10)
```

---

## Portable `.qn` Interchange Files

```swift
// Export to a standalone immutable .qn file
try db.exportQN(to: destinationURL)

// Import .qn into an in-memory collection
let snapshotDb = try QenloCollection(importQN: fileURL, dimension: 128)
defer { try? snapshotDb.close() }
print("Loaded \(snapshotDb.stats().liveRows) live rows")
```

---

## Batch Operations

```swift
let records: [Record] = [
    Record(id: 10, userID: 1, timestamp: 1000, vector: [0.1, 0.2, 0.3]),
    Record(id: 11, userID: 1, timestamp: 1001, vector: [0.4, 0.5, 0.6]),
]

// Atomic batch insertion
try db.addBatch(records)

// Batch deletion by ID
try db.deleteBatch([10, 11])
```

---

## License

Dual-licensed under **MIT** or **Apache-2.0** at your option.

