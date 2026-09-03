# Blasphem

Blasphem is a deterministic pre-send toxicity nudge. It checks a message before send, across 15 languages. It runs no neural model and no AI runtime.

It ships a Rust library, a command-line binary, a browser and Node package, a React Native package, a Python extension, and a Go module. Every one answers the same judge contract. The evidence status is experimental.

## Install

| Runtime | Command |
| --- | --- |
| Command line | `curl --proto '=https' --tlsv1.2 -LsSf https://github.com/sospedra/blasphem/releases/latest/download/blasphem-installer.sh \| sh` |
| Node, browser | `pnpm add blasphem @blasphem/packs` |
| React Native | `pnpm add @blasphem/react-native @blasphem/packs` |
| Python | `pip install blasphem blasphem-packs` |
| Go | `go get github.com/sospedra/blasphem/packages/go` |
| Rust | `cargo add --git https://github.com/sospedra/blasphem blasphem` |

The command-line binary embeds all fifteen languages. Every other runtime loads language data from the packs, so both names go together.

## Command line

Install one of three ways:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/sospedra/blasphem/releases/latest/download/blasphem-installer.sh | sh
cargo install --git https://github.com/sospedra/blasphem blasphem
npx blasphem judge "you are a stupid loser"
```

Judge one message, or one message per stdin line:

```bash
blasphem judge "you are a stupid loser"
blasphem judge --json --locales en,es --grawlix "you are a stupid loser"
printf 'hello there\nTe voy a matar\n' | blasphem judge
```

```
safe=false score=0.95 locale=en
{"safe":false,"score":0.95,"locale":"en","grawlix":"you are a @#$%&! loser"}
safe=true score=0 locale=none
safe=false score=0.95 locale=es
```

The exit code is 0 when nothing nudged, 1 when a verdict nudged, and 2 on an error. `--locales` limits the loaded languages. `--no-detect` scores every loaded locale and reports the highest. The binary embeds all fifteen locales and needs no data files.

Build it from a checkout:

```bash
cargo build --release --locked --bin blasphem
```

See `CONTRIBUTING.md` to add training data, `LICENSE` for the first-party license, and `NOTICE` for third-party data licenses.

The runtime uses a clean-room lexicon built from Wiktionary and other license-clean sources, fixed dictionaries, deterministic context rules, and sparse integer tables. See `docs/clean-room-lexicon-report.md` and `NOTICE`.

Each language uses one independent 128 KiB sparse integer table. Offline labeled corpora produced these tables.

The runtime does not load TextDetox rows. It does not need Python, ONNX, TensorFlow, or a network connection.

## Rust

```rust
use blasphem::{Judge, JudgeOptions, Language};

let judge = Judge::new(JudgeOptions {
    locales: vec![Language::En, Language::Es],
    detect_language: true,
    grawlix: true,
})?;

let verdict = judge.judge("you are a stupid loser");

assert!(!verdict.safe);
assert_eq!(verdict.score, 0.95);
assert_eq!(verdict.locale, Some(Language::En));
assert_eq!(verdict.grawlix.as_deref(), Some("you are a @#$%&! loser"));
```

`JudgeOptions::default()` loads all 15 languages, detects the language, and returns no grawlix.

`score` is an ordinal value from 0.0 through 1.0. It is not a probability, and it is not calibrated across languages.

Text that no loaded locale routes returns `locale: None`, `score: 0.0`, and `safe: true`. The nudge fails open.

`Judge` is a struct rather than a free function because each locale carries its own lexicon and sparse table. Build one judge and reuse it.

The lexica are compiled into the binary. `blasphem::embedded_detector` exposes the same data for a single language.

## TypeScript

```bash
pnpm add blasphem @blasphem/packs
```

```ts
import { init, judge } from "blasphem";

await init({ locales: ["en", "es"], grawlix: true });

const v = judge("you are a stupid loser");

