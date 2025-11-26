// swift-tools-version:5.3
import PackageDescription

let package = Package(
    name: "Tauq",
    products: [
        .library(
            name: "Tauq",
            targets: ["Tauq"]),
    ],
    dependencies: [],
    targets: [
        .target(
            name: "CTauq",
            dependencies: [],
            path: "Sources/CTauq"
        ),
        .target(
            name: "Tauq",
            dependencies: ["CTauq"],
            path: "Sources/Tauq",
            linkerSettings: [
                // Assumes libtauq.a/dylib is available in search path
                // Users might need to set -Xlinker -L/path/to/target/release
                .linkedLibrary("tauq")
            ]
        ),
    ]
)
