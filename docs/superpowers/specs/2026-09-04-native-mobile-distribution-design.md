# Native mobile distribution design

## Status

Rubén approved the design in chat on 2026-09-04 and said "go". Implemented 2026-09-04 except the two test files, which stay pending his approval; see "Implementation notes". Plan: `docs/superpowers/plans/2026-09-04-native-mobile-distribution.md`. This document adds two native consumers to the binding set from `2026-09-03-blasphem-js-contract-and-packages-design.md`: a Swift package and an Android library. The React Native package under `packages/react-native` is unchanged.

One point differs from the chat. The chat proposed typed Swift handles per pack (`PackES.resource`). This document keeps `locales: ["es", "en"]` on both platforms and finds the bytes by name, so every binding shares one signature and a missing artifact fails at construction with the artifact to add.

## Goals

- `Judge(locales:detectLanguage:grawlix:)` in Swift for iOS and macOS apps, through Swift Package Manager.
- `Judge.create(context, options)` in Kotlin for Android apps, through Maven Central.
- The four verdict fields of the JavaScript contract: `safe`, `score`, `locale`, `grawlix`. The same error codes.
- The consumer picks the locales and whether detection ships. The app carries only those bytes. An app with `en` and `es` and detection carries 2.09 MB of data; without detection 0.72 MB; all fifteen with detection 9.19 MB.
- The Rust engine is the one in `crates/blasphem-ffi` for Swift, and a sibling JNI crate over the same `blasphem::Engine` for Android. No hand-written C++.

## Non-goals

- UniFFI. `crates/blasphem-ffi/include/blasphem.h` has 14 functions. A generator costs more than two thin wrappers, and its Kotlin output needs JNA.
- Java, or any JVM target outside Android. Rubén dropped Java on 2026-09-04.
- CocoaPods. SwiftPM is the channel.
- A consumer-facing `Package.swift` at the root of this repository. SwiftPM reads the manifest at the repository root and needs the zip checksum at the version tag; the zip exists only after the tag. A clone of this repository is 118.00 MB. A distribution repository solves both.
- Downloading packs at run time on native. Bytes ship inside the app.
- Digest verification on native. `manifest.json` guards CDN downloads. Here the package manager aligns the versions and the engine checks the pack magic and `PACK_FORMAT_VERSION` (`src/pack.rs:16`). The digest arguments of `blasphem_builder_add` pass NULL.
- Two engine builds, with and without detection code. See Open items.
- Changing `packages/react-native`.

## Data

The engine binary carries no data: `crates/blasphem-ffi/Cargo.toml` depends on `blasphem` with `default-features = false, features = ["language-detection"]`, so `embedded` is off. Every byte of language data comes from `packages/packs/dist`, built by the `packs` job in `publish.yml`.

| Kind | Files | Bytes | Largest | Smallest |
| --- | --- | --- | --- | --- |
| `<code>.pack` | 15 | 4.32 MB | `ja.pack` 0.43 MB | `vi.pack` 0.20 MB |
| `<code>.detect` | 15 | 4.87 MB | `es.detect` 1.04 MB | `ko.detect` 0.00 MB (4 KB) |

Detection is 53 percent of the data, so it ships apart from the packs. Codes and registry order come from `Language::ALL`: `en zh es ar ms pt fr hi ru ja de tr vi ko it`. `id` is an alias for `ms` at the API; the files use `ms`.

One artifact per locale per kind, 30 in total, on each platform. CI derives the list from the file names in `packages/packs/dist`.

## Contract

Both wrappers shall implement `contract.ts` from `packages/react-native/src/core`:

- `locales` is required and non-empty. The wrapper lowercases, resolves `id` to `ms`, rejects unknown codes with `BLASPHEM_LOCALE_UNSUPPORTED`, and drops repeats. The engine reports `locales` in registry order.
- `detectLanguage` defaults to true. When true, every locale needs its `.detect`; a missing slice is `BLASPHEM_LOCALE_MISSING` naming the artifact to add. When false, `.detect` files are never read.
- `grawlix` defaults to false.
- `judge(text)` is synchronous, safe from several threads, and never fails while the judge is open. After `close()` it fails with `BLASPHEM_CLOSED`.
- `Judgement` has `safe: Bool`, `score: Double`, `locale: String?`, `grawlix: String?`.
- Errors carry one of the nine codes in `errors.ts`. The engine reports `CODE: detail`; the wrapper parses the head as `fromEngineError` does and maps anything else to `BLASPHEM_PACK_INVALID`.
- Construction reads files, so it blocks. The README shall say to call it off the main thread.

