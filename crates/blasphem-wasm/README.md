# blasphem-wasm

Browser bindings for the multilingual nudge detector. The wasm carries the
code only. Language data arrives at run time as `.pack` and `.detect` files
from `@blasphem/packs`.

## Classes

`BlasphemEngineBuilder(detectLanguage, grawlix)` collects locales:

```js
const builder = new BlasphemEngineBuilder(true, false);
builder.add("en", enPackBytes, enPackSha256, enDetectBytes, enDetectSha256);
const engine = builder.build(); // consumes the builder
engine.locales;                 // ["en"]
engine.judge("text");           // { safe, score, locale, grawlix }
engine.free();
```

`add` takes `Uint8Array` buffers and optional 64-character hexadecimal digests.
Rust verifies each digest before it parses the bytes.

Every error is a string that starts with a contract code:
`BLASPHEM_LOCALE_UNSUPPORTED`, `BLASPHEM_DIGEST_MISMATCH`,
`BLASPHEM_FORMAT_VERSION`, `BLASPHEM_PACK_INVALID`, `BLASPHEM_LOCALES_EMPTY`.

## Build

`packages/blasphem/scripts/build.mjs` compiles this crate for
`wasm32-unknown-unknown` with `default-features = false` on `blasphem`, so no
artifact is embedded, then runs `wasm-bindgen --target web
--omit-default-module-path`. The JavaScript loader passes the wasm location
explicitly; the glue never resolves it from `import.meta.url`.

## Test

`cargo test -p blasphem-wasm` builds packs from the committed artifacts and
exercises `blasphem::Engine`, the type both classes wrap.
