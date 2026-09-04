# @blasphem/packs

Language data for the [JavaScript](../blasphem/README.md) and [React Native](../react-native/README.md) packages.
Each judge loads only its requested model profiles.

## Installation

The public npm release is pending.
For published versions, install matching engine and data versions:

```sh
npm install blasphem @blasphem/packs
```

Use [the source build](#build-from-source) for the current checkout.
Python distributes the same files through [blasphem-packs](../python-packs/README.md).

## Usage

Node discovers installed packs automatically:

```ts
import { init, judge } from "blasphem";

await init({ locales: ["en", "es"] });
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

Supported language codes:

```text
ar de en es fr hi id it ja ko ms pt ru tr vi zh
```

Use `id` for Indonesian and `ms` for Malay.
Both load `ms.pack` and `ms.detect`.
Results report the canonical model code, `ms`.
The 16 public language codes map to 15 model profiles.

## Browser and native assets

For browser hosting, use [the asset helper](../blasphem/README.md#browser-assets).
It copies the WASM module and data into your public directory.

React Native reads files from [the application bundle](../react-native/README.md#bundle-language-data).
Swift and Android supply data through their package products.
Go accepts a directory or an `fs.FS`.

## Build from source

Run from the repository root after installing the [development tools](../../CONTRIBUTING.md#set-up):

```sh
pnpm --filter @blasphem/packs run build
```

The [build script](scripts/build.mjs) runs `blasphem-train pack`.
It reads the model manifest, compiled tables, clean-room lexica, and language model.
It writes the distribution to `packages/packs/dist/`.
The manifest records the current file sizes and digests.

## Contributing and license

[Corpus changes](../../corpus/README.md) and [lexicon changes](../../lexicon/README.md) require regenerated packs.
Data retains the source terms recorded in [NOTICE](../../NOTICE).
The first-party build code uses [Apache-2.0](../../LICENSE).
