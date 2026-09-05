# † Blasphem

Blasphem checks text for toxicity before someone sends it.
It provides a warning for [16 languages](packages/javascript-packs/README.md#locales).
Use `id` for Indonesian and `ms` for Malay.

Blasphem hashes word and character n-grams into sparse feature vectors.
A linear classifier trained offline scores them with 16-bit weights.
Lexicons and context rules contribute to the verdict.
Detection runs locally without neural networks or cloud inference.

## How it works

1. Blasphem normalizes the text and identifies its language.
2. Dictionaries, context rules, and small integer weight tables score the message.
3. Blasphem returns a verdict and optional masked text.

Read [how Blasphem works](HOW.md) for the architecture.

## Distributions

| Platform | Installation and usage |
| --- | --- |
| Command line | [CLI guide](packages/cli/README.md) |
| Rust | [Rust crate](crates/blasphem/README.md) |
| JavaScript and TypeScript | [Node and browser package](packages/javascript/README.md) |
| React Native | [React Native package](packages/react-native/README.md) |
| Swift | [Swift package](packages/apple/README.md) |
| Android | [Kotlin library](packages/android/README.md) |
| Python | [Python package](packages/python/README.md) |
| Go | [Go module](packages/go/README.md) |

[Language packs](packages/javascript-packs/README.md) · [Python data wheel](packages/python-packs/README.md) · [WASM bindings](crates/blasphem-wasm/README.md)

## Contribute

| Contribution | Start here |
| --- | --- |
| Code, bindings, or documentation | [Development guide](CONTRIBUTING.md#code-and-documentation) |
| Labeled messages in `resources/corpus/` | [Corpus contribution guide](resources/corpus/README.md#contribute) |
| Words and categories in `resources/lexicon/` | [Lexicon contribution guide](resources/lexicon/README.md#contribute) |

Report problems through [GitHub issues](https://github.com/sospedra/blasphem/issues).
Include the language, input text, and expected result.

## Evidence and license

[Benchmarks](crates/blasphem-bench/README.md) · [Code license](LICENSE) · [Data notices](NOTICE)

*NIHIL PROFANUM*
