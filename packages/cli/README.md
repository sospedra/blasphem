# Blasphem CLI

Check one message or a stream of messages.
The Rust binary embeds all supported language data.

## Installation

Build from this checkout:

```sh
cargo install --locked --path crates/blasphem
```

Published binaries are pending the first public release.
The release workflow targets [GitHub Releases](https://github.com/sospedra/blasphem/releases).
After publication, the npm package also supports:

```sh
npx blasphem judge "you are a stupid loser"
```

The private `@blasphem/cli` workspace builds platform packages.
Install the public `blasphem` package for the npm command.

## Usage

```sh
blasphem judge "you are a stupid loser"
blasphem judge --json --locales en,es --grawlix "you are a stupid loser"
printf 'hello there\nTe voy a matar\n' | blasphem judge --json
```

Without a text argument, each stdin line is one message.
JSON mode writes one object per message.

## Options

```text
blasphem judge [OPTIONS] [TEXT]
```

| Flag | Effect |
| --- | --- |
| `--locales en,es` | Load only the selected locales |
| `--no-detect` | Return the highest score across loaded locales |
| `--grawlix` | Include masked text |
| `--json` | Write JSON output |
| `--help` | Show command help |

The default loads all 16 supported language codes.
Indonesian and Malay share one model profile.
See [the locale list](../packs/README.md#locales).

## Output and exit status

JSON objects contain `safe`, `score`, `locale`, and `grawlix`.
The score ranges from 0 to 1.
It is an ordinal value, not a probability.
Unrouted text produces a safe verdict with zero score.

| Status | Meaning |
| --- | --- |
| `0` | No message needs a warning |
| `1` | At least one message needs a warning |
| `2` | Invalid arguments, input, or runtime resources |

A broken output pipe exits successfully.
Account for status `1` in shell scripts.

## Development

Run from the repository root:

```sh
cargo build --release --locked -p blasphem --bin blasphem
cargo test --locked -p blasphem --test cli
pnpm --filter @blasphem/cli run build
```

The npm build packages the host binary.
[CLI source](../../crates/blasphem/src/main.rs) · [Contribute](../../CONTRIBUTING.md)

## License

Code uses [Apache-2.0](../../LICENSE).
Embedded data retains the terms recorded in [NOTICE](../../NOTICE).
