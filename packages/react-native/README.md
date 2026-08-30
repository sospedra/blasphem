# @blasphem/react-native

The `blasphem` pre-send nudge for React Native, on Nitro Modules. Same
`createJudge` contract as the web package. The engine is the Rust core behind
a C ABI, wrapped by C++ HybridObjects; `judge()` is synchronous over JSI.

This package is private and unpublished.

## Use

```ts
import { createJudge } from "@blasphem/react-native";

const judge = await createJudge({ locales: ["en", "es"], detectLanguage: true, grawlix: true });
judge.judge("you are a stupid loser"); // { safe: false, score: 0.64, locale: "en", grawlix: "you are a @#$%&! loser" }
judge.close();
```

`assets` is ignored here. Packs come from the app bundle.

## Packs in the app bundle

Copy the locales you ship from `node_modules/@blasphem/packs/dist/`:

| Platform | Where | Read by |
| --- | --- | --- |
| iOS | a folder reference named `blasphem` in the app target, holding `manifest.json`, `<code>.pack`, `<code>.detect` | `ios/HybridBlasphemAssets.swift` through `Bundle.main` |
| Android | `android/app/src/main/assets/blasphem/` | `HybridBlasphemAssets.kt` through `AssetManager` |

Ship only the locales you request. A requested locale absent from
`manifest.json` throws `BLASPHEM_LOCALE_MISSING` at construction. Every pack is
verified against the manifest digest before it parses.

## Expo web and react-native-web

The `browser` export condition re-exports `createJudge` from `blasphem`, an
optional peer. Install `blasphem` and pass `assets` when the app also targets
the web. The web page then needs the Content Security Policy from the
`blasphem` README: `script-src 'wasm-unsafe-eval'` and the `assets` origins in
`connect-src`. Native builds have no CSP.

## Build

```bash
pnpm --filter @blasphem/react-native run nitrogen     # regenerates nitrogen/generated from src/specs
pnpm --filter @blasphem/react-native run build:rust   # ios/BlasphemFFI.xcframework and android/libs/<abi>/libblasphem_ffi.a
pnpm --filter @blasphem/react-native run build        # dist/ with the inlined core
```

`build:rust` needs the Rust targets `aarch64-apple-ios`,
`aarch64-apple-ios-sim`, `aarch64-linux-android`, `armv7-linux-androideabi`,
and `x86_64-linux-android`, plus Xcode for `xcodebuild -create-xcframework`.
The Android archives are static libraries; the app's NDK links them through
`android/CMakeLists.txt`.

## Layout

| Path | Holds |
| --- | --- |
| `src/specs/BlasphemEngine.nitro.ts` | the Nitro spec: `BlasphemEngineBuilder`, `BlasphemEngine` (C++), `BlasphemAssets` (Swift, Kotlin) |
| `cpp/` | the C++ HybridObjects over `blasphem.h` |
| `ios/` | the Swift asset reader and the vendored XCFramework |
| `android/` | Gradle, CMake, the Kotlin asset reader, the JNI entry |
| `nitrogen/generated/` | nitrogen output, committed |

## Verification status

Verified in this repository: nitrogen generates the specs, the Rust archives
build for every target, the C++ compiles against the generated headers and
Nitro's headers, and the TypeScript compiles. Not verified: an iOS or Android
application build, which needs an example app.