Both wrappers shall accept a directory override, `packsDirectory`, that reads `<code>.pack` and `<code>.detect` from a folder instead of the app bundle. Tests and command-line hosts use it. Go has the same in `Options.Assets`.

## Swift

### Repository

`sospedra/blasphem-swift` holds the published package. CI writes it; nobody edits it by hand. Its `main` branch and its tags `v<version>` come from the `swift` job in `publish.yml`. The sources of truth stay in this repository under `packages/swift`:

| Path | Holds |
| --- | --- |
| `Sources/Blasphem/*.swift` | the wrapper: `Judge`, `Judgement`, `BlasphemError`, the bundle lookup, `Locales.generated.swift` |
| `Package.swift` | the development manifest: `binaryTarget(path: "BlasphemFFI.xcframework")`, one `Blasphem` target, the test target |
| `scripts/xcframework.mjs` | builds the three slices and the XCFramework with a module map |
| `scripts/distribution.mjs` | renders the published `Package.swift`, copies the sources and the 30 resource targets, pushes and tags |

The published manifest declares `swift-tools-version: 5.9`, `name: "Blasphem"`, platforms `.iOS("15.1")` and `.macOS(.v12)`, and 32 targets:

1. `BlasphemFFI`, a `binaryTarget` with `url: "https://github.com/sospedra/blasphem/releases/download/v<version>/BlasphemFFI.xcframework.zip"` and the checksum `swift package compute-checksum` printed for that zip.
2. `Blasphem`, the wrapper, depending on `BlasphemFFI`. Product `Blasphem`.
3. `BlasphemPack<CODE>` for each code, holding `Resources/<code>.pack` under `.copy` and one file `BlasphemPack<CODE>.swift` with an empty public enum, because a target needs a source. Product of the same name.
4. `BlasphemDetect<CODE>` for each code, the same shape over `<code>.detect`.

`CODE` is the code in upper case: `BlasphemPackES`, `BlasphemDetectMS`.

### Engine

`scripts/xcframework.mjs` shall build `blasphem-ffi` as a `staticlib` for `aarch64-apple-ios`, `aarch64-apple-ios-sim`, and `aarch64-apple-darwin`, with `CARGO_TARGET_DIR=target/ffi` as `packages/react-native/scripts/rust.mjs` does. It writes a headers directory with `blasphem.h` and this `module.modulemap`:

```
module BlasphemFFI {
  header "blasphem.h"
  export *
}
```

`xcodebuild -create-xcframework` combines the three archives and the headers into `BlasphemFFI.xcframework`. `ditto -c -k --keepParent` zips it. The Swift wrapper does `import BlasphemFFI`.

Intel simulators (`x86_64-apple-ios`) and Mac Catalyst are out, as in the React Native package.

### Finding the bytes

For a code `es` the wrapper shall look for the bundle `Blasphem_BlasphemPackES.bundle`, the name SwiftPM gives a resource bundle (`<package>_<target>`), in this order: `Bundle.main.url(forResource:withExtension:)`, then `Bundle(for: Judge.self).resourceURL`, then `Bundle.main.bundleURL`. That is the search the generated `Bundle.module` accessor performs, so it holds for apps, app extensions, tests, and executables. Inside the bundle it reads `es.pack`. `BlasphemDetectES` follows the same path for `es.detect`.

Xcode copies every resource bundle in the dependency graph. No symbol from the pack module needs a reference, so the linker cannot drop the data.

A missing bundle shall raise `BlasphemError(code: .localeMissing, message: "add the product BlasphemPackES to the target")`. `packsDirectory` skips the bundle search.

### API

```swift
import Blasphem

let judge = try Judge(locales: ["en", "es"], detectLanguage: true, grawlix: true)
judge.judge("you are a stupid loser")
// Judgement(safe: false, score: 0.64, locale: "en", grawlix: "you are a @#$%&! loser")
judge.locales   // ["en", "es"]
judge.close()
```

`Judge` is a `final class`, `@unchecked Sendable`. It owns one `OpaquePointer` from `blasphem_builder_build`. `judge(_:)` takes a read lock and calls `blasphem_engine_judge`; `close()` takes the write lock, calls `blasphem_engine_free`, and nils the pointer. `deinit` closes. `BlasphemError` is a `struct: Error` with `code: Code` (an enum of the nine codes) and `message: String`.

