# Swift SDK

Swift bindings over the shared C ABI for the platforms declared by the package:
macOS 13+ and iOS 16+.

## Swift Package Manager

Add to your `Package.swift`:

```swift
dependencies: [
    .package(url: "https://github.com/a3ro-dev/qenlo.git", from: "0.1.0-alpha.2")
]
```

## Quick Example

```swift
import Qenlo

let db = try QenloCollection(memoryDimension: 3)
try db.add(QenloRecord(id: 1, userID: 42, timestamp: 1700000000, vector: [0.1, 0.8, 0.5]))

let response = try db.search([0.1, 0.7, 0.5], filter: QenloFilter(userID: 42), k: 5)

print("Found \(response.results.count) results")
```

Collections default to CPU. A macOS native artifact compiled with portable GPU
support can receive a `QenloCollectionConfiguration` using `.automatic` or
`.gpuRequired`. Automatic mode reports its actual route and fallback. The iOS
artifact remains CPU-only unless explicitly built otherwise.
