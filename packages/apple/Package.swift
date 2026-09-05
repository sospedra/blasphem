// swift-tools-version: 5.9
// The development manifest. Run scripts/prepare.mjs before building.
// scripts/distribution.mjs renders the release binary URL and installed data.
import PackageDescription

let package = Package(
    name: "Blasphem",
    platforms: [.iOS("15.1"), .macOS(.v12)],
    products: [
        .library(name: "Blasphem", targets: ["Blasphem"]),
        .plugin(name: "BlasphemAssets", targets: ["BlasphemAssets"]),
    ],
    targets: [
        .binaryTarget(name: "BlasphemFFI", path: "BlasphemFFI.xcframework"),
        .target(name: "Blasphem", dependencies: ["BlasphemFFI"]),
        .executableTarget(name: "BlasphemAssetGenerator", sources: ["main.swift", "Locales.generated.swift"], resources: [.copy("Resources")]),
        .plugin(name: "BlasphemAssets", capability: .buildTool(), dependencies: ["BlasphemAssetGenerator"]),
    ]
)
