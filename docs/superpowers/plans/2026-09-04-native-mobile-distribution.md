# Native mobile distribution implementation plan

Spec: `docs/superpowers/specs/2026-09-04-native-mobile-distribution-design.md`. Executed 2026-09-04 in one session; boxes record what landed.

Two spec statements meet contrary facts on disk. The plan follows the facts:

1. The spec says `Locales.generated.swift` and `Locales.generated.kt` are gitignored "like the TypeScript table". `packages/core/src/locales.generated.ts`, `packages/go/locales.go`, and `packages/python/python/blasphem/_locales.py` are committed and a test in `locales_table.rs` fails when one is stale. The Swift and Kotlin tables are committed the same way, so `swift build` and Gradle work from a clean checkout.
2. The spec marks `JudgeTests.swift` and `JudgeTest.kt` "pending Rubén's approval". They are not written. Scratch hosts outside the repository verify the same cases, and the Gherkin for both files goes to Rubén with the report.

Toolchain pins: AGP 8.13.1, Gradle 8.14.5, Kotlin 2.2.21, `com.vanniktech.maven.publish` 0.37.0, `jni` 0.21, `compileSdk 35`, `minSdk 24`.

## Task 1: locale tables and the version mirror

- [x] `crates/blasphem-train/src/locales_table.rs`: `TableFormat::Swift` and `TableFormat::Kotlin`; `parse` accepts `swift`, `kotlin`, `kt`; `swift_table()` and `kotlin_table()` render `Language::ALL` with aliases.
- [x] Extend `every_format_lists_fifteen_locales_with_the_id_alias_on_ms` and `the_committed_tables_are_current` with the two formats and the two paths `packages/swift/Sources/Blasphem/Locales.generated.swift` and `packages/android/engine/src/main/kotlin/me/sospedra/blasphem/Locales.generated.kt`.
- [x] `crates/blasphem-train/src/main.rs`: the `--format` help and the error text list the five formats.
- [x] `crates/blasphem-train/src/versions.rs`: mirror `packages/android/gradle.properties` with `(?m)^VERSION_NAME=(.+)$`.
- [x] Write both tables with `cargo run -p blasphem-train -- locales-table --format swift|kotlin --output <path>`.
- [x] `cargo test -p blasphem-train locales_table versions` passes.

## Task 2: `crates/blasphem-jni`

- [x] `Cargo.toml`: `crate-type = ["cdylib"]`, `publish = false`, `blasphem` with `default-features = false, features = ["language-detection"]`, `jni = "0.21"`. Root `Cargo.toml` adds it to `members`, not `default-members`. `Cargo.lock` gains `jni`.
- [x] `src/lib.rs`: the seven `Java_me_sospedra_blasphem_Native_*` functions from the spec table. A `Builder { sources, detect_language, grawlix }` box and an `Engine` box travel as `jlong`. Failures throw `java.lang.RuntimeException` with the engine's `CODE: detail`. `engineJudge` constructs `me/sospedra/blasphem/Judgement` through `(ZDLjava/lang/String;Ljava/lang/String;)V`.
- [x] `cargo clippy --workspace --all-targets --locked -- -D warnings` clean. `cargo fmt --all --check` clean.
- [x] Host check: a scratch `Native.java` and `Judgement.java` in `/tmp` load the macOS cdylib and judge the README example.
- [x] `RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384" cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o packages/android/engine/src/main/jniLibs build --release --locked -p blasphem-jni`; `llvm-readelf -l` shows `LOAD` aligned to `0x4000` on `arm64-v8a` and `x86_64`.

## Task 3: `packages/swift`

