# Blasphem

Blasphem checks text for toxicity before someone sends it.
It provides an experimental warning for 16 languages.
Messages can stay on the device.
No runtime AI needed.

## How it works

1. Blasphem normalizes the text and identifies its language.
2. Dictionaries, context rules, and small integer weight tables score the message.
3. Blasphem returns a verdict and optional masked text.

Labeled messages train the weight tables offline.
The runtime uses no neural model.
Read [how Blasphem works](HOW.md) for the architecture.

## Distributions

| Platform | Installation and usage |
| --- | --- |
| Command line | [CLI guide](packages/cli/README.md) |
| Rust | [Rust crate](crates/blasphem/README.md) |
| JavaScript and TypeScript | [Node and browser package](packages/blasphem/README.md) |
| React Native | [React Native package](packages/react-native/README.md) |
| Swift | [Swift package](packages/swift/README.md) |
| Android | [Kotlin library](packages/android/README.md) |
| Python | [Python package](packages/python/README.md) |
| Go | [Go module](packages/go/README.md) |

[Language packs](packages/packs/README.md) · [Python data wheel](packages/python-packs/README.md) · [WASM bindings](crates/blasphem-wasm/README.md)

## Contribute

| Contribution | Start here |
| --- | --- |
| Code, bindings, or documentation | [Development guide](CONTRIBUTING.md#code-and-documentation) |
| Labeled messages in `corpus/` | [Corpus contribution guide](corpus/README.md#contribute) |
| Words and categories in `lexicon/` | [Lexicon contribution guide](lexicon/README.md#contribute) |

Report problems through [GitHub issues](https://github.com/sospedra/blasphem/issues).
Include the language, input text, and expected result.

## Evidence and license

[Benchmarks](benchmark/README.md) · [Code license](LICENSE) · [Data notices](NOTICE)
