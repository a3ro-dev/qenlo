# Swift & Apple Platforms

Native Swift package for iOS, macOS, iPadOS, and visionOS applications.

## Swift Package Manager

Add to your `Package.swift`:

```swift
dependencies: [
    .package(url: "https://github.com/a3ro-dev/qenlo.git", from: "0.1.0-alpha.1")
]
```

## Quick Example

```swift
import Qenlo

let db = try QenloCollection(memoryDimension: 3)
try db.add(id: 1, userId: 42, timestamp: 1700000000, vector: [0.1, 0.8, 0.5])

let response = try db.search(
    query: [0.1, 0.7, 0.5],
    filter: QenloFilter(userId: 42),
    topK: 5
)

print("Found \(response.results.count) results")
```
