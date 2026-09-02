# @blasphem/packs

Per-locale data for `blasphem` and `@blasphem/react-native`. The code packages
carry no language data; every judge loads its locales from these files.

```bash
pnpm add @blasphem/packs
```

Python reads the same data from the `blasphem-packs` wheel.

## Contents

| File | Holds |
| --- | --- |
| `<code>.pack` | the sparse table, the lexicon, and the rule-pack version for one language |
| `<code>.detect` | that language's slice of the language-identification model |
| `manifest.json` | `formatVersion` and `{ bytes, sha256 }` per file |

Codes: `ar de en es fr hi it ja ko ms pt ru tr vi zh`. `id` is an alias for `ms`
at the API; the files use `ms`.

Sizes in this build: 30 files, 9.19 MB. `en.pack` 0.42 MB, `en.detect` 0.33 MB.

## Use

Node reads the files from `node_modules` when `init` or `createJudge` gets
no `assets`.

A web application copies `dist/*` next to `blasphem_bg.wasm` into its public
directory and passes that path as `assets`. The loader fetches
`manifest.json`, then one `.pack` and one `.detect` per requested locale.
Nothing else downloads.

React Native copies the chosen locales into the application bundle.

## Integrity

`blasphem` verifies every file against `manifest.json` before it parses a
byte. A mismatch throws `BLASPHEM_DIGEST_MISMATCH` at construction and names
the file. The manifest proves integrity in transit, not provenance.

## Build

```bash
pnpm --filter @blasphem/packs run build
```

This runs `blasphem-train pack`, which checks every artifact and lexicon
against `resources/models/multilingual-v2/manifest.json` and writes `dist/`.

## License

The lexica are HurtLex 1.2, redistributed under CC BY-NC-SA 4.0. See NOTICE.
The code in this repository is Apache-2.0; this package is not.
