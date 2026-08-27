# Browser WASM

The default module embeds 15 toxicity packs and the language detector.

Build the module:

```bash
cargo build --release --locked --target wasm32-unknown-unknown -p blasphem-wasm
wasm-bindgen target/wasm32-unknown-unknown/release/blasphem_wasm.wasm \
  --target web \
  --out-dir target/wasm-web \
  --out-name blasphem
```

Select one explicit language or use automatic routing:

```js
import init, { BlasphemDetector } from "./blasphem.js";

await init();
const detector = new BlasphemDetector("AUTO");
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

## The judge binding

`BlasphemJudge` returns a plain object. The caller never frees it.

```js
import init, { BlasphemJudge } from "./blasphem_wasm.js";

await init();
const judge = new BlasphemJudge(["en", "es"], true, true);
const verdict = judge.judge("you are a stupid loser");

console.log(verdict.safe, verdict.score, verdict.locale, verdict.grawlix);
// false 0.64 "en" "you are a @#$%&! loser"
```

The constructor takes lowercase locale codes, a detect-language flag, and a grawlix flag.

An empty locale list loads all 15 languages.

`score` is an ordinal value from 0.0 through 1.0. It is not a probability.

`safe` is true when no nudge is due. Text that no locale routes returns `locale: null` and `safe: true`.

`grawlix` holds the masked text when the flag is set. It is `null` otherwise.

Prefer the `blasphem` npm package over this binding. It wraps `BlasphemJudge` and loads the module for you.

Build an explicit-only module without the language detector:

```bash
cargo build --release --locked --target wasm32-unknown-unknown \
  -p blasphem-wasm --no-default-features
```

The explicit-only constructor returns an error for `AUTO`.

The explicit-only build omits the language detector.

The measured explicit-only transfer is 1,633,893 Brotli bytes, including JavaScript glue.

The measured default transfer is 9,041,755 Brotli bytes, including JavaScript glue.

Both figures come from `reports/multilingual-wasm.json`, written by `./crates/blasphem-wasm/verify-browser.sh`.

Both current builds embed all 15 toxicity packs.

A future product should store one toxicity pack per language.

The product should load only the selected N packs.

AUTO should detect first and load only the resolved available pack.

A missing pack must return unknown and fail open.

The current shared language detector table does not shrink in proportion to N.

All checks use embedded data. The browser runtime makes no network request after module initialization.

Run the real-browser smoke test:

```bash
./crates/blasphem-wasm/verify-browser.sh
```

The script uses local Chrome.

It checks 49 default-module cases and one explicit-only module contract.

The explicit-only contract checks an English route and the exact `AUTO` feature error.

The two modules use separate WASM instances.

It writes `reports/multilingual-wasm.json`.
