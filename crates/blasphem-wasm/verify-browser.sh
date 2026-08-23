#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
FULL_TARGET="$PROJECT_ROOT/target/task7-wasm-full"
FULL_INPUT="$FULL_TARGET/wasm32-unknown-unknown/release/blasphem_wasm.wasm"
FULL_OUTPUT="$PROJECT_ROOT/target/task7-wasm-full-web"
EXPLICIT_TARGET="$PROJECT_ROOT/target/task7-wasm-explicit"
EXPLICIT_INPUT="$EXPLICIT_TARGET/wasm32-unknown-unknown/release/blasphem_wasm.wasm"
EXPLICIT_OUTPUT="$PROJECT_ROOT/target/task7-wasm-explicit-web"
REPORT_OUTPUT="$PROJECT_ROOT/reports/multilingual-wasm.json"

CARGO_TARGET_DIR="$FULL_TARGET" cargo build --release --locked --target wasm32-unknown-unknown -p blasphem-wasm --manifest-path "$PROJECT_ROOT/Cargo.toml"
mkdir -p "$FULL_OUTPUT"
wasm-bindgen "$FULL_INPUT" --target web --out-dir "$FULL_OUTPUT" --out-name blasphem

CARGO_TARGET_DIR="$EXPLICIT_TARGET" cargo build --release --locked --target wasm32-unknown-unknown -p blasphem-wasm --no-default-features --manifest-path "$PROJECT_ROOT/Cargo.toml"
mkdir -p "$EXPLICIT_OUTPUT"
wasm-bindgen "$EXPLICIT_INPUT" --target web --out-dir "$EXPLICIT_OUTPUT" --out-name blasphem

node "$SCRIPT_DIR/tests/run-browser-smoke.mjs" "$PROJECT_ROOT" "$REPORT_OUTPUT" "$FULL_OUTPUT" "$EXPLICIT_OUTPUT"