## Android

### Layout

`packages/android` is a Gradle project with Kotlin DSL:

| Path | Holds |
| --- | --- |
| `engine/` | the `blasphem` AAR: Kotlin sources, `src/main/jniLibs/<abi>/libblasphem_jni.so`, `consumer-rules.pro` |
| `packs/<code>/pack/src/main/assets/blasphem/<code>.pack` | one asset module per pack, synced from `packages/packs/dist` |
| `packs/<code>/detect/src/main/assets/blasphem/<code>.detect` | one asset module per detect slice |
| `bom/` | `blasphem-bom`, a Maven BOM pinning all 31 artifacts |
| `settings.gradle.kts` | includes `engine`, `bom`, and one module per file present under `packs/` |
| `build.gradle.kts` | configures every subproject: Android library plugin, `minSdk 24`, `compileSdk 35`, `namespace "me.sospedra.blasphem"`, publishing, signing |
| `gradle.properties` | `VERSION_NAME=<version>`, mirrored by `blasphem-train sync-versions` |
| `scripts/sync-packs.mjs` | copies the 30 files into the asset modules; the copies are gitignored |

Group id `me.sospedra.blasphem`, verified through a DNS TXT record on `sospedra.me`, the domain that serves `blasphem.sospedra.me`. Artifact ids: `blasphem`, `blasphem-pack-<code>`, `blasphem-detect-<code>`, `blasphem-bom`. Kotlin package `me.sospedra.blasphem`.

```kotlin
dependencies {
  implementation(platform("me.sospedra.blasphem:blasphem-bom:0.1.0"))
  implementation("me.sospedra.blasphem:blasphem")
  implementation("me.sospedra.blasphem:blasphem-pack-en")
  implementation("me.sospedra.blasphem:blasphem-pack-es")
  implementation("me.sospedra.blasphem:blasphem-detect-en")
  implementation("me.sospedra.blasphem:blasphem-detect-es")
}
```

Gradle merges the `assets/blasphem/` folders of every AAR into the APK. Two artifacts never carry the same file name, so the merge cannot conflict. The path matches what `packages/react-native/android/src/main/java/com/margelo/nitro/blasphem/HybridBlasphemAssets.kt` reads, so an app that has both sees one folder.

### Engine

A new crate `crates/blasphem-jni`, `crate-type = ["cdylib"]`, `publish = false`, joins `[workspace] members` but not `default-members`. It depends on `blasphem` with the same features as `blasphem-ffi` and on `jni = "0.21"`. It does not depend on `blasphem-ffi`; both crates are thin layers over `blasphem::Engine` and `blasphem::EngineSource`.

It shall export these functions for `me.sospedra.blasphem.Native`:

| Kotlin declaration | Behavior |
| --- | --- |
| `external fun builderNew(detectLanguage: Boolean, grawlix: Boolean): Long` | boxes a builder, returns its address |
| `external fun builderAdd(builder: Long, locale: String, pack: ByteArray, detect: ByteArray?)` | `EngineSource::new` with NULL digests; throws on failure |
| `external fun builderBuild(builder: Long): Long` | `Engine::build`; consumes the builder on success; throws and keeps the builder on failure |
| `external fun builderFree(builder: Long)` | drops an unconsumed builder |
| `external fun engineLocales(engine: Long): Array<String>` | `Engine::locales` |
| `external fun engineJudge(engine: Long, text: String): Judgement` | constructs `me.sospedra.blasphem.Judgement` through `NewObject` |
| `external fun engineFree(engine: Long)` | drops the engine |

Failures throw `java.lang.RuntimeException` with the engine's `CODE: detail` text. Kotlin converts it to `BlasphemException`.

`cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o packages/android/engine/src/main/jniLibs build --release --locked -p blasphem-jni` produces the three `.so`. `RUSTFLAGS` carries `-C link-arg=-Wl,-z,max-page-size=16384`, because Google Play requires 16 KB page alignment for apps targeting API 35 and later. `llvm-readelf -l` shall show every `LOAD` segment aligned to `0x4000` on the 64-bit ABIs. `consumer-rules.pro` keeps the native method names:

```
-keepclasseswithmembernames class me.sospedra.blasphem.Native { native <methods>; }
```

