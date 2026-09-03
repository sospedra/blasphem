# blasphem (Android)

Multilingual pre-send toxicity nudge for Android apps, over the Rust core
through JNI. Same `judge` contract as the JavaScript package. Maven Central,
group `me.sospedra.blasphem`, `minSdk 24`.

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

```kotlin
import me.sospedra.blasphem.Judge
import me.sospedra.blasphem.JudgeOptions

val judge = Judge.create(context, JudgeOptions(locales = listOf("en", "es"), detectLanguage = true, grawlix = true))
judge.judge("you are a stupid loser")
// Judgement(safe=false, score=0.95, locale=en, grawlix=you are a @#$%&! loser)
judge.locales   // [en, es]
judge.close()
```

`Judge.create` reads the packs from the app assets and builds the engine. It
blocks on file reads, so call it off the main thread. `judge` is synchronous
and safe from several threads. After `close()` it throws `BlasphemException`
with `Code.CLOSED`. `Judge` implements `AutoCloseable`.

## Artifacts

| Artifact | Holds |
| --- | --- |
| `blasphem` | the Kotlin wrapper and `libblasphem_jni.so` for `arm64-v8a`, `armeabi-v7a`, `x86_64` |
| `blasphem-pack-<code>` | `assets/blasphem/<code>.pack`: the sparse table, the lexicon, and the rule-pack version for one language |
| `blasphem-detect-<code>` | `assets/blasphem/<code>.detect`: that language's slice of the language-identification model |
| `blasphem-bom` | a Maven BOM pinning all 31 artifacts to one version |

Codes: `ar de en es fr hi it ja ko ms pt ru tr vi zh`. `id` is an alias for
`ms` at the API; the artifacts use `ms`. Add one pack artifact per locale you
request, and one detect artifact per locale when `detectLanguage` is true, the
default. Gradle merges the `assets/blasphem/` folder of every AAR into the
APK. A missing artifact fails at construction with `Code.LOCALE_MISSING` and
names the artifact to add.

The asset path matches what `@blasphem/react-native` reads, so an app that has
both sees one folder.

`JudgeOptions.packsDirectory` reads `<code>.pack` and `<code>.detect` from a
folder instead of the assets. Tests and command-line hosts use it.

## Errors

`Judge.create` throws `BlasphemException`; `code` is one of `LOCALES_EMPTY`,
`LOCALE_UNSUPPORTED`, `LOCALE_MISSING`, `ASSETS_REQUIRED`, `FETCH_FAILED`,
`DIGEST_MISMATCH`, `FORMAT_VERSION`, `PACK_INVALID`, `CLOSED`. `message`
carries the detail. `ASSETS_REQUIRED` and `DIGEST_MISMATCH` exist for parity
with the JavaScript codes and are never thrown here.

## Shrinking

`consumer-rules.pro` ships in the AAR and keeps the native method names and
the `Judgement` constructor the engine constructs through JNI. No app-side
rule is needed.

## Build

```bash
RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384" \
  cargo ndk --platform 24 -t arm64-v8a -t armeabi-v7a -t x86_64 \
  -o packages/android/engine/src/main/jniLibs build --release --locked -p blasphem-jni
node packages/android/scripts/sync-packs.mjs      # packs/<code>/{pack,detect} modules from packages/packs/dist
cd packages/android && ./gradlew assembleRelease  # every AAR under */build/outputs/aar
```

`cargo-ndk` and the Android NDK produce the three `.so`. The page-size flag
aligns every `LOAD` segment to 16 KB, which Google Play requires for apps that
target API 35 and later. `settings.gradle.kts` includes one module per data
file present under `packs/`, so the sync must run before Gradle. The Rust
crate is `crates/blasphem-jni`; the Kotlin table `Locales.generated.kt` comes
from `blasphem-train locales-table --format kotlin`.

The `android` job in `.github/workflows/publish.yml` builds, runs the
instrumented smoke test on an API 35 emulator, and publishes to the Central
Portal with `com.vanniktech.maven.publish`.

## License

`blasphem` and `blasphem-bom` are Apache-2.0. The pack and detect artifacts
carry data under CC BY-NC-SA 4.0 and include NOTICE.
