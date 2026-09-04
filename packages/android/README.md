# Blasphem for Android

Local toxicity checks for Kotlin applications.
The library calls the Rust engine through JNI.

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
dependencyResolutionManagement {
    repositories {
        google()
        mavenLocal()
        mavenCentral()
    }
}
```

Add the BOM, engine, and language data in `build.gradle.kts`:

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

## Usage

`context` is your application's Android `Context`.

```kotlin
import me.sospedra.blasphem.Judge
import me.sospedra.blasphem.JudgeOptions

val options = JudgeOptions(
    locales = listOf("en", "es"),
    grawlix = true,
)

Judge.create(context, options).use { judge ->
    val verdict = judge.judge("you are a stupid loser")
    println(verdict)
}
```

Construction reads files and should run off the main thread.
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

Gradle merges each data AAR into `assets/blasphem/`.
Add one pack for each requested model profile.
Detection defaults to `true` and requires matching detection artifacts.
Set `detectLanguage = false` to omit them.
The judge then returns the highest score across loaded locales.

Use `id` for Indonesian and `ms` for Malay.
Both use `blasphem-pack-ms` and `blasphem-detect-ms`.
See [all locale codes](../packs/README.md#locales).

`grawlix` defaults to `false`.
`JudgeOptions.packsDirectory` accepts a `java.io.File` instead of bundled assets.
That directory contains the pack and required detection files.
The Kotlin loader does not read `manifest.json`.

## Results and errors

`Judgement` contains `safe: Boolean`, `score: Double`, `locale: String?`, and `grawlix: String?`.
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
pnpm --filter @blasphem/packs run build
node packages/android/scripts/sync-packs.mjs
```

Then run from `packages/android`:

```sh
./gradlew assembleRelease
./gradlew publishToMavenLocal -PRELEASE_SIGNING_ENABLED=false
```

The sync script creates the locale modules before Gradle starts.
The linker flag requests 16 KB page alignment.
Local publication makes the artifacts available through `mavenLocal()`.

[Contribute](../../CONTRIBUTING.md)

## License

Code uses [Apache-2.0](../../LICENSE).
Language data retains the terms recorded in [NOTICE](../../NOTICE).