v.safe;  // false
v.score; // 0.95
```

`init` loads the locales once and installs one judge for the module. `judge` is synchronous, so it runs on every keystroke. Before `init` resolves it returns the fail-open verdict and never throws. `createJudge(options)` returns an independent judge instead.

The package is isomorphic and runs the same in Node and the browser. Node runs the native binary when its platform package is installed and the wasm otherwise. See `packages/blasphem/README.md`.

## React Native

```bash
pnpm add @blasphem/react-native @blasphem/packs
```

```ts
import { init, judge } from "@blasphem/react-native";

await init({ locales: ["en", "es"], grawlix: true });

const v = judge("you are a stupid loser");

v.safe;  // false
v.score; // 0.95
```

The engine is the Rust core behind a C ABI on Nitro Modules, so `judge()` is synchronous over JSI. Packs come from the application bundle rather than from `assets`. See `packages/react-native/README.md`.

## Swift

```swift
// Package.swift: https://github.com/sospedra/blasphem-swift, products Blasphem, BlasphemPackEN, BlasphemPackES, BlasphemDetectEN, BlasphemDetectES
import Blasphem

let judge = try Judge(locales: ["en", "es"], grawlix: true)
try judge.judge("you are a stupid loser")
// Judgement(safe: false, score: 0.95, locale: "en", grawlix: "you are a @#$%&! loser")
```

Swift Package Manager, iOS 15.1 and macOS 12. The engine is the Rust core in an XCFramework; each `BlasphemPack<CODE>` and `BlasphemDetect<CODE>` product carries one data file, so the app ships only the locales it links. See `packages/swift/README.md`.

## Android

```kotlin
// Gradle: platform("me.sospedra.blasphem:blasphem-bom:0.1.0"), blasphem, blasphem-pack-en, blasphem-pack-es, blasphem-detect-en, blasphem-detect-es
import me.sospedra.blasphem.Judge
import me.sospedra.blasphem.JudgeOptions

val judge = Judge.create(context, JudgeOptions(locales = listOf("en", "es"), grawlix = true))
judge.judge("you are a stupid loser")
// Judgement(safe=false, score=0.95, locale=en, grawlix=you are a @#$%&! loser)
```

Maven Central, `minSdk 24`. The engine is the Rust core behind JNI; each `blasphem-pack-<code>` and `blasphem-detect-<code>` artifact carries one asset, so the app ships only the locales it adds. See `packages/android/README.md`.

## Python

```bash
pip install blasphem blasphem-packs
```

```python
import blasphem

blasphem.init(["en", "es"], grawlix=True)
blasphem.judge("you are a stupid loser")
# Judgement(safe=False, score=0.95, locale='en', grawlix='you are a @#$%&! loser')
```

A PyO3 extension over the same Rust core. The wheel is abi3 and covers Python 3.10 and later. `blasphem-packs` supplies the language data when `init` gets no `assets`. See `packages/python/README.md`.

## Go

```bash
go get github.com/sospedra/blasphem/packages/go
```

```go
import blasphem "github.com/sospedra/blasphem/packages/go"