The ABIs match the React Native package. `x86` is out.

### API

```kotlin
import me.sospedra.blasphem.Judge
import me.sospedra.blasphem.JudgeOptions

val judge = Judge.create(context, JudgeOptions(locales = listOf("en", "es"), detectLanguage = true, grawlix = true))
judge.judge("you are a stupid loser")
// Judgement(safe=false, score=0.64, locale=en, grawlix=you are a @#$%&! loser)
judge.locales   // [en, es]
judge.close()
```

`Judge.create` reads `assets/blasphem/<code>.pack`, and `<code>.detect` when detection is on, through `context.assets`. A missing asset throws `BlasphemException(code = LOCALE_MISSING, message = "add me.sospedra.blasphem:blasphem-pack-es")`. `JudgeOptions.packsDirectory: File?` reads from a folder instead. `Judge` implements `AutoCloseable`; `judge` holds a read lock, `close` the write lock, over a `ReentrantReadWriteLock`. `BlasphemException` carries `code: Code`, an enum of the nine codes.

### Publishing

The `com.vanniktech.maven.publish` plugin publishes every subproject to the Central Portal with automatic release, sources and javadoc jars, and GPG signing. Engine artifacts declare Apache-2.0. Pack and detect artifacts declare `CC-BY-NC-SA-4.0` and include `NOTICE`, as `@blasphem/packs` and `blasphem-packs` do.

## Locale tables

`blasphem-train locales-table` gains `--format swift` and `--format kotlin` next to `ts`, `go`, and `python`, so both wrappers resolve aliases and validate codes from `Language::ALL`. The build scripts write `Locales.generated.swift` and `Locales.generated.kt`; both are gitignored like the TypeScript table.

## Versions

`crates/blasphem-train/src/versions.rs` gains one mirror: `packages/android/gradle.properties` with the pattern `(?m)^VERSION_NAME=(.+)$`. The existing drift test covers it. The Swift repository takes its version from the tag at generation time and needs no mirror.

## Pipeline

`release.yml` is unchanged. `publish.yml` gains two jobs after `plan` and extends `verify`.

**swift**, on `macos-15`, `permissions: contents: write`:

1. Add the three Apple targets. Cache `target/ffi`.
2. `node packages/swift/scripts/xcframework.mjs` builds and zips the XCFramework.
3. `gh release upload v<version> BlasphemFFI.xcframework.zip --clobber` puts the zip on the Release cargo-dist created.
4. `swift package compute-checksum BlasphemFFI.xcframework.zip`.
5. Download the `packs-dist` artifact.
6. `node packages/swift/scripts/distribution.mjs --checksum <sum> --version <version>` clones `sospedra/blasphem-swift` over SSH with the deploy key from the secret `BLASPHEM_SWIFT_DEPLOY_KEY`, replaces the tree with the rendered manifest, `Sources/`, the 30 resource targets, `LICENSE`, `NOTICE`, and `README.md`, commits `Publish <version>`, pushes `main`, tags `v<version>`, pushes the tag. An existing tag makes the step print `status=exists` and exit 0.

The distribution repository grows only when a pack or a slice changes bytes, because git stores one blob per content. Code releases that leave the data alone add a manifest and a commit.

**android**, on `ubuntu-24.04`:

1. Install `cargo-ndk`, add the three Android targets. The runner has the NDK.
2. Build the three `.so` with the page-size flag.
3. Download `packs-dist`, `node packages/android/scripts/sync-packs.mjs`.
4. Run the instrumented smoke test on an API 35 `x86_64` emulator with `reactivecircus/android-emulator-runner`. A failure stops the job before anything publishes.
5. `./gradlew publishAllPublicationsToMavenCentralRepository` with the secrets `MAVEN_CENTRAL_USERNAME`, `MAVEN_CENTRAL_PASSWORD`, `SIGNING_KEY`, `SIGNING_PASSWORD`.

**verify** adds `swift` and `android` to `needs` and gains two steps. On `macos-15`: a scratch executable package depends on `sospedra/blasphem-swift` at `v<version>` with the products `Blasphem`, `BlasphemPackEN`, `BlasphemPackES`, `BlasphemDetectEN`, `BlasphemDetectES`, builds, and judges the README example; the step fails unless `safe` is false and `locale` is `en`. On `ubuntu-24.04`: poll `https://repo1.maven.org/maven2/me/sospedra/blasphem/blasphem/<version>/` for up to 30 minutes, then resolve the BOM plus `blasphem-pack-es` in a scratch Gradle project and list the AAR: `jni/arm64-v8a/libblasphem_jni.so`, `jni/armeabi-v7a/libblasphem_jni.so`, `jni/x86_64/libblasphem_jni.so`, and `assets/blasphem/es.pack` must be present.

