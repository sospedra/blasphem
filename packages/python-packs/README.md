# blasphem-packs (Python)

The blasphem packs as a data wheel. `blasphem` reads them when `init` gets no
`assets` directory.

```bash
pip install blasphem-packs
```

`blasphem_packs.directory()` returns the path holding `manifest.json` and the
`.pack` and `.detect` files, for a caller that wants to pass `assets` itself.

## Build

```bash
python sync_packs.py      # copies packages/packs/dist into blasphem_packs/
uv build                  # writes the wheel
```

License: the lexica are HurtLex 1.2 under CC BY-NC-SA 4.0. See
`packages/packs/NOTICE`.