err := blasphem.Init(blasphem.Options{Locales: []string{"en", "es"}, Assets: "/srv/blasphem-packs", Grawlix: true})
verdict := blasphem.Judge("you are a stupid loser")
// {Safe:false Score:0.95 Locale:en Grawlix:you are a @#$%&! loser}
```

The core compiles to WebAssembly and is embedded in the module; wazero runs it. No cgo, and `CGO_ENABLED=0` builds work. See `packages/go/README.md`.

## Reproduce every artifact

The canonical command runs every check:

```bash
cargo run --release --locked -p blasphem-train -- reproduce
```

It verifies raw inputs, rebuilds every model and language artifact, builds
the native and WASM binaries, runs the Rust tests and Clippy, then runs the
package checks through `pnpm`: install, build, pack check, the Node smoke,
and the browser smoke on Playwright Chromium and WebKit. Install the pinned
browsers once:

```bash
pnpm --filter blasphem exec playwright install chromium webkit
```

Without Node, pnpm, or the browsers, skip the JavaScript checks:

```bash
cargo run --release --locked -p blasphem-train -- reproduce --skip-browser
```

Both forms read only committed inputs. Neither downloads a corpus, lexicon,
or model source. Both write generated data to a temporary directory and
return a nonzero status after any mismatch.

## Licensing

Blasphem first-party code uses the Apache License 2.0. See `LICENSE`.

The corpora and lexica in this repository keep their own recorded licenses,
including one unresolved license and two share-alike licenses. See `NOTICE`
for the full accounting.

## Setup

`setup` downloads HurtLex from upstream for ad-hoc reference and comparison only. It plays no part in building the shipped detector, which reads `data/clean-room-v1` instead. See `docs/clean-room-lexicon-report.md`.

```bash
cargo run --release -p blasphem-train -- setup
```

The shipping detector supports `EN,ZH,ES,AR,MS,PT,FR,HI,RU,JA,DE,TR,VI,KO,IT`.

The parser accepts `ID` as an alias for `MS`.

Download every HurtLex 1.2 language, for comparison only:

```bash
cargo run --release -p blasphem-train -- setup --languages all
```

The runtime uses conservative clean-room lexicon entries only.

## Policy checks

The `check` command is the diagnostic behind the evidence reports. It is hidden from `--help` and reads a HurtLex-formatted lexicon from `--data-dir` (default `data/raw-v1/hurtlex`, now removed). Point it at a nested mirror of `data/clean-room-v1` instead; see "Bridging the flat and nested lexicon layouts" below.

Run these policy checks:

```bash
cargo run --release -p blasphem --bin blasphem -- check --language EN --text "You are an idiot"
cargo run --release -p blasphem --bin blasphem -- check --language EN --text "I will kill you"
cargo run --release -p blasphem --bin blasphem -- check --language EN --text '"You are an idiot," she said'
cargo run --release -p blasphem --bin blasphem -- check --language ES --text "Te voy a matar"
cargo run --release -p blasphem --bin blasphem -- check --language ES --text "ojala se muera toda tu familia"
cargo run --release -p blasphem --bin blasphem -- check --language ES --text "No te voy a matar"
```

The output has five category values: `profanity`, `targeted_abuse`, `identity_attack`, `threat_language`, and `sentiment_support`.

The first line contains `ok`, `score`, `threshold`, and `should_nudge`.

Use `ok` or `should_nudge` for the pre-send message. The integer score ranges from 0 through 100.

The integer score is ordinal. It is not a probability or a confidence score.

The `lexical_score` field is the legacy lexical score from 0.0 through 1.0.

The `sparse_score` line contains the selected language table score.

Sentiment only modifies active lexical or context evidence. Sentiment alone cannot select `review` or `block`.

The policy layer excludes 64 verified HurtLex collisions by language and normalized lemma.

The exclusion table covers AR, DE, EN, ES, IT, JA, and RU.

An excluded term remains in the raw lexical matches. The policy output adds `lexical_collision_excluded` evidence with zero points.

An exclusion does not reduce the legacy lexical `score` field.

Lexical rule evidence includes the candidate view, normalized offsets, and raw UTF-8 byte offsets.

The selected language applies only its language-specific exclusions.

Explicit German reactivates `Hund` only for the exact direct-abuse phrase `du hund`.

The runtime does not derive the exclusion table from TextDetox labels.

The CLI has context packs for all 15 supported languages.

Reply context can supply a target:

```bash
cargo run --release -p blasphem --bin blasphem -- check --language EN --reply-target person --text "Idiot"
```

Use `protected-group` for a protected-group reply target. The caller must supply all reply-target context.

Use `--language AUTO` to detect a supported language before the toxicity check:

```bash
cargo run --release --bin blasphem -- check --language AUTO --text "Cuando te vea, te rompo los dientes"
```

AUTO returns an unknown route for unreliable input.

Unsupported-language rejection is best-effort with this 15-profile model.

Unsupported input can misroute to a supported language.

An unknown route sets `ok=true`, `score=0`, and `evaluated=false`.

Use an explicit code to bypass the language detector.

Spanish uses POS-aware plural expansion for high-confidence noun and adjective forms.

Spanish also detects directed hostility, harm wishes, self-harm commands, threats, and direct group hostility.

Negation, quotes, reports, and counterspeech can suppress these semantic events.

## Spanish checks

Run the fixed Spanish behavior panel:

```bash
cargo run --release -p blasphem-train -- eval \
  --input samples/spanish-audit.tsv \
  --minimum-action review
