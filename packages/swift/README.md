# Blasphem (Swift)

Multilingual pre-send toxicity nudge for iOS and macOS apps, over the Rust
core. Same `judge` contract as the JavaScript package. Swift Package Manager
only; the package lives at `github.com/sospedra/blasphem-swift`.

```swift
// Package.swift
.package(url: "https://github.com/sospedra/blasphem-swift.git", from: "0.1.0")

// The target that judges
.product(name: "Blasphem", package: "blasphem-swift"),
.product(name: "BlasphemPackEN", package: "blasphem-swift"),
.product(name: "BlasphemPackES", package: "blasphem-swift"),
.product(name: "BlasphemDetectEN", package: "blasphem-swift"),
.product(name: "BlasphemDetectES", package: "blasphem-swift"),
```

```swift
import Blasphem

let judge = try Judge(locales: ["en", "es"], detectLanguage: true, grawlix: true)
try judge.judge("you are a stupid loser")
// Judgement(safe: false, score: 0.64, locale: "en", grawlix: "you are a @#$%&! loser")
judge.locales   // ["en", "es"]
judge.close()
```

`Judge(locales:detectLanguage:grawlix:packsDirectory:)` reads the packs from
the app bundle and builds the engine. It blocks on file reads, so call it off
the main thread. `judge(_:)` is synchronous and safe from several threads.
After `close()` it throws `.closed`. `deinit` closes.

## Products

| Product | Holds |
| --- | --- |
| `Blasphem` | the wrapper and the engine, `BlasphemFFI.xcframework` |
| `BlasphemPack<CODE>` | `<code>.pack`: the sparse table, the lexicon, and the rule-pack version for one language |
| `BlasphemDetect<CODE>` | `<code>.detect`: that language's slice of the language-identification model |

Codes: `AR DE EN ES FR HI IT JA KO MS PT RU TR VI ZH`. `id` is an alias for
`ms` at the API; the products use `MS`. Link one pack product per locale you
request, and one detect product per locale when `detectLanguage` is true, the
default. Xcode copies every linked resource bundle into the app, so an app with
`en` and `es` and detection carries four bundles and 2.08 MB of data; without
detection two bundles and 0.72 MB. A missing product fails at construction
with `.localeMissing` and names the product to add.

`packsDirectory` reads `<code>.pack` and `<code>.detect` from a folder
instead of the bundle. Tests and command-line hosts use it.

## Errors

Construction throws `BlasphemError`; `code` is one of `.localesEmpty`,
`.localeUnsupported`, `.localeMissing`, `.assetsRequired`, `.fetchFailed`,
`.digestMismatch`, `.formatVersion`, `.packInvalid`, `.closed`. `message`
carries the detail. `.assetsRequired` and `.digestMismatch` exist for parity
with the JavaScript codes and are never raised here.

## Platforms

iOS 15.1 and macOS 12, Apple silicon devices and simulators. Intel simulators
and Mac Catalyst are out, as in `@blasphem/react-native`.

## Build

The sources of truth are `packages/swift` in `sospedra/blasphem`. CI renders
and pushes `sospedra/blasphem-swift`; nobody edits that repository by hand.

```bash
node packages/swift/scripts/xcframework.mjs          # BlasphemFFI.xcframework and its zip
swift build                                          # from packages/swift, against the local XCFramework
node packages/swift/scripts/distribution.mjs \
  --version 0.1.0 --checksum <sha256> --output /tmp/blasphem-swift   # the published tree, rendered locally
```

`xcframework.mjs` needs the Rust targets `aarch64-apple-ios`,
`aarch64-apple-ios-sim`, and `aarch64-apple-darwin`, plus Xcode. The `swift`
job in `.github/workflows/publish.yml` uploads the zip to the GitHub Release,
computes its checksum, and runs `distribution.mjs --repo`.

## License

The wrapper and the engine are Apache-2.0. The pack and detect products carry
data under CC BY-NC-SA 4.0; see NOTICE.
