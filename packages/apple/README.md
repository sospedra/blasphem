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
    from: "0.1.0"
)
```

Add the engine and selected data products to your target dependencies:

```swift
.product(name: "Blasphem", package: "blasphem-swift"),
.product(name: "BlasphemPackEN", package: "blasphem-swift"),
.product(name: "BlasphemPackES", package: "blasphem-swift"),
.product(name: "BlasphemDetectEN", package: "blasphem-swift"),
.product(name: "BlasphemDetectES", package: "blasphem-swift"),
```

## Usage

```swift
import Blasphem

let judge = try Judge(locales: ["en", "es"], grawlix: true)
defer { judge.close() }

let verdict = try judge.judge("you are a stupid loser")
print(verdict)
```

Construction reads files and should run off the main thread.
Reuse the judge for later messages.
The judge supports concurrent callers.
`deinit` also releases the engine.

## Products and configuration

| Product | Contents |
| --- | --- |
| `Blasphem` | Swift API and Rust engine |
| `BlasphemPack<CODE>` | One toxicity pack |
| `BlasphemDetect<CODE>` | One language-detection slice |

Link one pack for each requested model profile.
Detection defaults to `true` and also requires each matching detection product.
Set `detectLanguage: false` to omit detection products.
The judge then returns the highest score across loaded locales.

Use `id` for Indonesian and `ms` for Malay.
Both use `BlasphemPackMS` and `BlasphemDetectMS`.
See [all 16 supported languages](https://github.com/sospedra/blasphem/blob/main/packages/javascript-packs/README.md#locales).

`grawlix` defaults to `false`.
Set `packsDirectory` to a directory URL to read external pack files.
That directory contains `<code>.pack` and any required `<code>.detect` files.
The Swift loader does not read `manifest.json`.

## Results and errors

`Judgement` contains `safe: Bool`, `score: Double`, `locale: String?`, and `grawlix: String?`.
The score is ordinal, between 0 and 1.
It is not a probability.
Unrouted text returns a safe verdict with zero score.

Construction throws `BlasphemError` for invalid options, missing products, or invalid data.
Its `code` and `message` describe the failure.
A closed judge throws the `.closed` code.
See [the API source](https://github.com/sospedra/blasphem/tree/main/packages/apple/Sources/Blasphem).

## Build from source

The source repository is `sospedra/blasphem`.
Run these commands from its root with Xcode installed:

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim aarch64-apple-darwin
node packages/apple/scripts/xcframework.mjs
swift build --package-path packages/apple
```

Add `packages/apple` as a local Swift package dependency.
The development manifest provides the `Blasphem` product.
It does not create per-locale resource products.

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

The release renderer creates the separate resource products.
See [distribution.mjs](https://github.com/sospedra/blasphem/blob/main/packages/apple/scripts/distribution.mjs).
Send code changes to [the source repository](https://github.com/sospedra/blasphem), not the generated distribution.

[Contribute](https://github.com/sospedra/blasphem/blob/main/CONTRIBUTING.md)

## License

Code uses [Apache-2.0](https://github.com/sospedra/blasphem/blob/main/LICENSE).
Language data retains the terms recorded in [NOTICE](https://github.com/sospedra/blasphem/blob/main/NOTICE).
