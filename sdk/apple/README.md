# Qenlo for macOS and iOS

The Swift package provides typed, documented bindings over Qenlo's shared Rust
engine. Release artifacts include a signed-ready XCFramework for macOS arm64 and
x86_64, iOS arm64, and the iOS Simulator.

```swift
let db = try QenloCollection(memoryDimension: 3)
defer { try? db.close() }
try db.add(.init(id: 1, userID: 7, timestamp: 10, vector: [1, 0, 0]))
let response = try db.search([1, 0, 0], filter: .init(userID: 7))
```

Source builds set `LIBRARY_PATH` to a compiled `libqenlo_ffi.a` or `.dylib`.
The release workflow assembles the XCFramework before publishing the package.
