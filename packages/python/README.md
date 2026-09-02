# blasphem (Python)

Multilingual pre-send toxicity nudge over the Rust core, as a PyO3 extension.
Same contract as the JavaScript package.

```bash
pip install blasphem blasphem-packs
```

The wheel is abi3 and covers Python 3.10 and later on macOS, Linux (glibc and
musl), and Windows.

```python
import blasphem

blasphem.init(["en", "es"], grawlix=True)
blasphem.judge("you are a stupid loser")
# Judgement(safe=False, score=0.64, locale='en', grawlix='you are a @#$%&! loser')
```

`init` loads the locales once and installs the module judge. `judge` is
synchronous and never raises: before `init` and after `close` it returns the
fail-open verdict. `ready()` tells which. `init` with the same options is free;
with other options it builds a new judge first and retires the old one after.

`blasphem.Judge(locales, *, assets=None, detect_language=True, grawlix=False)`
builds an independent judge, usable as a context manager, when one per module
is not enough.

## Packs

Packs come from `assets`, a directory with `manifest.json` and the `.pack` and
`.detect` files, or from the installed `blasphem-packs` package when `assets`
is omitted. Every file is verified against the manifest before it parses.

## Errors

`init` and `Judge` raise `BlasphemError`; `.code` is one of
`BLASPHEM_LOCALES_EMPTY`, `BLASPHEM_LOCALE_UNSUPPORTED`, `BLASPHEM_LOCALE_MISSING`,
`BLASPHEM_ASSETS_REQUIRED`, `BLASPHEM_FETCH_FAILED`, `BLASPHEM_DIGEST_MISMATCH`,
`BLASPHEM_FORMAT_VERSION`, `BLASPHEM_PACK_INVALID`. A `Judge` raises
`BLASPHEM_CLOSED` after `close()`; the module-level `judge` never raises.

## Build

```bash
uv venv .venv && uv pip install --python .venv/bin/python maturin
VIRTUAL_ENV=$PWD/.venv .venv/bin/maturin develop --release   # from packages/python
```

`maturin build --release` writes an abi3 wheel for Python 3.10 and later. The
Rust crate is `crates/blasphem-python`, a standalone Cargo workspace. The data
package lives in `packages/python-packs`: run `python sync_packs.py`, then
`uv build`.
