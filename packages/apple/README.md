# Blasphem for Swift

Local toxicity checks for iOS and macOS applications.
Swift calls the Rust engine through a bundled XCFramework.

Blasphem hashes word and character n-grams into sparse feature vectors.
A linear classifier trained offline scores them with 16-bit weights.
Lexicons and context rules contribute to the verdict.
Detection runs locally without neural networks or cloud inference.

## Requirements

| Component | Minimum or supported target |
| --- | --- |
| Swift tools | 5.9 |
| iOS | 15.1, arm64 devices and simulators |
| macOS | 12, Apple silicon |

The current XCFramework has no Intel or Mac Catalyst slices.
See [the slice list](https://github.com/sospedra/blasphem/blob/main/packages/apple/scripts/xcframework.mjs).

## Installation

The public Swift package release is pending.
Use [the local source build](#build-from-source) for the current checkout.

The release distribution is configured for `sospedra/blasphem-swift`.
Its Swift Package Manager dependency is:

```swift
.package(
    url: "https://github.com/sospedra/blasphem-swift.git",
    from: "2.0.0"
)
```

Add the library and build plugin to your application target:

```swift
.executableTarget(
    name: "App",
    dependencies: [.product(name: "Blasphem", package: "blasphem-swift")],
    plugins: [.plugin(name: "BlasphemAssets", package: "blasphem-swift")]
)
```

In Xcode, add the `Blasphem` library to the application.
Add `BlasphemAssets` under Build Phases > Run Build Tool Plug-ins.
Place `blasphem.json` beside the consuming `Package.swift` or `.xcodeproj`:

```json
{"locales":["en","es"],"assets":"bundled","detectLanguage":true}
```

`locales` accepts a nonempty array or `"all"`.
The plugin resolves `id` to `ms`, removes duplicates, and uses registry order.
`assets` defaults to `"bundled"`. Set `"remote"` for persistent downloads.
`"jsdelivr"` remains a compatibility alias for `"remote"`.
`detectLanguage` defaults to `true`; `false` omits detection slices.
The plugin reads the installed release resources. Builds make no CDN requests.

## Usage

```swift
import Blasphem

let judge = try await Judge.create(grawlix: true)
defer { judge.close() }
print(try judge.judge("you are a stupid loser"))
```

The plugin generates this factory inside your consuming target.
SwiftPM uses that target's resource bundle. Xcode uses its application bundle.
Construction finishes before the factory returns. Reuse the judge for later messages.

Bundled applications contain only selected packs, detection slices, configuration, and notices.
Remote applications contain configuration and the native engine.
The first remote call downloads the pinned manifest, then the selected files.
SHA-256 and byte counts verify all downloaded files.
The cache uses Application Support and exact release versions.
Each file has two attempts and a 30-second total deadline.
Concurrent calls share downloads. Complete cached data supports offline restarts.
Every cache read checks integrity. Corrupt data requires a new download.
Cancellation leaves valid cached data intact.

## Advanced directories

```swift
let judge = try Judge(
    locales: ["en", "es"],
    detectLanguage: false,
    packsDirectory: URL(fileURLWithPath: "/absolute/path/to/packs")
)
```

An explicit directory contains `<code>.pack` and required `<code>.detect` files.
This advanced initializer does not use generated configuration or remote storage.
`grawlix` defaults to `false`.
Disabling detection returns the highest score across loaded locales.

## Results and errors

`Judgement` contains `safe: Bool`, `score: Double`, `locale: String?`, and `grawlix: String?`.
`grawlix` contains masked text for unsafe verdicts when requested, otherwise `nil`.
The score is ordinal, between 0 and 1.
It is not a probability.
Unrouted text returns a safe verdict with zero score.

Construction throws for invalid options, missing files, or invalid data.
Its `code` and `message` describe the failure.
A closed judge throws the `.closed` code.
See [the API source](https://github.com/sospedra/blasphem/tree/main/packages/apple/Sources/Blasphem).

## Build from source

The source repository is `sospedra/blasphem`.
Run these commands from its root with Xcode installed:

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim aarch64-apple-darwin
node packages/apple/scripts/xcframework.mjs
node packages/apple/scripts/prepare.mjs
swift build --package-path packages/apple
```

Add `packages/apple` as a local Swift package dependency.
The development manifest provides the library and build plugin.
The prepare command installs canonical resources for the plugin generator.

For local development, supply the committed canonical pack directory:

```swift
import Blasphem
import Foundation

let judge = try Judge(
    locales: ["en", "es"],
    packsDirectory: URL(
        fileURLWithPath: "/path/to/blasphem/resources/packs",
        isDirectory: true
    )
)
defer { judge.close() }

print(try judge.judge("you are a stupid loser"))
```

The release renderer packages the generator with exact release resources.
See [distribution.mjs](https://github.com/sospedra/blasphem/blob/main/packages/apple/scripts/distribution.mjs).
Send code changes to [the source repository](https://github.com/sospedra/blasphem), not the generated distribution.

[Contribute](https://github.com/sospedra/blasphem/blob/main/CONTRIBUTING.md)

## License

Code uses [Apache-2.0](https://github.com/sospedra/blasphem/blob/main/LICENSE).
Language data retains the terms recorded in [NOTICE](https://github.com/sospedra/blasphem/blob/main/NOTICE).
