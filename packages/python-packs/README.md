# blasphem-packs (Python)

The blasphem packs as a data wheel. `blasphem` reads them when `init` gets no
`assets` directory.

```bash
python sync_packs.py      # copies packages/packs/dist into blasphem_packs/
uv build                  # writes the wheel
```

License: the lexica are HurtLex 1.2 under CC BY-NC-SA 4.0. See
`packages/packs/NOTICE`.
