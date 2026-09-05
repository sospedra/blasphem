# blasphem-wasm

Low-level WebAssembly bindings for the Rust toxicity engine.
The module contains code, without embedded language data.
Most applications should use [the JavaScript package](../../packages/javascript/README.md).

Blasphem hashes word and character n-grams into sparse feature vectors.
A linear classifier trained offline scores them with 16-bit weights.
Lexicons and context rules contribute to the verdict.
Detection runs locally without neural networks or cloud inference.

## Build

Install the [repository toolchain](../../CONTRIBUTING.md#set-up).
Run from the repository root:

```sh
cargo build --release --locked --target wasm32-unknown-unknown -p blasphem-wasm
wasm-bindgen target/wasm32-unknown-unknown/release/blasphem_wasm.wasm \
  --target web \
  --omit-default-module-path \
  --out-dir target/wasm-web \
  --out-name blasphem
```

The CLI version must match the crate's `wasm-bindgen` pin.
The generated loader requires an explicit module location.

## Usage

Serve the generated `blasphem.js` and `blasphem_bg.wasm` files.
Serve matching [language packs](../../packages/javascript-packs/README.md) under `/blasphem`.

```js
import init, { BlasphemEngineBuilder } from "./blasphem.js";

async function readBytes(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`HTTP ${response.status}: ${url}`);
  return new Uint8Array(await response.arrayBuffer());
}

await init({ module_or_path: "./blasphem_bg.wasm" });

const manifestResponse = await fetch("/blasphem/manifest.json");
if (!manifestResponse.ok) throw new Error("Could not load the pack manifest");
const manifest = await manifestResponse.json();
const [pack, detect] = await Promise.all([
  readBytes("/blasphem/en.pack"),
  readBytes("/blasphem/en.detect"),
]);

const builder = new BlasphemEngineBuilder(true, false);
builder.add(
  "en",
  pack,
  manifest.files["en.pack"].sha256,
  detect,
  manifest.files["en.detect"].sha256,
);

const engine = builder.build();
try {
  console.log(engine.locales);
  console.log(engine.judge("you are a stupid loser"));
} finally {
  engine.free();
}
```

Serve WASM with the `application/wasm` content type.
Apply the [browser CSP requirements](../../packages/javascript/README.md#content-security-policy).

## API

| Call | Purpose |
| --- | --- |
| `new BlasphemEngineBuilder(detectLanguage, grawlix)` | Create a locale builder |
| `builder.add(locale, pack, packSha256, detect, detectSha256)` | Supply one locale's files |
| `builder.build()` | Consume the builder and return an engine |
| `engine.locales` | Read the loaded model codes |
| `engine.judge(text)` | Return a plain verdict object |
| `engine.free()` | Release the engine |

`add` accepts `Uint8Array` data and optional hexadecimal SHA-256 digests.
`build` verifies supplied digests and parses the packs.
Detection-disabled builders do not need detection slices.
Do not use a builder after `build` or an engine after `free`.

Results contain `safe`, `score`, `locale`, and `grawlix`.
`grawlix` contains masked text for unsafe verdicts when requested, otherwise `null`.
The generated `Judgement` type narrows `grawlix` to `null` when `safe` is `true`.
The score is ordinal, between 0 and 1.
Errors are strings prefixed with `BLASPHEM_*` contract codes.
See [the bindings](src/lib.rs) and [engine](../blasphem/src/engine.rs).

## Features

`language-detection` is enabled by default.
To remove detection support:

```sh
cargo build --release --locked --target wasm32-unknown-unknown \
  -p blasphem-wasm --no-default-features
```

Use `detectLanguage = false` with that build.
Language data remains external with either configuration.

## Development

Run from the repository root:

```sh
cargo test --locked -p blasphem-wasm
```

These Rust tests exercise the shared engine.
The [JavaScript browser checks](../../packages/javascript/README.md#build-from-source) exercise browser execution.

[Contribute](../../CONTRIBUTING.md)

## License

Code uses [Apache-2.0](../../LICENSE).
Language data retains the terms recorded in [NOTICE](../../NOTICE).