The header comment of `publish.yml` shall change: npm, PyPI, and Go authenticate over OIDC; Maven Central and the Swift repository hold five repository secrets.

## Errors

| Case | Swift | Kotlin |
| --- | --- | --- |
| Empty `locales` | `.localesEmpty` | `LOCALES_EMPTY` |
| Unknown code | `.localeUnsupported` | `LOCALE_UNSUPPORTED` |
| Pack or slice not found | `.localeMissing`, names the product | `LOCALE_MISSING`, names the artifact |
| File exists and cannot be read | `.fetchFailed` | `FETCH_FAILED` |
| Wrong pack magic or version | `.packInvalid`, `.formatVersion` | `PACK_INVALID`, `FORMAT_VERSION` |
| `judge` after `close` | `.closed` | `CLOSED` |

`assetsRequired` and `digestMismatch` exist in both enums for parity with the JavaScript codes and are never raised on native.

## Tests

Three new test files, all opt-in and pending Rubén's approval:

- `packages/swift/Tests/BlasphemTests/JudgeTests.swift`, run by `swift test` against the local XCFramework with `packsDirectory` pointing at `packages/packs/dist`. Cases: the README verdict, registry order of `locales`, `detectLanguage: false` never reads a slice, an unknown code, a missing pack directory names the product, `judge` after `close`.
- `packages/android/engine/src/androidTest/kotlin/me/sospedra/blasphem/JudgeTest.kt`, the instrumented smoke on the emulator with `en` and `es` assets. Cases: the README verdict, a missing detect slice names `blasphem-detect-es`, `judge` after `close`.
- `crates/blasphem-train/src/locales_table.rs` gains cases for the Swift and Kotlin tables next to the Go and Python ones. This is an existing test module, not a new file.

The JavaScript bindings share their expectations in `packages/blasphem/tests/cases.mjs`. The plan ports `README_EXAMPLE` and `SUPPLIED_CASES` from that file into both suites as fixture tables.

## Acceptance criteria

- `cargo clippy --workspace --all-targets --locked -- -D warnings` is clean with `blasphem-jni` in the workspace.
- `cargo test -p blasphem-train` passes, with the Gradle mirror and the two table formats.
- `dist generate --check` is clean.
- `node packages/swift/scripts/xcframework.mjs` produces an XCFramework with `ios-arm64`, `ios-arm64-simulator`, and `macos-arm64`, each with `Headers/module.modulemap`.
- `swift test` in `packages/swift` passes on macOS.
- The instrumented test passes on an API 35 emulator.
- `llvm-readelf -l` shows `LOAD` alignment `0x4000` for `arm64-v8a` and `x86_64`.
- A scratch iOS app with `en` and `es` and detection carries four bundles and 2.09 MB of data; without detection two bundles and 0.72 MB.
- Both `verify` steps pass on the first published version.

## Open items

1. **Engine size.** The static archives measure 8.24 MB (`ios-arm64`) and 11.95 MB (`arm64-v8a`) before the linker strips dead code. The size the app carries, and the share owned by `language-detection`, are unmeasured. The plan measures both. If the detection code exceeds 1.00 MB, a second engine build without it becomes a follow-up decision.
2. **Manual steps for Rubén.** Create `sospedra/blasphem-swift` with a write deploy key. Verify `me.sospedra` on the Central Portal through DNS. Generate a GPG key. Store the five secrets.
3. **Maven Central propagation.** Central publishes within minutes but promises no bound. The `verify` poll stops at 30 minutes and fails the run; a rerun of `verify` alone is the remedy.
4. **First Swift release order.** The `swift` job needs the Release to exist for step 3. `publish.yml` already starts from the Release run, so the order holds without change.

## Implementation notes, 2026-09-04

