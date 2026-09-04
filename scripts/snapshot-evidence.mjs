import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, relative, resolve } from "node:path";
import { parseArgs } from "node:util";

const root = resolve(import.meta.dirname, "..");
const { values } = parseArgs({ options: {
  reports: { type: "string", default: "reports" },
  benchmark: { type: "string" },
  output: { type: "string", default: "apps/web/src/data/evidence.json" },
} });
const reports = resolve(root, values.reports);
const output = resolve(root, values.output);

function readReport(path) {
  const bytes = readFileSync(path);
  return {
    data: JSON.parse(bytes),
    source: { path: relative(root, path), sha256: createHash("sha256").update(bytes).digest("hex") },
  };
}

function latestBenchmark(directory) {
  const runs = readdirSync(directory)
    .filter((name) => name.endsWith(".json"))
    .map((name) => readReport(resolve(directory, name)))
    .filter(({ data }) => isFullBenchmark(data))
    .sort((left, right) => right.data.generated_unix_seconds - left.data.generated_unix_seconds
      || left.source.path.localeCompare(right.source.path));
  if (!runs.length) throw new Error(`No retrained test benchmark in ${directory}`);
  return runs[0];
}

function isFullBenchmark(data) {
  return data.schema_version === 1 && data.retrained && data.test?.split === "test";
}

function select(record, fields) {
  return Object.fromEntries(fields.map((field) => {
    if (!(field in record)) throw new Error(`Evidence misses ${field}`);
    return [field, record[field]];
  }));
}

function mapValues(record, project) {
  return Object.fromEntries(Object.entries(record).map(([key, value]) => [key, project(value)]));
}

function cases(report) {
  return Object.values(report.languages).flatMap((language) => language.cases);
}

function totals(report) {
  const entries = cases(report);
  return { total: entries.length, passed: entries.filter((entry) => entry.passed).length };
}

const inputs = mapValues({
  performance: "multilingual-performance.json",
  sizes: "multilingual-size.json",
  browser: "browser-smoke.json",
  routing: "language-auto-validation.json",
  behavior: "multilingual-behavior.json",
  smoke: "multilingual-cli-smoke.json",
}, (name) => readReport(resolve(reports, name)));
const benchmark = values.benchmark
  ? readReport(resolve(root, values.benchmark))
  : latestBenchmark(resolve(reports, "benchmarks"));
if (!isFullBenchmark(benchmark.data)) {
  throw new Error("The website requires a retrained full-engine test benchmark");
}
const { performance, sizes, browser, routing, behavior, smoke } = mapValues(inputs, (input) => input.data);
const snapshot = {
  schema_version: 1,
  sources: { benchmark: benchmark.source, ...mapValues(inputs, (input) => input.source) },
  benchmark: benchmark.data,
  performance: {
    ...select(performance, ["computer", "target_triple", "rust_version", "peak_rss_bytes"]),
    fixtures: mapValues(performance.fixtures, (fixture) => select(fixture, ["p95_nanoseconds"])),
  },
  sizes: { artifacts: mapValues(sizes.artifacts, (artifact) => select(artifact, ["bytes"])) },
  browser: {
    ...select(browser, ["supplied_case_count", "passed_case_count", "auto_case_count", "passed_auto_case_count",
      "unknown_case_count", "passed_unknown_case_count", "runtime_network_requests"]),
    engines: browser.engines.map((engine) => select(engine, ["engine", "version", "status"])),
    browser_builds: mapValues(browser.browser_builds,
      (build) => select(build, ["raw_total_bytes", "gzip_total_bytes", "brotli_total_bytes"])),
  },
  routing: {
    ...select(routing, ["computer", "target_triple", "corpus", "supported", "unsupported", "languages",
      "cold_initialization_nanoseconds", "limitations"]),
    timing: { groups: mapValues(routing.timing.groups, (group) => select(group, ["p95_nanoseconds"])) },
  },
  behavior: totals(behavior),
  smoke: {
    ...totals(smoke),
    thresholds: [...new Set(cases(smoke).map((entry) => entry.threshold))],
    languages: mapValues(smoke.languages, (language) => ({ cases: language.cases
      .filter((entry) => entry.suite === "supplied")
      .map((entry) => select(entry, ["text", "expected_nudge"])) })),
  },
};
mkdirSync(dirname(output), { recursive: true });
writeFileSync(output, `${JSON.stringify(snapshot, null, 2)}\n`);
console.log(`snapshot=${relative(root, output)} sources=${Object.keys(snapshot.sources).length}`);
