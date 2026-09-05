import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, readFileSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(packageRoot, "../..");
const committedWasm = resolve(packageRoot, "blasphem_ffi.wasm");
const args = process.argv.slice(2);

if (args.length > 1 || (args.length === 1 && args[0] !== "--check")) {
  throw new Error("Usage: node packages/go/scripts/build-wasm.mjs [--check]");
}

const check = args[0] === "--check";
const cargo = process.env.CARGO || "cargo";
const rustc = process.env.RUSTC || "rustc";
const targetDir = process.env.CARGO_TARGET_DIR || resolve(projectRoot, "target/go-wasm");
const cargoEnv = { ...process.env, CARGO_TARGET_DIR: targetDir };

function capture(command, commandArgs, env = cargoEnv) {
  return execFileSync(command, commandArgs, {
    cwd: projectRoot,
    encoding: "utf8",
    env,
    maxBuffer: 16 * 1024 * 1024,
    stdio: ["ignore", "pipe", "inherit"],
  }).trim();
}

function splitRustFlags(value) {
  return value.trim() === "" ? [] : value.trim().split(/\s+/);
}

function inheritedRustFlags() {
  if (Object.hasOwn(process.env, "CARGO_ENCODED_RUSTFLAGS")) {
    const value = process.env.CARGO_ENCODED_RUSTFLAGS;
    return value === "" ? [] : value.split("\x1f");
  }
  if (Object.hasOwn(process.env, "RUSTFLAGS")) return splitRustFlags(process.env.RUSTFLAGS);
  if (Object.hasOwn(process.env, "CARGO_BUILD_RUSTFLAGS")) {
    return splitRustFlags(process.env.CARGO_BUILD_RUSTFLAGS);
  }
  return [];
}

const metadata = JSON.parse(capture(cargo, ["metadata", "--format-version=1", "--locked"]));
const sysroot = capture(rustc, ["--print", "sysroot"], process.env);
const registryRoots = new Set(
  metadata.packages
    .filter((entry) => entry.source?.startsWith("registry+"))
    .map((entry) => dirname(dirname(entry.manifest_path))),
);

const mappings = [
  [metadata.workspace_root, "/workspace"],
  [metadata.target_directory, "/target"],
  [sysroot, "/rustc"],
  ...[...registryRoots].map((root) => [root, "/cargo/registry"]),
].sort(([left], [right]) => left.length - right.length || left.localeCompare(right));

const rustFlags = [
  ...inheritedRustFlags(),
  ...mappings.map(([from, to]) => `--remap-path-prefix=${from}=${to}`),
];
const buildEnv = {
  ...cargoEnv,
  CARGO_ENCODED_RUSTFLAGS: rustFlags.join("\x1f"),
};
delete buildEnv.RUSTFLAGS;

execFileSync(cargo, [
  "build",
  "--release",
  "--locked",
  "-p",
  "blasphem-ffi",
  "--target",
  "wasm32-unknown-unknown",
], { cwd: projectRoot, env: buildEnv, stdio: "inherit" });

const builtWasm = resolve(metadata.target_directory, "wasm32-unknown-unknown/release/blasphem_ffi.wasm");
const builtBytes = readFileSync(builtWasm);
const hash = createHash("sha256").update(builtBytes).digest("hex");

if (check) {
  if (!builtBytes.equals(readFileSync(committedWasm))) {
    throw new Error("The embedded Go WASM is stale; run node packages/go/scripts/build-wasm.mjs");
  }
} else {
  copyFileSync(builtWasm, committedWasm);
}

console.log(`status=${check ? "verified" : "generated"} sha256=${hash} bytes=${statSync(builtWasm).size}`);
