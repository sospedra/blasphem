import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { README_EXAMPLE, SUPPLIED_CASES, caseTotal, failures, invariantsHold, runCases, verdictSignature } from "../tests/cases.mjs";
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

// The `blasphem` bin runs the embedded binary from the platform package. Its
// verdicts must match the pack-based library on the shared cases.
const launcher = resolve(packageRoot, "bin/blasphem.mjs");
const cliBinary = target === null ? null : resolve(projectRoot, "packages/cli/npm", target.name, "bin", process.platform === "win32" ? "blasphem.exe" : "blasphem");

function cliJudge(args) {
  const run = spawnSync(process.execPath, [launcher, "judge", "--json", ...args], { encoding: "utf8" });
  if (run.error) throw run.error;
  const line = run.stdout.trim();
  return { status: run.status, verdict: line === "" ? null : JSON.parse(line), stderr: run.stderr };
}

function readmeCase() {
  const expected = README_EXAMPLE.verdict;
  const { status, verdict, stderr } = cliJudge(["--locales", "en,es", "--grawlix", README_EXAMPLE.text]);
  const matches = verdict !== null && verdict.safe === expected.safe && Math.abs(verdict.score - expected.score) < 1e-9 && verdict.locale === expected.locale && verdict.grawlix === expected.grawlix;
  return { case_id: "cli-readme-example", passed: status === 1 && invariantsHold(verdict, true) && matches, status, verdict, stderr };
}

function suppliedCase([caseId, language, text, expectedNudge]) {
  const { status, verdict, stderr } = cliJudge(["--no-detect", "--locales", language, text]);
  const passed = verdict !== null && invariantsHold(verdict, false) && verdict.safe === !expectedNudge && status === (expectedNudge ? 1 : 0);
  return { case_id: `cli-${caseId}`, passed, status, verdict, stderr };
}

const cliResults = cliBinary !== null && existsSync(cliBinary) ? [readmeCase(), ...SUPPLIED_CASES.map(suppliedCase)] : null;
const cliFailed = cliResults?.filter((result) => !result.passed) ?? [];
if (cliFailed.length > 0) {
  console.error(`status=failed run=cli cases=${cliResults.length - cliFailed.length}/${cliResults.length}`);
  console.error(JSON.stringify(cliFailed, null, 2));
  process.exit(1);
}

console.log(`status=passed node=${process.version} transport=${first.transport} cases=${caseTotal(first)} wasm_cases=${caseTotal(second)} cli_cases=${cliResults === null ? "skipped" : cliResults.length} import_ms=${importMs.toFixed(1)}`);
