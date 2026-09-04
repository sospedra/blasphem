# blasphem for Python

Local toxicity checks through a PyO3 extension over the Rust engine.
Requires Python 3.10 or later.

Blasphem hashes word and character n-grams into sparse feature vectors.
A linear classifier trained offline scores them with 16-bit weights.
Lexicons and context rules contribute to the verdict.
Detection runs locally without neural networks or cloud inference.

## Installation

Public PyPI packages are pending release.
Use [the source build](#build-from-source) for the current checkout.

The release installation command is:

```sh
python -m pip install blasphem blasphem-packs
```

`blasphem` contains the engine.
`blasphem-packs` supplies the language data.

## Quick start

```python
import blasphem

blasphem.init(["en", "es"], grawlix=True)
try:
    verdict = blasphem.judge("you are a stupid loser")
    print(verdict)
finally:
    blasphem.close()
```

Initialization loads the selected locales.
Reuse the judge for subsequent messages.
Calls to `judge` are synchronous.

## API

| Function | Purpose |
| --- | --- |
| `init(locales, **options)` | Initialize the module judge |
| `judge(text)` | Return a `Judgement` |
| `ready()` | Report whether initialization completed |
| `close()` | Release the module judge |
| `Judge(locales, **options)` | Create an independent judge |

Before `init` and after `close`, the module judge returns a safe verdict.
Initialization with unchanged options reuses the current judge.
Failed replacement initialization keeps the previous judge.

Independent judges support context managers:

```python
from blasphem import Judge

with Judge(["en"], detect_language=False) as detector:
    print(detector.judge("you are a stupid loser"))
```

## Configuration

| Parameter | Default | Meaning |
| --- | --- | --- |
| `locales` | Required | Nonempty iterable of locale codes |
| `assets` | `None` | Pack directory, otherwise installed `blasphem_packs` |
| `detect_language` | `True` | Route to the detected language |
| `grawlix` | `False` | Return masked text |

The options after `locales` are keyword-only.
`assets` accepts a string or `pathlib.Path`.
With detection disabled, the judge returns the highest score across loaded locales.

Use `id` for Indonesian and `ms` for Malay.
See [all 16 supported languages](../javascript-packs/README.md#locales).

## Results and exceptions

`Judgement` is a frozen dataclass:

| Attribute | Type | Meaning |
| --- | --- | --- |
| `safe` | `bool` | No warning is due |
| `score` | `float` | Ordinal value from 0 to 1 |
| `locale` | `str \| None` | Selected model profile |
| `grawlix` | `str \| None` | Masked text when requested |

The score is not a probability.
Unrouted text returns a safe verdict with zero score.

`init` and `Judge` raise `BlasphemError`.
Its `code` identifies locale, asset, digest, or format failures.
An independent judge raises `BLASPHEM_CLOSED` after closure.
See [the public implementation](python/blasphem/__init__.py) for all codes.

## Build from source

Use the committed [canonical packs](../../resources/packs/README.md).
From the repository root:

```sh
python3 packages/python-packs/sync_packs.py
```

Then run from `packages/python`:

```sh
cp ../../NOTICE NOTICE
uv venv .venv
uv pip install --python .venv/bin/python maturin ../python-packs
env VIRTUAL_ENV="$PWD/.venv" .venv/bin/maturin develop --release
```

Run examples with `.venv/bin/python`.
The native extension is [crates/blasphem-python](../../crates/blasphem-python/).
It has a separate Cargo workspace.
Maturin builds abi3 wheels with Python 3.10 as the minimum.

[Contribute](../../CONTRIBUTING.md)

## License

Code uses [Apache-2.0](../../LICENSE).
Language data retains the terms recorded in [NOTICE](../../NOTICE).
