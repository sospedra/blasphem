# @blasphem/react-native

Local toxicity checks for React Native through Nitro Modules.
The Rust engine runs behind C++ HybridObjects.
Calls to `judge` are synchronous over JSI.

Blasphem hashes word and character n-grams into sparse feature vectors.
A linear classifier trained offline scores them with 16-bit weights.
Lexicons and context rules contribute to the verdict.
Detection runs locally without neural networks or cloud inference.

## Installation

The public npm release is pending.
Use [the source build](#build-from-source) for the current checkout.

The release installation command includes the Nitro peer dependency:

```sh
npm install @blasphem/react-native @blasphem/packs react-native-nitro-modules
```

Run CocoaPods installation from your app's iOS directory:

```sh
bundle exec pod install
```

Rebuild the native application after installation.
This module requires a native build.
For Expo applications, use a development build with native projects and bundled assets.

## Requirements

| Platform | Requirement |
| --- | --- |
| iOS | iOS 15.1+, CocoaPods, arm64 device or simulator |
| Android | API 24+, Android NDK, supported Rust archives |
| JavaScript | React Native and `react-native-nitro-modules` peers |

The current iOS archive has no Intel simulator slice.
Android archives cover `arm64-v8a`, `armeabi-v7a`, `x86`, and `x86_64`.
The package manifest records development versions.
It does not define a tested React Native version range.

## Bundle language data

Copy files from `node_modules/@blasphem/packs/dist/` into your application:

| Platform | Destination |
| --- | --- |
| iOS | A folder reference named `blasphem` in the app target |
| Android | `android/app/src/main/assets/blasphem/` |

For English and Spanish with detection, include:

```text
manifest.json
en.pack
en.detect
es.pack
es.detect
```

If detection is disabled, omit the `.detect` files.
Keep the matching manifest.
A manifest can list unused locales without requiring their files.

## Usage

```ts
import { init, judge } from "@blasphem/react-native";

await init({ locales: ["en", "es"], grawlix: true });

const verdict = judge("you are a stupid loser");
console.log(verdict);
```

Initialize once and reuse the judge.
Check the initialization promise for asset errors.

## API and configuration

| Export | Purpose |
| --- | --- |
| `init(options)` | Load bundled data and initialize the module judge |
| `judge(text)` | Check a message synchronously |
| `ready()` | Report whether the module judge is ready |
| `close()` | Release the module judge |
| `createJudge(options)` | Build an independent judge |

The options require a nonempty `locales` array.
`detectLanguage` defaults to `true`.
`grawlix` defaults to `false`.
Native loaders ignore `assets` and read the app bundle.

Use `id` for Indonesian and `ms` for Malay.
Both need the `ms` data files.
See [all 16 supported languages](../javascript-packs/README.md#locales).

Results contain `safe`, `score`, `locale`, and `grawlix`.
The score is ordinal, between 0 and 1.
It is not a probability.

Before initialization and after closure, the module judge returns a safe verdict.
Independent judges throw `BLASPHEM_CLOSED` after closure.
See [the public exports](src/index.ts) and [shared contract](../javascript-common/src/contract.ts).

## Asset errors

Initialization rejects missing files, unsupported locales, and invalid data.
The error carries a `BLASPHEM_*` code.
Pack digests must match `manifest.json`.
See [the shared error codes](../javascript-common/src/errors.ts).

## Web support

The browser export forwards to the optional `blasphem` peer.
Install that package when the application targets the web.
Configure its [browser assets and CSP](../javascript/README.md#browser-assets).
The native bundle setup does not provide web assets.

## Build from source

Install the [development tools](../../CONTRIBUTING.md#set-up), Xcode, and the required Rust targets:

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

Run from the repository root:

```sh
pnpm install --frozen-lockfile
pnpm --filter @blasphem/packs run build
pnpm --filter @blasphem/react-native run nitrogen
pnpm --filter @blasphem/react-native run build:rust
pnpm --filter @blasphem/react-native run build
```

Link `packages/react-native` and `packages/javascript-packs` into the consuming application.
Install its Nitro peer and copy the language data.
Run CocoaPods and rebuild the app.

The TypeScript check is:

```sh
pnpm --filter @blasphem/react-native run check
```

Package compilation does not verify native autolinking or device behavior.
Check initialization and judging in the consuming iOS and Android applications.

[Contribute](../../CONTRIBUTING.md)

## License

Code uses [Apache-2.0](../../LICENSE).
Language data retains the terms recorded in [NOTICE](../../NOTICE).
