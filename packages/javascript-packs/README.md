# @blasphem/packs

Internal language data for the [JavaScript](../javascript/README.md) and [React Native](../react-native/README.md) packages.
Each judge loads only its requested model profiles.

Blasphem hashes word and character n-grams into sparse feature vectors.
A linear classifier trained offline scores them with 16-bit weights.
Lexicons and context rules contribute to the verdict.
Detection runs locally without neural networks or cloud inference.

## Installation

These registry commands require the published `1.0.0` release.
Applications install one library. Its exact data dependency resolves automatically:

```sh
npm install blasphem
```

Use [the source build](#build-from-source) for the current checkout.
Python distributes the same files through [blasphem-packs](../python-packs/README.md).

## Usage

Declare `blasphem.locales` in the application's `package.json`.
Node discovers this configuration and its installed packs automatically:

```ts
import { init, judge } from "blasphem";

await init();
console.log(judge("you are a stupid loser"));
```

For custom loaders, the package exports a file map:

```ts
import { FILES } from "@blasphem/packs/files";

const manifestURL = FILES["manifest.json"];
const englishPackURL = FILES["en.pack"];
```

The values are `URL` objects.
The engine verifies pack digests against the manifest.

## Contents

| File | Purpose |
| --- | --- |
| `<code>.pack` | Sparse table, lexicon, and rule-pack identity |
| `<code>.detect` | Language-identification slice |
| `manifest.json` | Format version, file sizes, and SHA-256 digests |
| `files.js` and `files.d.ts` | File URLs for Node and deployment tracing |

Each model profile has a pack and a detection slice.
Detection-disabled judges need only the manifest and packs.
Do not mix engine and data versions.

## Locales

Blasphem supports 16 languages:

```text
ar de en es fr hi id it ja ko ms pt ru tr vi zh
```

Use `id` for Indonesian and `ms` for Malay.

## Browser and native assets

For browser hosting, use [the asset helper](../javascript/README.md#browser-assets).
It copies the WASM module and data into your public directory.

React Native reads files from [the application bundle](../react-native/README.md#bundle-language-data).
Swift and Android build plugins select internal data automatically.
Go locale subpackages embed selected data. Custom directories remain available.

## Build from source

Run from the repository root after installing the [development tools](../../CONTRIBUTING.md#set-up):

```sh
pnpm --filter @blasphem/packs run build
```

The [build script](scripts/build.mjs) exports committed [canonical packs](../../resources/packs/README.md).
It verifies the manifest and copies its artifacts into `packages/javascript-packs/dist/`.
It adds the npm-specific file URLs for deployment tracing.
Run `pnpm packs:generate` after model, lexicon, or language-model changes.
Run `pnpm packs:check` to verify reproducibility.

## Contributing and license

[Corpus changes](../../resources/corpus/README.md) and [lexicon changes](../../resources/lexicon/README.md) require regenerated packs.
Data retains the source terms recorded in [NOTICE](../../NOTICE).
The first-party build code uses [Apache-2.0](../../LICENSE).