```

The recorded Spanish test result is `previously_evaluated` experimental evidence.

Do not rerun the Spanish test split during calibration or behavior work.

The behavior panel is a contract set. Do not treat its result as an unbiased production estimate.

## TextDetox data

[TextDetox](https://huggingface.co/datasets/textdetox/multilingual_toxicity_dataset) supports offline sparse-table compilation and evaluation.

The offline pipeline downloads one pinned Parquet file for each catalog TextDetox language.

The pipeline parses each Parquet file locally and publishes the same canonical acquisition TSV.

The acquisition TSV has this header:

```text
source_id<TAB>language<TAB>toxic<TAB>text
```

The accepted TextDetox source codes are:

```text
am ar de en es fr he hi hin it ja ru tt uk zh
```

`hin` means Hinglish and maps to `HINGLISH`. `hi` means Hindi and maps to `HI`.

Acquisition records the pinned Hugging Face dataset revision in each source ID.

Acquisition rejects a URL that does not contain the pinned revision.

The source record stores separate SHA-256 values for the Parquet download and the canonical TSV.

The catalog TextDetox set is EN, ZH, AR, FR, HI, RU, JA, DE, IT, and ES.

Hash buckets 0 through 69 select development. Buckets 70 through 84 select validation. Buckets 85 through 99 select test.

The hash split is deterministic and is not stratified. Realized split counts can differ from 70/15/15.

Exact normalized duplicates within one detector language stay in one group. Label-conflict groups do not enter evaluation.

`corpus/` holds one committed TSV file per language. That file is the single source of truth for its rows. See `corpus/README.md`.

`resources/datasets/behavior-provenance-v1.tsv` records the 55 audit-only rows the behavior panels cite. Those rows stay out of the corpus.

Paraphrases, copied templates, and cross-language copies can still cross splits.

Do not tune rules or thresholds on the test split.

Compile the model set from development, validation, and frozen clean behavior controls:

```bash
cargo run --release --locked -p blasphem-train -- compile \
  --corpus-root corpus \
  --source-lock resources/datasets/source-lock-v1.json \
  --hurtlex-root data/clean-room-v1 \
  --behavior-root tests/fixtures/behavior \
  --output resources/models/multilingual-v2
```

`--hurtlex-root` is a legacy name. It now points at the clean-room lexica, and `compile` reads them directly: it resolves `{hurtlex-root}/{CODE}.tsv`, which is exactly the flat layout `data/clean-room-v1` already uses.

Most profiles use word unigrams, word bigrams, and character 3-grams through 5-grams.

Chinese uses Han unigrams and character 2-grams through 5-grams. Han, Latin, and mixed-script grams use separate hash namespaces.

Turkish uses character 3-grams through 5-grams within each token. Its compiler uses class-weighted L2 logistic training.

The compiler hashes features into 65,536 bins. It writes fixed integer weights and a validation boundary.

The clean controls set a minimum boundary. They do not enter the accuracy metrics.

The checked-in manifest is `resources/models/multilingual-v2/manifest.json`.

### Bridging the flat and nested lexicon layouts

`compile` reads the clean-room lexica flat, as `{root}/{CODE}.tsv`. `evaluate`, `behavior`, `cli-smoke`, and the `check` diagnostic below instead still expect the historical HurtLex layout, `{root}/{CODE}/1.2/hurtlex_{CODE}.tsv`. The two look interchangeable and are not; passing `data/clean-room-v1` straight to `evaluate` fails. Build a one-time nested mirror first:

```bash
for f in data/clean-room-v1/??.tsv; do
  code=$(basename "$f" .tsv)
  mkdir -p "/tmp/hurtlex-bridge/$code/1.2"
  cp "$f" "/tmp/hurtlex-bridge/$code/1.2/hurtlex_$code.tsv"
