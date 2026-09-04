# blasphem

A Rust library for toxicity warnings before a message is sent.

Blasphem hashes word and character n-grams into sparse feature vectors.
A linear classifier trained offline scores them with 16-bit weights.
Lexicons and context rules contribute to the verdict.
Detection runs locally without neural networks or cloud inference.

## Installation

This crate currently uses a Git dependency.
It is not published to crates.io.

```sh
cargo add --git https://github.com/sospedra/blasphem blasphem
```

For a local checkout, add a path dependency to `Cargo.toml`:

```toml
[dependencies]
blasphem = { path = "/path/to/blasphem/crates/blasphem" }
```

The crate declares Rust 1.85 or later.
Repository development uses the [pinned toolchain](../../rust-toolchain.toml).

## Usage

```rust
use blasphem::{Judge, JudgeOptions, Language};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let judge = Judge::new(JudgeOptions {
        locales: vec![Language::En, Language::Es],
        detect_language: true,
        grawlix: true,
    })?;

    let verdict = judge.judge("you are a stupid loser");
    println!("{verdict:?}");
    Ok(())
}
```

Create one `Judge` and reuse it.
`judge(&str)` returns a `Judgement` synchronously.
Rust releases its resources when the judge is dropped.

## Configuration

| `JudgeOptions` field | Default | Meaning |
| --- | --- | --- |
| `locales` | Empty vector | Load all supported model profiles |
| `detect_language` | `true` | Select the detected language |
| `grawlix` | `false` | Return text with matched spans masked |

With detection disabled, the judge returns the highest score across loaded locales.
Use `"id".parse::<Language>()` for Indonesian and `"ms".parse::<Language>()` for Malay.
See [all 16 supported languages](../../packages/javascript-packs/README.md#locales).

## Result

| Field | Type | Meaning |
| --- | --- | --- |
| `safe` | `bool` | No warning is due |
| `score` | `f64` | Ordinal value from 0 to 1 |
| `locale` | `Option<Language>` | Selected model profile |
| `grawlix` | `Option<String>` | Masked text when requested |

The score is not a probability.
Unrouted text returns `safe: true`, zero score, and no locale.
Construction returns `JudgeError` for invalid resources.

## Cargo features

| Feature | Default | Purpose |
| --- | --- | --- |
| `embedded` | Enabled | Embed language resources and enable `Judge::new` |
| `language-detection` | Enabled | Enable automatic language detection |

Embedded builds need no external pack files.
Without embedded data, use `Judge::from_packs` with `PackSource` values.
See [the API source](src/judge.rs) for resource and digest parameters.

## Documentation and development

Run from the repository root:

```sh
cargo doc --locked -p blasphem --no-deps --open
cargo test --locked -p blasphem
```

[Contribute](../../CONTRIBUTING.md) · [Architecture](../../HOW.md) · [CLI guide](../../packages/cli/README.md)

## License

Code uses [Apache-2.0](../../LICENSE).
Embedded data retains the terms recorded in [NOTICE](../../NOTICE).
