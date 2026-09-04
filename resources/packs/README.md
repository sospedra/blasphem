# Canonical language packs

This directory holds the committed binary artifacts for every distribution.
Keep the packs, detection slices, and manifest in Git.
The repository root holds the canonical NOTICE.
Do not edit generated artifacts by hand.

`manifest.json` records each artifact's byte length and SHA-256 digest.
`<code>.pack` contains a V2 sparse model, lexicon, and rule identity.
`<code>.detect` contains the language-identification slice.
Indonesian and Malay share the `ms` files.

## Generation

Run from the repository root:

```sh
pnpm packs:generate
pnpm packs:check
```

Generation reads `resources/models/multilingual-v2`, `lexicon`, and the compiled language model.
The generator validates source digests before replacing artifacts.
The check regenerates into a temporary directory and compares the manifest.
`pnpm regenerate` regenerates models and then these packs.

## Distribution

Every exporter reads this manifest and verifies artifact sizes and hashes.
Package copies and the generated NOTICE are excluded from Git.
Symlinks are unnecessary and do not form part of a published package.

Runtime adapters and distribution instructions belong under `packages/`.
This directory contains shared artifacts and their documentation.

Use matching engine and pack release versions.
Spanish now uses V2 artifacts. Older engines can reject these packs.
Spanish normalization, features, weights, and linguistic rules remain unchanged.

## License

The data retains the source terms in [NOTICE](../../NOTICE).
Generation code uses Apache-2.0.
