import { copyFileSync, cpSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { packageRoot, projectRoot, readCrate } from "./crate.mjs";
import { assertWasmBindgen, buildWasm, generateGlue, stream } from "./wasm.mjs";

const distribution = resolve(packageRoot, "dist");
const sources = resolve(packageRoot, "src");
const coreSource = resolve(projectRoot, "packages/core/src");
const coreCopy = resolve(sources, "core");
const GLUE_FILES = ["blasphem.js", "blasphem.d.ts", "blasphem_bg.wasm", "blasphem_bg.wasm.d.ts"];
const VERSION_FILE = "version.generated.ts";
const targetDir = resolve(projectRoot, "target/npm-wasm");
const REQUIRED_CLASSES = ["class BlasphemEngineBuilder", "class BlasphemEngine"];

function clean() {
  rmSync(distribution, { recursive: true, force: true });
  rmSync(coreCopy, { recursive: true, force: true });
  for (const file of [...GLUE_FILES, VERSION_FILE]) rmSync(resolve(sources, file), { force: true });
}

/** The exact versions `assets: "jsdelivr"` pins: this package and the packs it was built beside. */
function writeVersions() {
  const own = JSON.parse(readFileSync(resolve(packageRoot, "package.json"), "utf8"));
  const packs = JSON.parse(readFileSync(resolve(projectRoot, "packages/packs/package.json"), "utf8"));
  const source = `// Written by scripts/build.mjs. Do not edit.\nexport const VERSIONS = { blasphem: ${JSON.stringify(own.version)}, packs: ${JSON.stringify(packs.version)} } as const;\n`;
  writeFileSync(resolve(sources, VERSION_FILE), source);
  return { blasphem: own.version, packs: packs.version };
}

/** The private core is never published. Each package carries its own copy. */
function inlineCore() {
  cpSync(coreSource, coreCopy, { recursive: true });
  const copied = readdirSync(coreCopy).filter((name) => name.endsWith(".ts"));
  if (copied.length === 0) throw new Error(`${coreSource} has no TypeScript sources`);
  return copied.length;
}

function assertClasses() {
  const glue = readFileSync(resolve(sources, "blasphem.js"), "utf8");
  const missing = REQUIRED_CLASSES.filter((marker) => !glue.includes(marker));
  if (missing.length > 0) throw new Error(`blasphem.js lacks ${missing.join(", ")}. The crate needs js_name and js_class attributes.`);
  if (glue.includes("import.meta.url")) throw new Error("blasphem.js still references import.meta.url; pass --omit-default-module-path");
}

function compileTypeScript() {
  stream("pnpm", ["exec", "tsc", "--project", resolve(packageRoot, "tsconfig.json")]);
}

function copyGlue() {
  mkdirSync(distribution, { recursive: true });
  for (const file of GLUE_FILES) copyFileSync(resolve(sources, file), resolve(distribution, file));
}

const crate = readCrate();
assertWasmBindgen(crate.wasmBindgenVersion);
clean();
const versions = writeVersions();
const coreFiles = inlineCore();
generateGlue(buildWasm(crate, { targetDir }), sources);
assertClasses();
compileTypeScript();
copyGlue();
const wasmBytes = statSync(resolve(distribution, "blasphem_bg.wasm")).size;
const glueBytes = statSync(resolve(distribution, "blasphem.js")).size;
console.log(`status=built wasm_bytes=${wasmBytes} wasm_mb=${(wasmBytes / 1048576).toFixed(2)} glue_bytes=${glueBytes} core_files=${coreFiles} versions=${versions.blasphem}/${versions.packs}`);
