# Blasphem command-line interface design

## Status

Rubén approved the design in chat on 2026-09-04 and said "go". Claude implemented it the same day on `development`, uncommitted. The `check` command keeps the contract from `2026-09-01-multilingual-sparse-nudge-detector-design.md` and `2026-09-02-eldc-auto-language-design.md`; this document only hides it from help.

## Goals

- One `blasphem` binary a person installs and runs: `blasphem judge "text"`.
- The same four verdict fields as the JavaScript contract: `safe`, `score`, `locale`, `grawlix`.
- Three install paths: GitHub Releases through cargo-dist, `cargo install`, and `npx blasphem`.
- Exit codes a shell script can branch on.

## Non-goals

- Pack loading (`--packs <dir>`). The binary embeds all fifteen locales. `Judge::from_packs` exists for a follow-up.
- A Homebrew tap. It needs a tap repository and a `HOMEBREW_TAP_TOKEN`; neither exists.
- An npm publish pipeline. No package in this workspace has one yet.
- Replacing `check`. It stays as the evidence tool for `tests/cli.rs`, `tests/multilingual_cli_contract.rs`, and `blasphem-train reproduce`.

## Command

```
blasphem judge [--locales en,es] [--no-detect] [--grawlix] [--json] [TEXT]
```

The binary shall judge `TEXT` when given. Without `TEXT` and with stdin not a terminal, it shall judge one message per stdin line and print one verdict per line, in order, as it reads. Without `TEXT` on a terminal it shall exit 2 with a usage error.

`--locales` shall accept the fifteen codes and the `id` alias for `ms`, in any case, comma separated. An unknown code shall exit 2 and name it. Absent, every locale loads.

`--no-detect` shall score every loaded locale and report the highest, as `JudgeOptions::detect_language = false` does. The default routes by detected language and fails open to `locale=none` when nothing routes.

`--grawlix` shall add the masked text. `--json` shall print one object per verdict with the keys in this order: `safe`, `score`, `locale`, `grawlix`. `score` is 0.0 through 1.0. Absent values are `null`.

The human line shall be `safe=<bool> score=<number> locale=<code|none>`, plus ` grawlix="<escaped>"` when requested. Debug escaping keeps every verdict on one line.

Exit codes: 0 when no verdict nudged, 1 when any verdict nudged, 2 on an error. A closed stdout, as in `blasphem judge < file | head`, exits 0.

`check` shall stay hidden from `--help`, keep its output, and exit 2 instead of 1 on errors.

## Data

The binary embeds the sparse tables, the HurtLex lexica, and the language model through the `embedded` feature and `Judge::new`. Verdicts shall equal the pack path. The Node smoke enforces this: when the platform binary exists, it runs the README example and the thirty supplied cases through the launcher and compares them with the shared expectations.

## Distribution

1. GitHub Releases. `dist-workspace.toml` pins cargo-dist 0.32.0, seven targets (`aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-pc-windows-msvc`), the `shell` and `powershell` installers, and `include = ["NOTICE"]`. `.github/workflows/release.yml` is generated; `dist generate --check` must pass. A tag `v<version>` releases. The two aarch64 Linux targets build on GitHub's `ubuntu-22.04-arm` runners, free because the repository is public. `[profile.dist]` inherits `release` and changes nothing, so the release binary follows the recipe `reproduce` builds. `[package.metadata.dist] dist = true` opts the root package in, because `publish = false` hides binaries from dist.
2. `cargo install --git https://github.com/sospedra/blasphem blasphem` works from the repository. crates.io waits for `publish = false` to change.
3. npm. The `blasphem` package gains a `blasphem` bin, `bin/blasphem.mjs`, and seven `@blasphem/cli-<os>-<cpu>` optionalDependencies. `packages/cli` builds the binary for the host and generates the platform manifests, mirroring `packages/node`. The launcher resolves the platform package, spawns the binary with inherited stdio, and forwards its exit code. Platform packages carry `NOTICE` and the license `Apache-2.0 AND CC-BY-NC-SA-4.0`, because the binary embeds HurtLex.

The npm cost: every install of `blasphem` on a covered platform also downloads the CLI package, 11.28 MB on darwin-arm64. The alternative is a separate `@blasphem/cli` package that owns the optionalDependencies; that loses `npx blasphem`. Rubén chose `npx blasphem`.

## Error behavior

| Case | Exit | Output |
| --- | --- | --- |
| No nudge | 0 | one verdict per message |
| Any nudge | 1 | one verdict per message |
| Unknown locale | 2 | `Error: unsupported locale "xx"` |
| No text on a terminal | 2 | `Error: no text given: pass TEXT, or pipe one message per line` |
| Usage error | 2 | clap's message |
| Broken stdout pipe | 0 | nothing |
| Launcher without a platform package | 2 | install hint and the releases URL |

## Tests

`tests/cli.rs` gains eight cases: safe verdict exits 0, nudge exits 1, JSON shape, grawlix masking, one verdict per stdin line, empty stdin, unknown locale exits 2, top-level help shows `judge` and hides `check`.

`packages/blasphem/scripts/node-smoke.mjs` runs the launcher when `packages/cli/npm/<host>/bin/blasphem` exists and reports `cli_cases=skipped` otherwise. `pack:check` asserts the new bin and the fourteen optionalDependencies.

## Acceptance criteria

- `cargo test --test cli` passes with the eight new cases.
- `cargo clippy -p blasphem --all-targets --locked -- -D warnings` passes.
- `dist plan` lists one release, `blasphem`, with seven targets; `dist generate --check` passes.
- `node packages/blasphem/bin/blasphem.mjs judge --json --locales en,es --grawlix "you are a stupid loser"` prints the README verdict and exits 1.
- `pnpm --filter blasphem run pack:check` and `test:node` pass.

## Implementation notes, 2026-09-04

**Binary size.** `target/release/blasphem` grew from 8.75 MB to 11.28 MB. `check` read HurtLex from disk, so the linker dropped the embedded lexica and sparse tables; `judge` references them through `Judge::new`.

**Safe verdicts carry a score.** "I hope you have a wonderful day" scores 0.29 and stays safe; the threshold is 0.5. The test pins 0.29.

**dist plan.** One release, `blasphem 0.1.0`, tag `v0.1.0`, 19 artifacts: seven archives with checksums, two installers, `sha256.sum`, `source.tar.gz`. Runners: `macos-14`, `macos-15-intel`, `ubuntu-22.04`, `ubuntu-22.04-arm`, `windows-2022`.

**Verified.** `cargo test --test cli --test multilingual_cli_contract`: 24 and 4 cases. `cargo clippy --workspace --all-targets --locked -- -D warnings` clean. `dist generate --check` clean. Launcher: the README example returns `{"safe":false,"score":0.64,"locale":"en","grawlix":"you are a @#$%&! loser"}` and exit 1. `pnpm --filter blasphem run test:node`: `cases=65 wasm_cases=65 cli_cases=31`. `pack:check`: 40 files.

**Left out.** `packages/blasphem/README.md` does not yet mention `npx blasphem`; another session was editing that file. No npm publish workflow exists, so the platform packages are built for the host only, as with `@blasphem/node-*`.

**wasm bytes.** Moving `serde_json` from dev-dependencies to dependencies changes the wasm from 1,611,137 to 1,611,129 bytes. Isolated in a clean `git archive HEAD` copy: the HEAD manifest rebuilds yesterday's artifact byte for byte (sha256 87e9f8d9…), the new manifest rebuilds today's (f7457c65…). The crate metadata hash changes and functions reorder; no code is added. Rubén can gate the dependency behind a `cli` feature if bit-identical wasm matters.
