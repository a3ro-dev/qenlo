# Swift SDK (preview)

Swift bindings over the shared C ABI for macOS 13+ and iOS 16+.

The source package lives in `sdk/apple`. The repository root is not currently a
SwiftPM package, so alpha.5 does not claim remote `Package.swift` installation.
Build the wrapper from source or use the `QenloFFI.xcframework.zip` attached to
the matching GitHub release.

## Quick example

```swift
import Qenlo

let db = try QenloCollection(memoryDimension: 3)
try db.add(QenloRecord(id: 1, userID: 42, timestamp: 1700000000, vector: [0.1, 0.8, 0.5]))

let response = try db.search([0.1, 0.7, 0.5], filter: QenloFilter(userID: 42), k: 5)

print("Found \(response.results.count) results")
```

Collections default to CPU. On macOS, binaries compiled with portable GPU support accept a `QenloCollectionConfiguration` with `.automatic` or `.gpuRequired`. Automatic mode reports the active execution route and any fallback. The iOS artifact runs strictly on CPU unless built with custom GPU flags.
