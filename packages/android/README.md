# Blasphem for Android

Local toxicity checks for Kotlin applications.
The library calls the Rust engine through JNI.

Blasphem hashes word and character n-grams into sparse feature vectors.
A linear classifier trained offline scores them with 16-bit weights.
Lexicons and context rules contribute to the verdict.
Detection runs locally without neural networks or cloud inference.

## Requirements

Android API 24 or later.
The native library supports `arm64-v8a`, `armeabi-v7a`, and `x86_64`.
The build targets JVM 17.

## Installation

The public Maven Central release is pending.
First [build and publish locally](#build-from-source).

For that local publication, add `mavenLocal()` to your dependency repositories.
The release distribution uses `mavenCentral()`.
Configure these repositories in `settings.gradle.kts`:

```kotlin
pluginManagement {
    repositories {
        google()
        mavenLocal()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositories {
        google()
        mavenLocal()
        mavenCentral()
    }
}
```

Apply the plugin and select languages in `build.gradle.kts`:

```kotlin
plugins {
    id("me.sospedra.blasphem") version "2.0.0"
}
blasphem {
    locales.set(listOf("en", "es")) // Or locales.set("all").
    assets.set("bundled") // Default. Use "remote" for jsDelivr.
    detectLanguage.set(true) // Independent of the delivery mode.
}
```

## Usage

`context` is your application's Android `Context`.

```kotlin
import me.sospedra.blasphem.Judge
// Inside a coroutine. Configuration comes from the Gradle plugin.
Judge.create(context).use { judge ->
    val verdict = judge.judge("you are a stupid loser")
    println(verdict)
}
```

The suspend factory reads files through the IO dispatcher.
Keep one judge for repeated checks.
`judge` is synchronous and supports concurrent callers.
Call `close()` when its owner no longer needs it.

## Artifacts and configuration

| Artifact | Contents |
| --- | --- |
| `blasphem` | Kotlin API and native engine |
| `blasphem-pack-<code>` | One toxicity pack |
| `blasphem-detect-<code>` | One language-detection slice |
| `blasphem-bom` | Matching artifact versions |

The plugin adds the engine and exact internal data dependencies automatically.
Gradle merges only selected data AARs into `assets/blasphem/`.
Both delivery modes emit `assets/blasphem/bundle.json`.
The selection must contain at least one supported language or `"all"`.
Unknown codes and empty selections fail the build.
Detection defaults to `true`.
Set `detectLanguage.set(false)` to omit detection files.
The judge then returns the highest score across loaded locales.

Use `id` for Indonesian and `ms` for Malay.
Both use `blasphem-pack-ms` and `blasphem-detect-ms`.
See [all 16 supported languages](../javascript-packs/README.md#locales).

`grawlix` defaults to `false`.
`JudgeOptions.packsDirectory` accepts a `java.io.File` instead of bundled assets.
That directory contains the pack and required detection files.
Use `Judge.create(context, JudgeOptions(...))` for explicit directories or runtime options such as `grawlix`.
This advanced overload remains synchronous.

## Remote data

`assets.set("remote")` bundles configuration and the native engine without language data.
`"jsdelivr"` remains a compatibility alias.
The factory downloads selected files from the exact `@blasphem/packs@2.0.0` jsDelivr release.
The build embeds the trusted release manifest length and SHA-256 digest.
Remote builds make no CDN requests.
Initialization fails if that exact release is unavailable.

Private files storage retains verified manifests and data across process restarts.
The storage separates versions, format versions, filenames, and integrity identities.
Each cache read verifies the file length and SHA-256 digest.
Downloads have 30-second request deadlines and at most two attempts per file.
Concurrent requests share verified downloads, and temporary files commit through atomic renames.
The factory constructs an engine only after every selected file is available.
Valid cached selections start offline with no network requests.
Cancellation and failed initialization preserve valid cached files.

## Results and errors

`Judgement` contains `safe: Boolean`, `score: Double`, `locale: String?`, and `grawlix: String?`.
`grawlix` contains masked text for unsafe verdicts when requested, otherwise `null`.
The score is ordinal, between 0 and 1.
It is not a probability.
Unrouted text returns a safe verdict with zero score.

`Judge.create` throws `BlasphemException` for invalid options, missing data, or invalid packs.
Its `code` and `message` describe the failure.
A closed judge throws `Code.CLOSED`.
See [the Kotlin API](engine/src/main/kotlin/me/sospedra/blasphem/).

## R8

The AAR includes [consumer rules](engine/consumer-rules.pro).
They preserve the JNI method names and native result constructor.
Applications do not need to copy those rules.

## Build from source

Install the [development tools](../../CONTRIBUTING.md#set-up), JDK 17, Android SDK, NDK, and `cargo-ndk`.
Run from the repository root:

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
env RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384" \
  cargo ndk --platform 24 -t arm64-v8a -t armeabi-v7a -t x86_64 \
  -o packages/android/engine/src/main/jniLibs build --release --locked -p blasphem-jni
node packages/android/scripts/sync-packs.mjs
```

Then run from `packages/android`:

```sh
./gradlew assembleRelease
./gradlew publishToMavenLocal -PRELEASE_SIGNING_ENABLED=false
```

The sync script creates the locale modules before Gradle starts.
It also generates plugin locale tables and trusted release manifest constants.
The plugin publishes an implementation JAR and the `me.sospedra.blasphem` marker POM.
`generatePomFileForPluginMavenPublication` and
`generatePomFileForBlasphemPluginMarkerMavenPublication` inspect metadata without publication.
The linker flag requests 16 KB page alignment.
Local publication makes the artifacts available through `mavenLocal()`.

[Contribute](../../CONTRIBUTING.md)

## License

Code uses [Apache-2.0](../../LICENSE).
Language data retains the terms recorded in [NOTICE](../../NOTICE).