done
```

Then pass `--hurtlex-root /tmp/hurtlex-bridge` (or `--data-dir /tmp/hurtlex-bridge` for `check`) to those four commands, in place of any `data/raw-v1/hurtlex` shown below.

## Pre-test evidence

Write the 15-language validation evidence:

```bash
cargo run --release --locked -p blasphem-train -- evaluate \
  --split validation \
  --corpus-root corpus \
  --model-manifest resources/models/multilingual-v2/manifest.json \
  --hurtlex-root /tmp/hurtlex-bridge \
  --output reports/multilingual-validation.json
```

This report is calibration evidence. It is not an independent test estimate.

The report records each language matrix, precision, recall, specificity, F1, and false warning rate.

It also projects precision at one-percent and five-percent toxic message prevalence.

Write the 360-case behavior contract evidence:

```bash
cargo run --release --locked -p blasphem-train -- behavior \
  --fixture-root tests/fixtures/behavior \
  --corpus-root corpus \
  --model-manifest resources/models/multilingual-v2/manifest.json \
  --hurtlex-root /tmp/hurtlex-bridge \
  --output reports/multilingual-behavior.json
```

The behavior panels are authored contract evidence. They are not a production accuracy sample.

Write the 60-case native smoke evidence:

```bash
cargo run --release --locked -p blasphem-train -- cli-smoke \
  --model-manifest resources/models/multilingual-v2/manifest.json \
  --hurtlex-root /tmp/hurtlex-bridge \
  --output reports/multilingual-cli-smoke.json
```

The smoke report covers all 15 languages. It records two supplied cases and two context cases per language.

These commands call the shipping `NudgeDetector::check` path. They do not open a sealed test split.

The legacy `eval` command accepts manual audit TSV files. Do not use it for a frozen test split.

The audit TSV header is `language<TAB>label<TAB>text`. A label is `toxic` or `clean`.

The evaluator reports overall and per-language confusion matrices. It also reports accuracy, precision, recall, specificity, and F1.

TextDetox source labels have different definitions. Binary labels cannot measure category accuracy.

The source corpora use different label rules. They do not represent production prevalence.

Evaluation errors can identify manual audit candidates. Runtime code never generates exclusions from TextDetox labels.

Any row that influences a rule becomes audit-only. Do not report that row in later quality metrics.

## Offline dataset workflow

These commands import a new upstream source. The reproduction path does not run
them: it reads `corpus/` directly. This repository ships no raw dataset files, so
supply your own before running `acquire` or `prepare`.

Observe the catalog before the source review:

```bash
cargo run --release --locked -p blasphem-train -- observe \
  --source-catalog resources/datasets/source-catalog-v1.json \
  --output data/source-observation-v1
```

Review `data/source-observation-v1/source-observation-v1.json` and every recorded license before freezing.

```bash
cargo run --release --locked -p blasphem-train -- freeze-sources \
  --observation data/source-observation-v1/source-observation-v1.json \
  --reviewed \
  --output resources/datasets/source-lock-v1.json

cargo run --release --locked -p blasphem-train -- acquire \
  --source-lock resources/datasets/source-lock-v1.json \
  --output data/raw-v1

cargo run --release --locked -p blasphem-train -- prepare \
  --source-lock resources/datasets/source-lock-v1.json \
  --raw-root data/raw-v1 \
  --output data/prepared-draft-v1
