# blasphem-packs

Language data for the [Blasphem Python package](../python/README.md).
This wheel contains the manifest, toxicity packs, and language-detection slices.
Requires Python 3.10 or later.

## Installation

The public PyPI release is pending.
For published versions, install the matching engine and data versions:

```sh
python -m pip install blasphem blasphem-packs
```

For the current checkout, use [the source build](#build-from-source).

## Usage

The engine discovers installed data automatically:

```python
import blasphem

blasphem.init(["en"])
print(blasphem.judge("you are a stupid loser"))
blasphem.close()
```

To obtain the data directory explicitly:

```python
from blasphem_packs import directory

print(directory())
```

`directory()` returns a `pathlib.Path`.
Pass it as `assets` when constructing a Python judge.

## Contents

| File | Purpose |
| --- | --- |
| `manifest.json` | Format version and file digests |
| `<code>.pack` | Toxicity model, lexicon, and rule identity |
| `<code>.detect` | Language-detection data |

See [the shared pack guide](../packs/README.md#locales) for language codes.
The data wheel carries no Python extension.

## Build from source

Build the shared packs from the repository root:

```sh
pnpm --filter @blasphem/packs run build
python3 packages/python-packs/sync_packs.py
```

Build the wheel from `packages/python-packs`:

```sh
uv build
```

The sync script copies the generated data from `packages/packs/dist/`.
Rebuild the wheel after any pack change.

[Contribute](../../CONTRIBUTING.md)

## License

The data retains its source terms.
See the repository [NOTICE](../../NOTICE) for the current license records.
