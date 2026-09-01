// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "Qenlo",
    platforms: [.macOS(.v13), .iOS(.v16)],
    products: [.library(name: "Qenlo", targets: ["Qenlo"])],
    targets: [
        .target(
            name: "CQenlo",
            publicHeadersPath: "include",
            linkerSettings: [.linkedLibrary("qenlo_ffi")]
        ),
        .target(name: "Qenlo", dependencies: ["CQenlo"]),
        .testTarget(name: "QenloTests", dependencies: ["Qenlo"]),
    ]
)
