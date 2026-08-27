import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { resolve } from "node:path";
import { packageRoot, projectRoot, readCrate } from "./crate.mjs";

const distribution = resolve(packageRoot, "dist");
const sources = resolve(packageRoot, "src");
const GLUE_FILES = ["blasphem.js", "blasphem.d.ts", "blasphem_bg.wasm", "blasphem_bg.wasm.d.ts"];
const targetDir = resolve(projectRoot, "target/npm-wasm");
const REQUIRED_CLASSES = ["class BlasphemDetector", "class BlasphemJudge", "class BlasphemResult"];

function capture(command, args) {
  return execFileSync(command, args, { encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] });
}

function stream(command, args, env = process.env) {
  execFileSync(command, args, { stdio: "inherit", env });
}

function assertWasmBindgen(expected) {
  const found = capture("wasm-bindgen", ["--version"]).trim();
  if (found === `wasm-bindgen ${expected}`) return;
  throw new Error(
    `wasm-bindgen-cli must be ${expected}, found "${found}". Run: cargo install wasm-bindgen-cli --version ${expected} --locked`,
  );
}

function buildCrate(crate) {
  stream(
    "cargo",
    [
      "build",
      "--release",
      "--locked",
      "--target",
      "wasm32-unknown-unknown",
      "-p",
      crate.name,
      "--manifest-path",
      resolve(projectRoot, "Cargo.toml"),
    ],
    { ...process.env, CARGO_TARGET_DIR: targetDir },
  );
  return resolve(targetDir, "wasm32-unknown-unknown/release", `${crate.libName}.wasm`);
}

function generateGlue(wasmPath) {
  rmSync(distribution, { recursive: true, force: true });
  for (const file of GLUE_FILES) rmSync(resolve(sources, file), { force: true });
  stream("wasm-bindgen", [wasmPath, "--target", "web", "--out-dir", sources, "--out-name", "blasphem"]);
}

function copyGlue() {
  mkdirSync(distribution, { recursive: true });
  for (const file of GLUE_FILES) copyFileSync(resolve(sources, file), resolve(distribution, file));
}

function assertClasses() {
  const glue = readFileSync(resolve(sources, "blasphem.js"), "utf8");
  const missing = REQUIRED_CLASSES.filter((marker) => !glue.includes(marker));
  if (missing.length === 0) return;
  throw new Error(`dist/blasphem.js lacks ${missing.join(", ")}. The crate needs js_name and js_class attributes.`);
}

function compileTypeScript() {
  stream("pnpm", ["exec", "tsc", "--project", resolve(packageRoot, "tsconfig.json")]);
}

const crate = readCrate();
assertWasmBindgen(crate.wasmBindgenVersion);
generateGlue(buildCrate(crate));
assertClasses();
compileTypeScript();
copyGlue();
const wasmBytes = statSync(resolve(distribution, "blasphem_bg.wasm")).size;
const glueBytes = statSync(resolve(distribution, "blasphem.js")).size;
console.log(`status=built wasm_bytes=${wasmBytes} glue_bytes=${glueBytes}`);
