# Blasphem CLI implementation plan

Spec: `docs/superpowers/specs/2026-09-04-blasphem-cli-design.md`. Executed 2026-09-04 in one session; boxes record what landed.

## Task 1: `judge` subcommand

- [x] Append eight `judge` tests to `tests/cli.rs`; run them; all fail on "unrecognized subcommand".
- [x] Rewrite `src/main.rs`: `Judge` variant, `Check` hidden, `main -> ExitCode`, `exit_code` maps Ok(false)/Ok(true)/Err to 0/1/2, broken pipe to 0.
- [x] Move `serde_json` from dev-dependencies to dependencies for `--json`.
- [x] `cargo fmt -p blasphem`, `cargo test --test cli --test multilingual_cli_contract`, `cargo clippy -p blasphem --all-targets --locked -- -D warnings`.

## Task 2: cargo-dist

- [x] `brew install cargo-dist` (0.32.0; no asdf plugin exists).
- [x] `dist init --yes --skip-generate --hosting github --ci github -i shell -i powershell -t <7 targets>`.
- [x] Root `Cargo.toml`: `repository`, `[profile.dist] inherits = "release"`, `[package.metadata.dist] dist = true`.
- [x] `dist-workspace.toml`: `include = ["NOTICE"]`.
- [x] `dist generate` writes `.github/workflows/release.yml`; `dist generate --check` and `dist plan` pass.

## Task 3: `npx blasphem`

- [x] `packages/cli`: `package.json`, `NOTICE`, `scripts/targets.mjs`, `scripts/build.mjs`, `scripts/npm-dirs.mjs`, `npm/<target>/{package.json,NOTICE}` for seven targets.
- [x] `packages/blasphem/bin/blasphem.mjs` launcher; `package.json` bin and optionalDependencies; `pack-check.mjs` expectations.
- [x] `pnpm-workspace.yaml` adds `packages/cli/npm/*`; `turbo.json` adds `@blasphem/cli#build` and makes `blasphem#test` depend on it; `.gitignore` adds `/packages/cli/npm/*/bin/`.
- [x] `pnpm install` updates the lockfile.
- [x] `node-smoke.mjs` runs the launcher on the README example and the supplied cases when the host binary exists.

## Task 4: Documentation

- [x] README: install paths and `judge` usage; `check` noted as the hidden diagnostic.
- [x] Spec implementation notes with measurements.

## Follow-ups (not in this plan)

- `--packs <dir>` on the binary, verifying `manifest.json` digests.
- Homebrew tap: create `sospedra/homebrew-tap`, add `HOMEBREW_TAP_TOKEN`, set `installers += ["homebrew"]`, `tap`, `publish-jobs = ["homebrew"]`.
- npm publish workflow that downloads the seven release binaries into `packages/cli/npm/*/bin/` before `pnpm publish`.
