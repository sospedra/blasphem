# Pinned tools

The browser package builds only with these versions.

| Tool | Version | Source |
| --- | --- | --- |
| Rust | 1.97.0 | `rust-toolchain.toml` at the repository root |
| Node | 24.18.0 | root `package.json` `engines` |
| pnpm | 11.13.0 | root `package.json` `packageManager` |
| `wasm-bindgen-cli` | 0.2.127 | `crates/blasphem-wasm/Cargo.toml`; install with `cargo install wasm-bindgen-cli --version 0.2.127 --locked` |
| `wasm32-unknown-unknown` target | matches Rust | `rustup target add wasm32-unknown-unknown` |

`scripts/build.mjs` reads the crate name and the `wasm-bindgen` pin from the crate manifest and stops on a CLI mismatch.
