import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { caseTotal, failures, runCases, verdictSignature } from "../tests/cases.mjs";
import { packageRoot, projectRoot } from "./crate.mjs";
import { binaryName, hostTarget } from "../../node/scripts/targets.mjs";

const entry = resolve(packageRoot, "dist/node.js");
if (!existsSync(entry)) throw new Error(`missing ${entry}. Run: pnpm --filter blasphem run build`);
const packs = resolve(projectRoot, "packages/packs/dist/manifest.json");
if (!existsSync(packs)) throw new Error(`missing ${packs}. Run: pnpm --filter @blasphem/packs run build`);

const target = hostTarget();
const nativeBuilt = target !== null && existsSync(resolve(projectRoot, "packages/node/npm", target.name, binaryName(target)));

const started = performance.now();
const api = await import(pathToFileURL(entry).href);
const importMs = performance.now() - started;

function report(label, fragment) {
  if (fragment.passed) return;
  console.error(`status=failed run=${label} node=${process.version} cases=${caseTotal(fragment) - failures(fragment).length}/${caseTotal(fragment)}`);
  console.error(JSON.stringify(failures(fragment), null, 2));
  process.exit(1);
}

const first = await runCases(api);
report("default", first);
const expectedTransport = nativeBuilt ? "native" : "wasm";
if (first.transport !== expectedTransport) {
  console.error(`status=failed reason=transport expected=${expectedTransport} actual=${first.transport}`);
  process.exit(1);
}

process.env.BLASPHEM_FORCE_WASM = "1";
const second = await runCases(api);
report("forced-wasm", second);
if (second.transport !== "wasm") {
  console.error(`status=failed reason=forced-wasm-transport actual=${second.transport}`);
  process.exit(1);
}
if (verdictSignature(first) !== verdictSignature(second)) {
  console.error("status=failed reason=transports-disagree");
  process.exit(1);
}

console.log(`status=passed node=${process.version} transport=${first.transport} cases=${caseTotal(first)} wasm_cases=${caseTotal(second)} import_ms=${importMs.toFixed(1)}`);
