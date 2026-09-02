# ELDC automatic language design

## Goal

The detector shall accept an explicit language or detect one supported language automatically.

The runtime shall contain no C code, FFI, neural network, network request, or external process.

The runtime shall preserve the existing toxicity result contract.

## Supported languages

The automatic detector shall support these 15 canonical codes.

```text
EN ZH ES AR MS PT FR HI RU JA DE TR VI KO IT
```

`MS` shall identify the shared Malay and Indonesian product profile.

The explicit parser shall accept `ID` as a compatibility alias for `MS`.

Automatic detection shall return `MS` for Malay or Indonesian text.

The port shall contain no score profile for another ELDC language.

An unreliable result shall resolve to `unknown`.

A result without a supported score shall resolve to `unknown`.

## Upstream pin

The port shall use ELDC commit `a0301db809ff2e48a418018aa5359fb0c4354eb8`.

The imported files shall have fixed SHA-256 digests.

The generated model shall record its format version and upstream commit.

The repository shall record ELDC attribution and the Apache-2.0 license.

The C source shall remain outside the shipping dependency graph.

## Ported behavior

The Rust core shall preserve the upstream input limit of 1,000 UTF-8 bytes.

The Rust core shall preserve the upstream limit of 500 unique features.

The Rust core shall preserve ELDC byte-oriented word splitting.

The Rust core shall preserve ELDC CJK character features.

The Rust core shall preserve the exact Unicode bit tables and lowercase table.

The Rust core shall interpret packed features as little-endian integers.

The Rust core shall preserve wrapping 64-bit hash operations.

The Rust core shall preserve truncated `f32` weights and `f32` score accumulation.

The Rust core shall preserve stable score ordering for ties.

The Rust core shall preserve this normalized score formula.

```text
1 - exp(-0.0001 * raw_score / feature_count)
```

The Rust core shall require at least three extracted features for reliability.

The top score shall reach 85 percent of the selected language average.

The top score shall exceed the second score by more than 0.02.

## Subset model

The importer shall read the pinned ELDC generated database.

The importer shall retain only weights for the 15 supported profiles.

The importer shall map upstream `ms` weights to canonical `MS`.

The importer shall keep hash-slot fingerprints required for linear probing.

The importer shall remove all unsupported language weights from the score blob.

The importer shall write one versioned little-endian binary artifact.

The runtime shall validate the artifact header and every section boundary.

The runtime shall embed the artifact for native and browser builds.

## Packaging boundary

The ELDC crate shall remain separate from the toxicity detector crate.

The root crate shall expose language detection through an optional Cargo feature.

An explicit-only library build shall not link the ELDC artifact.

The full experimental CLI and browser build shall enable automatic detection by default.

Automatic routing shall expose a supported-language boundary for future partial language bundles.

A future product package may ship one toxicity resource pack per selected language.

An automatic result for an unavailable product pack shall resolve to `unknown`.

The current experiment may embed all 15 toxicity packs in its full browser bundle.

The current ELDC table remains shared across its 15 profiles.

Removing score profiles alone shall not claim a proportional ELDC size reduction.

The production browser package shall have three independent artifact classes.

The first artifact shall contain the shared toxicity evaluation code.

Each selected language shall have one toxicity data pack.

The optional automatic artifact shall contain ELDC code and data.

The toxicity data pack shall own the sparse model, HurtLex data, and language-specific rule data.

A package manifest shall record each canonical language, artifact digest, and artifact size.

An explicit browser session shall load only its requested toxicity data packs.

An automatic browser session shall load ELDC before it selects a toxicity data pack.

The loader shall restrict automatic results to the configured toxicity pack set.

An unavailable automatic result shall remain unknown and shall fail open.

The product build may bundle the selected artifacts or load them as separate cached files.

The current experiment does not implement the external toxicity pack format.

The product API shall accept a `SupportedLanguageSet`.

A `ToxicityPackProvider` shall return one validated pack for one concrete language.

The shared WASM code shall not embed a sparse model, HurtLex data, or language rule data.

The product shall replace static model and rule references with owned or shared pack storage.

The browser loader may fetch pack files during asynchronous initialization.

The synchronous `check` operation shall make no network request.

The browser loader shall cache one parsed ELDC instance across automatic detectors.

Cargo language features may create fixed builds, but they shall not be the only N-language mechanism.

## Public Rust API

`Language` shall remain a concrete supported language.

`LanguageSelection` shall represent `Explicit(Language)` or `Auto`.

`LanguageResolution` shall represent `Known(Language)` or `Unknown`.

`LanguageDetector` shall return the resolution, reliability, score, and feature count.

Explicit selection shall bypass `LanguageDetector`.

Automatic selection shall evaluate toxicity only after a known resolution.

The toxicity score shall remain separate from the language score.

## CLI contract

The CLI shall accept `--language AUTO` case-insensitively.

The CLI shall accept all 15 canonical explicit codes.

The CLI shall accept `ID` as an alias for explicit `MS`.

The existing first result line shall remain unchanged for evaluated text.

Automatic mode shall print one stable routing line.

```text
language_mode=auto route=known detected_language=ES reliable=true language_score=0.9123
```

Unknown text shall fail open.

Unknown text shall print `ok=true score=0 threshold=50 should_nudge=false`.

Unknown text shall print `route=unknown` and `evaluated=false`.

Explicit mode shall print the canonical selected language in its routing line.

## Browser contract

The existing explicit constructor shall remain valid.

The browser constructor shall accept `AUTO`.

The browser result shall retain `ok`, `score`, `threshold`, and `shouldNudge`.

The browser result shall add `evaluated`, `resolvedLanguage`, `languageReliable`, and `languageScore`.

Unknown text shall return `ok=true`, `score=0`, and `shouldNudge=false`.

The browser runtime shall make no network request.

An explicit-only WASM build shall compile without the automatic language detector feature.

## Verification

The C oracle shall use the same 15-language ELDC subset configuration.

The parity suite shall compare the language, reliability, order, score, and feature count.

The normalized score error shall not exceed `0.000001`.

The parity suite shall cover the upstream tests and representative benchmark rows.

The parity suite shall cover empty, short, mixed-script, CJK, malformed-boundary, and 1,000-byte inputs.

Every existing explicit toxicity decision shall remain unchanged.

Native and browser automatic results shall match.

The final report shall record native size, browser size, cold time, and warm latency.

The final report shall state that unsupported-language rejection is best-effort with a 15-profile model.

The final verification shall include an explicit-only build without the ELDC feature.
