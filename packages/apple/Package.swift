// swift-tools-version: 5.9
// The development manifest. The published one, with the 30 resource targets
// and the binary target by URL, is rendered by scripts/distribution.mjs.
import PackageDescription

let package = Package(
    name: "Blasphem",
    platforms: [.iOS("15.1"), .macOS(.v12)],
    products: [
        .library(name: "Blasphem", targets: ["Blasphem"]),
    ],
    targets: [
        .binaryTarget(name: "BlasphemFFI", path: "BlasphemFFI.xcframework"),
        .target(name: "Blasphem", dependencies: ["BlasphemFFI"]),
    ]
)