**Deviations.** The generated `Locales.generated.swift` and `Locales.generated.kt` are committed, not gitignored: the TypeScript, Go, and Python tables are committed too, and `the_committed_tables_are_current` in `locales_table.rs` now covers all five. Swift `judge(_:)` throws, because `.closed` after `close()` needs a throwing signature; the README example reads `try judge.judge(...)`. Pack and detect modules carry unique namespaces `me.sospedra.blasphem.pack.<code>` and `me.sospedra.blasphem.detect.<code>`, because AGP 8 fails a build whose libraries share a namespace. The `swift` and `android` jobs also `need` `packs`, since both download `packs-dist`. Pack AARs keep `META-INF/NOTICE` inside `classes.jar` by removing AGP's default exclusion. `consumer-rules.pro` also keeps the `Judgement` constructor, which the engine resolves by name through JNI. The development `Package.swift` declares no test target: SwiftPM rejects a target whose directory does not exist.

**Pins.** AGP 8.13.1, Gradle 8.14.5 (wrapper committed), Kotlin 2.2.21, `com.vanniktech.maven.publish` 0.37.0, `jni` 0.21.1, `cargo-ndk` 4.1.2 with `--platform 24`. A local `publishToMavenLocal` passes `-PRELEASE_SIGNING_ENABLED=false`.

**Sizes.**

| Artifact | MB |
| --- | --- |
| `libblasphem_ffi.a` ios-arm64 / ios-arm64-simulator / macos-arm64 | 8.24 / 8.22 / 8.19 |
| `BlasphemFFI.xcframework.zip` | 6.30 |
| `libblasphem_jni.so` arm64-v8a / armeabi-v7a / x86_64 | 1.89 / 1.33 / 2.12 |
| macOS arm64 release executable, dead-stripped, wrapper plus engine | 2.19 |
| the same without `language-detection` | 2.15 |
| a Foundation hello-world executable | 0.06 |

The app carries about 2.13 MB of engine on Apple silicon; the detection code is 0.04 MB of it, so open item 1 closes without a second engine build. Data for `en` and `es` with detection: four bundles, 2.08 MB (the goal said 2.09); without detection two bundles, 0.72 MB.

**Verified.** `cargo test -p blasphem-train --lib locales_table` 4 passed, `versions` 2 passed. `cargo fmt --all --check` clean. `cargo clippy --workspace --exclude blasphem --all-targets --locked -- -D warnings` and `cargo clippy -p blasphem --lib --bins` clean. `dist generate --check` clean. `node packages/swift/scripts/xcframework.mjs` writes three slices, each with `Headers/module.modulemap`; `swift build` in `packages/swift` passes. A scratch macOS executable against the rendered distribution (binary target by path) passes 12 checks: the README verdict from the bundles, registry order, the missing product `BlasphemPackFR` named, grawlix only when requested, `packsDirectory` equals the bundles, `id` to `ms`, empty and unknown locales, `detectLanguage: false` never reads a slice, a missing directory, `.closed` after `close()`, the 30 supplied cases, four bundles. A scratch Java 17 host loads the macOS `libblasphem_jni.dylib` and gets the README verdict and the `CODE: detail` errors. `llvm-readelf -l` shows every `LOAD` at `0x4000` on all three ABIs. `./gradlew assembleRelease publishToMavenLocal` publishes 32 artifacts; the `es` pack AAR holds `assets/blasphem/es.pack` and `META-INF/NOTICE`; the pack POM says CC-BY-NC-SA-4.0, the engine POM Apache-2.0; the BOM pins 31. The `verify` scratch resolves the BOM plus `blasphem` and `blasphem-pack-es` from `mavenLocal()` and finds the three `.so` and `es.pack`. On the API 35 arm64 emulator, an R8-shrunk scratch app passes 7 instrumented cases (README verdict, missing pack and detect artifacts named, order and alias, invalid locales, `packsDirectory`, closed judge, 64 judgements from 8 threads); the mapping shows `Native`, its methods, and `Judgement.<init>` unrenamed. `:engine:connectedDebugAndroidTest` with no test sources passes, so the CI step is inert until the test lands.

**Not verified.** The `swift`, `android`, and `verify` jobs against GitHub and the Central Portal: they need the repository, the DNS record, the key, and the five secrets from open item 2. An iOS device or simulator app; the macOS executable stands in for the bundle lookup. Pre-existing and unrelated: `cargo clippy --workspace --all-targets` and `cargo test -p blasphem-train --test cli` fail at HEAD because `data/raw-v1/hurtlex/*` is staged for deletion in the working tree while `tests/spanish_compatibility.rs`, `tests/cli.rs`, and the `pack` CLI test still read it.
