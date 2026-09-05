# Pinned tools

JavaScript source builds and package checks use these pinned versions.
The [prebuilt website build](README.md#build-without-rust) needs only Node and pnpm.

| Tool | Version | Source |
| --- | --- | --- |
| Rust | 1.97.0 | `rust-toolchain.toml` at the repository root |
| Node | 24.18.0 | root `package.json` `engines` |
| pnpm | 11.13.0 | root `package.json` `packageManager` |
| `wasm-bindgen-cli` | 0.2.127 | `crates/blasphem-wasm/Cargo.toml`; install with `cargo install wasm-bindgen-cli --version 0.2.127 --locked` |
| `wasm32-unknown-unknown` target | matches Rust | `rustup target add wasm32-unknown-unknown` |
| napi, napi-derive | 3 | `crates/blasphem-napi/Cargo.toml`; no CLI needed, `packages/node/scripts/build.mjs` runs cargo |
| Playwright | 1.62.1 | `package.json` `devDependencies` |
| Chromium and WebKit | pinned by Playwright 1.62.1 | `pnpm --filter blasphem exec playwright install chromium webkit` |
| react-native-nitro-modules, nitrogen | 0.37.1 | `packages/react-native/package.json` |

`scripts/build.mjs` copies `packages/javascript-common/src` into `src/core` and compiles TypeScript.
The default build reads the crate manifest and rejects a `wasm-bindgen` CLI mismatch.
With `BLASPHEM_WASM_PREBUILT=1`, it verifies committed artifacts and skips Rust compilation and binding generation.
`scripts/browser-smoke.mjs` launches the pinned browsers and stops when one is missing.
`packages/node/scripts/build.mjs` builds the native binary for the host only.
CI fills the other six platform packages.
