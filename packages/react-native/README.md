# @blasphem/react-native

Local toxicity checks for React Native through Nitro Modules.
The Rust engine runs behind C++ HybridObjects.
Calls to `judge` are synchronous over JSI.

Blasphem hashes word and character n-grams into sparse feature vectors.
A linear classifier trained offline scores them with 16-bit weights.
Lexicons and context rules contribute to the verdict.
Detection runs locally without neural networks or cloud inference.

## Installation

These registry commands require the published `1.0.0` release.
Use [the source build](#build-from-source) for the current checkout.

Install the native module and its required Nitro peer:

```sh
npm install @blasphem/react-native react-native-nitro-modules
```

The library resolves its exact internal data dependency automatically.
Development installations contain all data. Application bundles contain only selected data.
Choose [bundled locales](#bundle-language-data) or [CDN downloads](#download-language-data).

For bare React Native, run CocoaPods from the app's iOS directory:

```sh
bundle exec pod install
```

Rebuild the native application after installation.
This module requires a native build.
Nitro remains required with either data source.

## Expo

This package targets Expo SDK 57, React Native 0.86.3, and Nitro 0.37.1.
Expo SDK 57 requires iOS 16.4 or later.
Use a development or production build. Expo Go cannot load this native module.

```sh
npx expo install @blasphem/react-native react-native-nitro-modules
```

Register the plugin in `app.json`:

```json
{
  "expo": {
    "plugins": ["@blasphem/react-native/app.plugin"]
  }
}
```

The plugin reads the `blasphem` selection from your application's `package.json`.
It prepares the same assets used by CocoaPods and Gradle.
`@expo/config-plugins` is an optional peer for Expo integration.
Bare React Native applications do not need Expo packages.

Build the native application after choosing a data source:

```sh
npx expo run:ios
npx expo run:android
```

## Requirements

| Platform | Requirement |
| --- | --- |
| iOS | iOS 15.1+, CocoaPods, arm64 device or simulator |
| Android | API 24+, Android NDK, supported Rust archives |
| JavaScript | React Native 0.86.3 and `react-native-nitro-modules` 0.37.1 peers |

The current iOS archive has no Intel simulator slice.
Android archives cover `arm64-v8a`, `armeabi-v7a`, `x86`, and `x86_64`.
The package manifest pins the supported peer versions.

## Bundle language data

Select locales in your application's `package.json`:

```json
{
  "blasphem": {
    "locales": ["en", "es"],
    "assets": "bundled",
    "detectLanguage": true
  }
}
```

CocoaPods and Gradle copy selected data automatically. Manual file copies are unnecessary.
Only selected `.pack` files and their manifest enter the application binary.
With detection enabled, the build also includes selected `.detect` files.
`detectLanguage: false` omits detection files and becomes the bundled initialization default.
Missing configuration, unknown locales, and empty arrays fail the build.
Use `"locales": "all"` for every locale in the installed release.
Removing a locale removes its generated assets on the next build.

The packs version must match the native package version.
Missing packs, unsupported locales, and invalid digests stop the build.
After changing the selection, rerun CocoaPods on iOS and rebuild the native application.
Expo prebuild also refreshes the selected assets.

## Download language data

Keep the same locale selection and set `"assets": "remote"` in `package.json`.
The build emits configuration and notices, without language data.
The native engine remains bundled. The build makes no CDN requests.
Both delivery modes use configuration-only initialization:

```ts
import { init, judge } from "@blasphem/react-native";

await init();
const verdict = judge("you are a stupid loser");
```

The native loader downloads the exact package version from jsDelivr.
It requests only selected locales and required detection files.
It verifies file sizes and SHA-256 digests before storing the data locally.
Data stays in iOS Application Support or Android application files storage.
Later launches reuse verified files and the manifest without network requests.

Each locale version downloads once per installation while its stored data remains valid.
Data removal, corruption, or a package version change can require another download.
First use needs a network connection. Failed downloads reject initialization.
After initialization, `judge()` stays synchronous and runs locally.
Concurrent requests share verified downloads. Each file gets at most two attempts.
Downloads have a 30-second deadline and use atomic native file replacement.
Cancellation and failed replacement preserve valid stored data.
Version namespaces remain separate during upgrades.
Remote delivery changes bundle size, not selected model memory.

The CDN requires the matching npm release. Unpublished versions cannot download from jsDelivr.

## Usage

```ts
import { init, judge } from "@blasphem/react-native";

await init({ grawlix: true });

const verdict = judge("you are a stupid loser");
console.log(verdict);
```

Initialize once and reuse the judge.
Check the initialization promise for asset errors.

## API and configuration

| Export | Purpose |
| --- | --- |
| `init(options?)` | Read generated configuration and initialize the module judge |
| `judge(text)` | Check a message synchronously |
| `ready()` | Report whether the module judge is ready |
| `close()` | Release the module judge |
| `createJudge(options)` | Build an independent judge |

Application configuration requires a nonempty `locales` array or `"all"`.
Delivery defaults to `"bundled"`. Detection defaults to `true`.
Default initialization needs no repeated locale, delivery, or detection selection.
`grawlix` defaults to `false`.
Advanced independent instances retain explicit options.
`assets: "jsdelivr"` remains an alias for `"remote"`.
Other native `assets` values reject with `BLASPHEM_ASSETS_REQUIRED`.
The runtime locale selection must have matching files in the chosen data source.

Use `id` for Indonesian and `ms` for Malay.
Both need the `ms` data files.
See [all 16 supported languages](../javascript-packs/README.md#locales).

Results contain `safe`, `score`, `locale`, and `grawlix`.
`grawlix` contains masked text for unsafe verdicts when requested, otherwise `null`.
TypeScript narrows `grawlix` to `null` when `safe` is `true`.
The score is ordinal, between 0 and 1.
It is not a probability.

Before initialization and after closure, the module judge returns a safe verdict.
Failed replacement initialization preserves the previous ready judge.
Independent judges throw `BLASPHEM_CLOSED` after closure.
See [the public exports](src/index.ts) and [shared contract](../javascript-common/src/contract.ts).

## Asset errors

Initialization rejects missing files, unsupported locales, and invalid data.
The error carries a `BLASPHEM_*` code.
Pack digests must match `manifest.json`.
See [the shared error codes](../javascript-common/src/errors.ts).

## Web support

The browser export forwards to the automatically installed `blasphem` dependency.
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

Link `packages/react-native` into the consuming application and install its Nitro peer.
Configure the selected locales in the application's `package.json`.
Run CocoaPods or Expo prebuild, then rebuild the app.

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
