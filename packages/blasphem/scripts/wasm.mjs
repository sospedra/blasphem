import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { projectRoot } from "./crate.mjs";

export function capture(command, args) {
  return execFileSync(command, args, { encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] });
}

export function stream(command, args, env = process.env) {
  execFileSync(command, args, { stdio: "inherit", env });
}

/** Stops unless the wasm-bindgen CLI matches the crate's exact pin. */
export function assertWasmBindgen(expected) {
  const found = capture("wasm-bindgen", ["--version"]).trim();
  if (found === `wasm-bindgen ${expected}`) return;
  throw new Error(
    `wasm-bindgen-cli must be ${expected}, found "${found}". Run: cargo install wasm-bindgen-cli --version ${expected} --locked`,
  );
}

/** Builds the crate for wasm32 in release mode and returns the module path. The crate turns the core's embedded data off. */
export function buildWasm(crate, { targetDir }) {
  const args = [
    "build",
    "--release",
    "--locked",
    "--target",
    "wasm32-unknown-unknown",
    "-p",
    crate.name,
    "--manifest-path",
    resolve(projectRoot, "Cargo.toml"),
  ];
  stream("cargo", args, { ...process.env, CARGO_TARGET_DIR: targetDir });
  return resolve(targetDir, "wasm32-unknown-unknown/release", `${crate.libName}.wasm`);
}

/**
 * Writes the web-target glue into outDir as blasphem.js and blasphem_bg.wasm.
 * `--omit-default-module-path` removes the `new URL(..., import.meta.url)`
 * fallback, so the loader must name the wasm location and bundlers never see
 * an implicit asset reference.
 */
export function generateGlue(wasmPath, outDir) {
  stream("wasm-bindgen", [wasmPath, "--target", "web", "--omit-default-module-path", "--out-dir", outDir, "--out-name", "blasphem"]);
}
