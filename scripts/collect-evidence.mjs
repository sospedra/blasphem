import { execFileSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const reports = resolve(root, "reports");
const inputs = resolve(root, "target/evidence-inputs");
const manifest = "resources/metadata/model-manifest.json";
const fixtures = "crates/blasphem/tests/fixtures/benchmark/messages.jsonl";
const target = execFileSync("rustc", ["-vV"], { encoding: "utf8" })
  .split("\n").find((line) => line.startsWith("host: "))?.slice(6);
if (!target) throw new Error("rustc did not report its host target");
const computer = process.env.GITHUB_RUN_ID ? `GitHub Actions ${process.env.RUNNER_OS}` : `${process.platform} ${process.arch}`;
const measured = ["--computer", computer, "--target-triple", target];
const modelInputs = ["--model-manifest", manifest, "--lexicon-root", "resources/lexicon"];
const source = "https://raw.githubusercontent.com/nitotm/eldc/a0301db809ff2e48a418018aa5359fb0c4354eb8/benchmark/text_files";

function run(program, arguments_) {
  execFileSync(program, arguments_, { cwd: root, stdio: "inherit" });
}

function bench(arguments_) {
  run("cargo", ["run", "--release", "--locked", "-p", "blasphem-bench", "--", ...arguments_]);
}

async function download(name) {
  const response = await fetch(`${source}/${name}`, { signal: AbortSignal.timeout(120_000) });
  if (!response.ok) throw new Error(`${name}: HTTP ${response.status}`);
  const path = resolve(inputs, name);
  await writeFile(path, Buffer.from(await response.arrayBuffer()));
  return path;
}

await mkdir(inputs, { recursive: true });
await mkdir(reports, { recursive: true });
// The AUTO command checks both corpus hashes before it measures any route.
const [texts, labels] = await Promise.all([
  download("tatoeba_50_v3.txt"),
  download("tatoeba_50_v3.languages.txt"),
]);

// Accuracy regenerates validation, behavior, and CLI evidence before judging test rows.
const benchmark = resolve(reports, "benchmarks/current.json");
bench(["accuracy", "--output", benchmark]);
run("pnpm", ["--filter", "@blasphem/packs", "build"]);
run("pnpm", ["--filter", "blasphem", "build"]);
run("pnpm", ["--filter", "blasphem", "test:browser"]);
bench(["benchmark", "--fixtures", fixtures, ...modelInputs, ...measured,
  "--output", resolve(reports, "multilingual-performance.json")]);
bench(["size", "--binary", "target/release/blasphem", ...modelInputs,
  "--target-triple", target, "--output", resolve(reports, "multilingual-size.json")]);
bench(["auto", "--texts", texts, "--labels", labels, "--fixtures", fixtures,
  ...modelInputs, ...measured, "--native-binary", "target/release/blasphem",
  "--language-model-artifact", "crates/blasphem-language/data/blasphem-language-15.bin",
  "--browser-report", resolve(reports, "browser-smoke.json"),
  "--c-parity-fixture", "crates/blasphem-language/tests/fixtures/c-parity-v1.jsonl",
  "--project-root", root, "--output", resolve(reports, "language-auto-validation.json")]);
run("node", ["scripts/snapshot-evidence.mjs", "--benchmark", benchmark,
  "--output", resolve(reports, "website-snapshot.json")]);
