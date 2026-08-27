# blasphem

Experimental multilingual pre-send toxicity nudge. Deterministic rules, HurtLex lexica, and one sparse integer table per language, compiled to WebAssembly. No AI runtime, and no network request after the module loads.

The package is isomorphic. The same entry runs in Node and in the browser.

This package is private and unpublished. Build it from the repository:

```bash
pnpm install --frozen-lockfile
pnpm --filter blasphem run build
```

The build writes `dist/`, holding the compiled client, the wasm-bindgen glue, and `blasphem_bg.wasm`.

## Use

```ts
import { judge } from "blasphem";

const v = judge("you are a stupid loser", {
  locales: ["en", "es"],
  detectLanguage: true,
  grawlix: true,
});

v.safe;    // false
v.score;   // 0.64
v.locale;  // "en"
v.grawlix; // "you are a @#$%&! loser"
```

Every option is optional. `judge("text")` works.

The module loads itself on import, so `judge` is synchronous. There is no `init()` to await, and nothing to `free()`.

## Options

| option | type | default | meaning |
| --- | --- | --- | --- |
| `locales` | `string[]` | all 15 | Lowercase locale codes to load. |
| `detectLanguage` | `boolean` | `true` | Route by detected language. |
| `grawlix` | `boolean` | `false` | Return the masked text. |

Accepted locales: `en`, `zh`, `es`, `ar`, `ms`, `pt`, `fr`, `hi`, `ru`, `ja`, `de`, `tr`, `vi`, `ko`, `it`. `id` is an alias for `ms`. Any other value throws.

With `detectLanguage: false` the judge scores every loaded locale and reports the highest.

## Result

| field | type | meaning |
| --- | --- | --- |
| `safe` | `boolean` | True when no nudge is due. |
| `score` | `number` | Ordinal, 0 through 1. Not a probability. |
| `locale` | `string \| null` | The locale that produced the score. |
| `grawlix` | `string \| null` | The masked text, when requested. |

`score` is ordinal. It ranks evidence. It is not calibrated across languages, and it is not a chance.

Text that no locale routes returns `locale: null`, `score: 0`, and `safe: true`. The nudge fails open.

## Cost

One judge is built per distinct option set and reused. The first call for a set loads its locales; later calls reuse them.

Measured on the default build: 9,041,755 Brotli bytes for the wasm and the glue, from `reports/multilingual-wasm.json`. The compiled client adds 908 Brotli bytes on top.

## Serving the module

The browser resolves `blasphem_bg.wasm` next to the glue. Keep `dist/` intact when you copy it, and serve the `.wasm` file as `application/wasm`.

To control loading yourself, await `load()` before the first `judge()` call. Importing the package already does this.

## Pinned tools

See `TOOLCHAIN.md`. The build stops when `wasm-bindgen --version` differs from the crate pin.