- [x] `Package.swift` (development manifest): tools 5.9, iOS 15.1, macOS 12, `binaryTarget(path: "BlasphemFFI.xcframework")`, target `Blasphem`, no test target until the pending test file exists, because SwiftPM rejects a declared target without sources.
- [x] `Sources/Blasphem/BlasphemError.swift`: `struct BlasphemError: Error` with `Code` (nine cases, raw values are the `BLASPHEM_*` strings) and `message`; `static func fromEngine(_ text: String)` parses `CODE: detail`.
- [x] `Sources/Blasphem/Judgement.swift`: `struct Judgement: Equatable, Sendable` with `safe`, `score`, `locale`, `grawlix`.
- [x] `Sources/Blasphem/Locales.swift`: `normalizeLocales([String]) throws -> [String]` over the generated table.
- [x] `Sources/Blasphem/Artifacts.swift`: `resourceURL(product:file:)` searches `Bundle.main.url(forResource:withExtension:)`, `Bundle(for: Judge.self).resourceURL`, `Bundle.main.bundleURL`; `readPack`, `readDetect` with `packsDirectory` override; missing bundle raises `.localeMissing` "add the product BlasphemPackES to the target".
- [x] `Sources/Blasphem/Judge.swift`: `final class Judge: @unchecked Sendable`; `init(locales:detectLanguage:grawlix:packsDirectory:) throws`; `locales`; `judge(_:) throws -> Judgement` under a read lock, `.closed` after `close()`; `close()` under the write lock; `deinit` closes.
- [x] `scripts/xcframework.mjs`: staticlib for `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `aarch64-apple-darwin` with `CARGO_TARGET_DIR=target/ffi`; headers directory with `blasphem.h` and `module.modulemap`; `xcodebuild -create-xcframework`; `ditto -c -k --keepParent` to `BlasphemFFI.xcframework.zip`; prints slice sizes in MB.
- [x] `scripts/distribution.mjs`: `--version`, `--checksum`, `--output <dir>` (render only) or `--repo <ssh url>` (clone, replace, commit `Publish <version>`, push `main`, tag `v<version>`; existing tag prints `status=exists`). Renders the 32-target manifest, copies `Sources/Blasphem`, writes `Sources/BlasphemPack<CODE>/{BlasphemPack<CODE>.swift,Resources/<code>.pack}` and the detect twins from `packages/packs/dist`, copies `LICENSE`, `NOTICE`, `README.md`.
- [x] `README.md`, `NOTICE`.
- [x] `.gitignore`: the XCFramework, its zip, `.build`, `.swiftpm`.
- [x] `node packages/swift/scripts/xcframework.mjs`; `swift build` in `packages/swift`.
- [x] Scratch executable in `/tmp` against the rendered distribution (path binary target): README verdict, registry order, `detectLanguage: false`, unknown code, missing product, closed judge, `SUPPLIED_CASES`; four bundles and their bytes.

## Task 4: `packages/android`

- [x] `settings.gradle.kts`: plugin repositories, `include(":engine", ":bom")`, one `:packs:<code>:pack` / `:packs:<code>:detect` per file present.
- [x] `build.gradle.kts`: plugins `apply false`; `engine` gets Kotlin, `consumer-rules.pro`, Apache-2.0 POM; pack and detect modules get unique namespaces `me.sospedra.blasphem.pack.<code>` / `.detect.<code>` (AGP 8 rejects duplicate namespaces), CC-BY-NC-SA-4.0 POM; every Android module publishes with `AndroidSingleVariantLibrary("release", sourcesJar, javadocJar)`; `bom` is a `java-platform` with 31 constraints.
- [x] `gradle.properties`: `VERSION_NAME`, `GROUP=me.sospedra.blasphem`, `SONATYPE_HOST=CENTRAL_PORTAL`, `SONATYPE_AUTOMATIC_RELEASE=true`, `RELEASE_SIGNING_ENABLED=true`, shared POM fields.
- [x] Gradle wrapper 8.14.5 from the cached Gradle 9.3.1 binary.
- [x] `engine/src/main/kotlin/me/sospedra/blasphem/`: `Native.kt`, `Judgement.kt`, `BlasphemException.kt` with `Code`, `JudgeOptions.kt`, `Locales.kt`, `Judge.kt` (`create(context, options)`, `AutoCloseable`, `ReentrantReadWriteLock`); `engine/consumer-rules.pro` keeps `Native` methods and the `Judgement` constructor.
- [x] `scripts/sync-packs.mjs`: writes `packs/<code>/{pack,detect}/src/main/{assets/blasphem/<file>,resources/META-INF/NOTICE,AndroidManifest.xml}`.
- [x] `README.md`, `NOTICE`; `.gitignore`: `packs/`, `jniLibs/`, `.gradle/`, `build/`, `local.properties`.
- [x] `./gradlew assembleRelease publishToMavenLocal -PRELEASE_SIGNING_ENABLED=false`; AAR lists `jni/<abi>/libblasphem_jni.so` and `assets/blasphem/es.pack`.
- [x] Scratch app in `/tmp` resolves the BOM from `mavenLocal()`; an R8-shrunk instrumented run on the `blasphem_probe` emulator (API 35, arm64-v8a) passes 7 cases: the README verdict, a missing pack and a missing detect slice naming the artifact, registry order and the `id` alias, invalid locales, `packsDirectory`, a closed judge, 64 judgements from 8 threads.

## Task 5: `publish.yml`

- [x] Header comment: npm, PyPI, and Go authenticate over OIDC; Maven Central and the Swift repository hold five repository secrets.
- [x] Job `swift` on `macos-15`, `contents: write`: targets, cache `target/ffi`, `xcframework.mjs`, `gh release upload --clobber`, checksum, `packs-dist`, SSH deploy key, `distribution.mjs`.
- [x] Job `android` on `ubuntu-24.04`: `cargo-ndk`, targets, JNI build with the page-size flag, `packs-dist`, `sync-packs.mjs`, emulator smoke on API 35 `x86_64`, `publishAllPublicationsToMavenCentralRepository` with the four secrets mapped to `ORG_GRADLE_PROJECT_*`.
- [x] Job `verify`: `needs` gains `swift` and `android`; macOS step builds the scratch executable against `sospedra/blasphem-swift` at `v<version>`; Linux step polls Maven Central for 30 minutes, resolves the BOM plus `blasphem-pack-es`, and lists the AAR entries.

## Task 6: documentation and measurements

- [x] Root `README.md`: `## Swift` and `## Android` sections after `## React Native`.
- [x] Spec: status line and implementation notes with the engine sizes, the detection share, and the bundle bytes.
- [x] `dist generate --check` clean. `cargo test -p blasphem-train --lib` passes; the `cli` test binary and `cargo clippy --workspace --all-targets` fail at HEAD on `data/raw-v1/hurtlex`, which the clean-room work has staged for deletion; every other target is clippy clean.

## Follow-ups (not in this plan)

- Rubén: create `sospedra/blasphem-swift` with a write deploy key, verify `me.sospedra` on the Central Portal, generate a GPG key, store the five secrets.
- The two test files, once approved.
- A second engine build without `language-detection`, if the measured share exceeds 1.00 MB.