```

Merge the prepared rows into `corpus/{LANGUAGE}.tsv`, then run `corpus-verify`.

The commands refuse an existing output. The acquire command verifies each frozen identity and SHA-256 digest before publication.

## Limits

The clean-room lexicon contains ambiguous entries, same as any lexicon of this kind. The conservative set can still produce false positives.

The fixed exclusion table can miss other abusive uses of dual-use words.

Identity attacks require a group target plus a direct relation, net negative lexical support, or protected-group reply context.

Only Russian and Arabic accept whitespace-only zero-copula identity syntax.

The fixed dictionaries have unequal language coverage.

The detector can miss new words, unsupported morphology, code-switching, and unsupported evasions.

Quote, report, negation, and counterspeech suppression use short rule windows.

All suppressed lexical spans add at most ten profanity points for one message.

General stemming remains disabled. Spanish has a small POS-aware plural rule set.

CJK detection uses compact character rules and character n-grams.

The runtime does not need the large Japanese and Korean tokenizer dictionaries.

The `blasphem-train setup`, `observe`, and `acquire` commands use the network.

HurtLex setup reads an unpinned `master` URL. Retain downloaded files for repeatable runs.

## Performance gate

Run the release-only dense-message latency gate:

```bash
cargo perf-gate
```

The gate checks 30 dense fixtures. It requires one short and one long fixture for each supported language.

## Browser WASM

Most applications should use the `blasphem` npm package, which wraps this crate and loads the packs. This section calls `crates/blasphem-wasm` directly.

The module carries code only. Language data arrives at run time as one `.pack` and one `.detect` file per locale from `@blasphem/packs`.

Build the module and generate the web bindings:

```bash
cargo build --release --locked --target wasm32-unknown-unknown -p blasphem-wasm
wasm-bindgen target/wasm32-unknown-unknown/release/blasphem_wasm.wasm \
  --target web \
  --omit-default-module-path \
  --out-dir target/wasm-web \
  --out-name blasphem
```

`--omit-default-module-path` drops the `new URL(..., import.meta.url)` fallback, so the caller names the wasm location and no bundler sees an implicit asset.

Add one locale at a time, then build the engine:

```js
import init, { BlasphemEngineBuilder } from "./blasphem.js";

await init({ module_or_path: "./blasphem_bg.wasm" });

const builder = new BlasphemEngineBuilder(true, false); // detectLanguage, grawlix
builder.add("en", enPackBytes, enPackSha256, enDetectBytes, enDetectSha256);

const engine = builder.build();         // consumes the builder
engine.locales;                         // ["en"]
engine.judge("you are a stupid loser"); // { safe, score, locale, grawlix }
engine.free();
```

`add` takes `Uint8Array` buffers and optional 64-character hexadecimal digests. Rust verifies each digest before it parses the bytes. Every error is a string that starts with a contract code. See `crates/blasphem-wasm/README.md`.

Build without the language detector:

```bash
cargo build --release --locked --target wasm32-unknown-unknown \
  -p blasphem-wasm --no-default-features
```

The detector's tables ship as `.detect` files, not inside the module, so the feature costs 9,093 Brotli bytes of code. `reports/browser-smoke.json` records what a page downloads. Brotli bytes, glue included:

| Download | Brotli |
| --- | --- |
| module and glue, no packs | 302,348 |
| plus `en.pack`, detection off | 404,822 |
| plus `en.pack` and `en.detect` | 681,746 |
| plus all 15 locales | 5,723,839 |

A judge fetches only the locales it was asked for. A locale absent from `manifest.json` throws at construction. Text that no loaded locale routes returns `safe: true` and fails open.

See the automatic-language-detection report under `docs/` for route, latency, size, and browser evidence.

Run the real-browser smoke test on Playwright Chromium and WebKit:

```bash
pnpm --filter blasphem run build
pnpm --filter blasphem run test:browser
```

The test drives `dist/browser.js` in both engines, asserts that an English-only judge downloads exactly `manifest.json`, `en.pack`, and `en.detect`, and writes experimental browser and compressed-size evidence to `reports/browser-smoke.json`.
