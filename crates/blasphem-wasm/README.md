# Browser WASM

The default module embeds 15 toxicity packs and the ELDC language detector.

Build the module:

```bash
cargo build --release --locked --target wasm32-unknown-unknown -p toxcheck-wasm
wasm-bindgen target/wasm32-unknown-unknown/release/toxcheck_wasm.wasm \
  --target web \
  --out-dir target/wasm-web \
  --out-name toxcheck_wasm
```

Select one explicit language or use automatic routing:

```js
import init, { WasmDetector } from "./toxcheck_wasm.js";

await init();
const detector = new WasmDetector("AUTO");
const result = detector.check("أتمنى أن تموت وحيدًا هذه الليلة");

console.log(
  result.ok,
  result.score,
  result.shouldNudge,
  result.evaluated,
  result.resolvedLanguage,
  result.languageReliable,
  result.languageScore,
);
result.free();
detector.free();
```

The constructor accepts `EN,ZH,ES,AR,MS,PT,FR,HI,RU,JA,DE,TR,VI,KO,IT`.

The constructor accepts `ID` as an alias for `MS`.

Automatic routing returns `resolvedLanguage = "unknown"` for unreliable input.

An unknown route returns `evaluated = false` and does not run a toxicity detector.

Explicit and unknown routes return no `languageScore` value.

Build an explicit-only module without ELDC:

```bash
cargo build --release --locked --target wasm32-unknown-unknown \
  -p toxcheck-wasm --no-default-features
```

The explicit-only constructor returns an error for `AUTO`.

The explicit-only build omits ELDC.

The measured explicit-only transfer is 1,630,271 Brotli bytes, including JavaScript glue.

The measured default transfer is 9,038,194 Brotli bytes, including JavaScript glue.

Both current builds embed all 15 toxicity packs.

A future product should store one toxicity pack per language.

The product should load only the selected N packs.

AUTO should detect first and load only the resolved available pack.

A missing pack must return unknown and fail open.

The current shared ELDC table does not shrink in proportion to N.

All checks use embedded data. The browser runtime makes no network request after module initialization.

Run the real-browser smoke test:

```bash
./crates/toxcheck-wasm/verify-browser.sh
```

The script uses local Chrome.

It checks 49 default-module cases and one explicit-only module contract.

The explicit-only contract checks an English route and the exact `AUTO` feature error.

The two modules use separate WASM instances.

It writes `reports/multilingual-wasm.json`.
