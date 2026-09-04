import { createHash } from "node:crypto";
import { createReadStream, existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { dirname, extname, resolve, sep } from "node:path";
import { brotliCompressSync, constants, gzipSync } from "node:zlib";
import { AUTO_CASES, SUPPLIED_CASES, UNKNOWN_CASES, failures } from "../tests/cases.mjs";
import { packageRoot, projectRoot, readCrate } from "./crate.mjs";
import { assertWasmBindgen } from "./wasm.mjs";

const require = createRequire(import.meta.url);
const ENGINES = ["chromium", "webkit"];
const SCHEMA_VERSION = 4;
const PAGE_TIMEOUT_MS = 180_000;
const INSTALL_HINT = "Run: pnpm --filter blasphem exec playwright install chromium webkit";
const WASM = "blasphem_bg.wasm";
const GLUE = "blasphem.js";

const distribution = resolve(packageRoot, "dist");
const packs = resolve(projectRoot, "resources/packs");
const reportPath = resolve(projectRoot, "reports/browser-smoke.json");
const ROUTES = { "/dist/": distribution, "/tests/": resolve(packageRoot, "tests"), "/assets/": packs, "/packs-only/": packs };
const JSDELIVR = "https://cdn.jsdelivr.net";
// The policy the README documents, plus 'unsafe-inline' for this test page's own inline module.
// The library needs only 'wasm-unsafe-eval' in script-src and its asset origins in connect-src.
const CSP = `default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; connect-src 'self' ${JSDELIVR}; style-src 'none'; img-src 'none'; object-src 'none'; base-uri 'none'`;

function loadPlaywright() {
  try {
    return { playwright: require("playwright"), version: require("playwright/package.json").version };
  } catch (error) {
    throw new Error(`cannot resolve playwright: ${error.message}. Run: pnpm install --frozen-lockfile`);
  }
}

function contentType(path) {
  switch (extname(path)) {
    case ".html": return "text/html; charset=utf-8";
    case ".js": case ".mjs": return "text/javascript; charset=utf-8";
    case ".wasm": return "application/wasm";
    case ".json": return "application/json";
    default: return "application/octet-stream";
  }
}

function canonicalJson(value) {
  if (value === null || typeof value === "string" || typeof value === "boolean") return JSON.stringify(value);
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new Error("canonical JSON rejects non-finite numbers");
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (typeof value === "object") {
    const entries = Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`);
    return `{${entries.join(",")}}`;
  }
  throw new Error(`canonical JSON rejects ${typeof value}`);
}

function relativePath(path) {
  if (!path.startsWith(`${projectRoot}${sep}`)) throw new Error(`artifact is outside the project root: ${path}`);
  return path.slice(projectRoot.length + 1);
}

function compressedFileRecord(path) {
  const bytes = readFileSync(path);
  return {
    relative_path: relativePath(path),
    sha256: createHash("sha256").update(bytes).digest("hex"),
    raw_bytes: bytes.length,
    gzip_bytes: gzipSync(bytes, { level: 9 }).length,
    brotli_bytes: brotliCompressSync(bytes, { params: { [constants.BROTLI_PARAM_QUALITY]: 11 } }).length,
  };
}

function totals(records) {
  const sum = (key) => records.reduce((total, record) => total + record[key], 0);
  return { raw_total_bytes: sum("raw_bytes"), gzip_total_bytes: sum("gzip_bytes"), brotli_total_bytes: sum("brotli_bytes") };
}

/** One download profile: the code plus the named pack files. */
function browserBuild(wasm, glue, packRecords) {
  return { wasm, javascript_glue: glue, packs: packRecords.map((record) => record.relative_path), ...totals([wasm, glue, ...packRecords]) };
}

function clientRecord() {
  const names = ["browser.js", "wasm-engine.js", ...readdirSync(resolve(distribution, "core")).filter((name) => name.endsWith(".js")).map((name) => `core/${name}`)];
  const files = Object.fromEntries(names.map((name) => [name, compressedFileRecord(resolve(distribution, name))]));
  return { files, ...totals(Object.values(files)) };
}

function assertInputs() {
  const missing = ["browser.js", GLUE, WASM].filter((name) => !existsSync(resolve(distribution, name)));
  if (missing.length > 0) throw new Error(`dist/ lacks ${missing.join(", ")}. Run: pnpm --filter blasphem run build`);
  if (!existsSync(resolve(packs, "manifest.json"))) throw new Error(`${packs} lacks manifest.json. Run: pnpm --filter @blasphem/packs run build`);
}

function serve(request, response) {
  const pathname = decodeURIComponent(new URL(request.url, "http://127.0.0.1").pathname);
  if (pathname === `/assets/${WASM}` || pathname === `/wasm-only/${WASM}`) {
    return sendFile(response, resolve(distribution, WASM));
  }
  const prefix = Object.keys(ROUTES).find((candidate) => pathname.startsWith(candidate));
  if (!prefix) {
    response.writeHead(404).end("not found");
    return;
  }
  const root = ROUTES[prefix];
  const path = resolve(root, `.${pathname.slice(prefix.length - 1)}`);
  if (!path.startsWith(`${root}${sep}`) || !existsSync(path) || !statSync(path).isFile()) {
    response.writeHead(404).end("not found");
    return;
  }
  sendFile(response, path);
}

function sendFile(response, path) {
  const headers = {
    "Content-Type": contentType(path),
    "Content-Length": statSync(path).size,
    "Cache-Control": "no-store",
  };
  if (extname(path) === ".html") headers["Content-Security-Policy"] = CSP;
  response.writeHead(200, headers);
  createReadStream(path).pipe(response);
}

async function runEngine(playwright, engine, origin) {
  const record = { engine, status: "failed", console_errors: [] };
  let browser;
  try {
    browser = await playwright[engine].launch({ headless: true });
  } catch (error) {
    record.error = `${engine} did not launch: ${error.message}. ${INSTALL_HINT}`;
    return record;
  }
  try {
    record.version = browser.version();
    const page = await browser.newPage();
    // The jsDelivr preset resolves to https://cdn.jsdelivr.net/npm/<package>@<version>/dist/<file>.
    // Serve those URLs from the local build so the smoke checks URL construction without the network.
    await page.route(`${JSDELIVR}/**`, (route) => {
      const url = new URL(route.request().url());
      const match = /^\/npm\/(blasphem|@blasphem\/packs)@[^/]+\/dist\/(.+)$/.exec(url.pathname);
      if (!match) return route.fulfill({ status: 404, body: "not found" });
      const root = match[1] === "blasphem" ? distribution : packs;
      const path = resolve(root, match[2]);
      if (!path.startsWith(`${root}${sep}`) || !existsSync(path)) return route.fulfill({ status: 404, body: "not found" });
      return route.fulfill({ status: 200, contentType: contentType(path), body: readFileSync(path), headers: { "Access-Control-Allow-Origin": "*" } });
    });
    page.on("console", (message) => {
      if (message.type() === "error") record.console_errors.push(message.text());
    });
    page.on("pageerror", (error) => record.console_errors.push(String(error)));
    await page.goto(`${origin}/tests/smoke.html`, { waitUntil: "load" });
    await page.waitForFunction(() => window.__blasphemReport?.status !== "running", undefined, { timeout: PAGE_TIMEOUT_MS });
    const pageReport = await page.evaluate(() => window.__blasphemReport);
    record.user_agent = await page.evaluate(() => navigator.userAgent);
    record.webassembly_available = await page.evaluate(() => typeof WebAssembly === "object");
    Object.assign(record, pageReport);
    record.status = pageReport.status === "passed" && record.console_errors.length === 0 ? "passed" : "failed";
  } catch (error) {
    record.status = "failed";
    record.error = String(error?.stack ?? error);
  } finally {
    await browser.close();
  }
  return record;
}

/** Counts the entries of one list that passed in every engine. */
function passedEverywhere(engines, listKey) {
  const [first, ...rest] = engines;
  if (!first?.[listKey]) return 0;
  const key = (entry) => entry.case_id ?? entry.text;
  const passedIn = (engine, entry) => engine[listKey]?.some((candidate) => key(candidate) === key(entry) && candidate.passed) === true;
  return first[listKey].filter((entry) => entry.passed && rest.every((engine) => passedIn(engine, entry))).length;
}

function describeFailure(engine) {
  const lines = [`${engine.engine}: ${engine.status}`];
  if (engine.error) lines.push(`  error: ${engine.error}`);
  for (const message of engine.console_errors) lines.push(`  console: ${message}`);
  for (const entry of failures(engine)) lines.push(`  case: ${JSON.stringify(entry)}`);
  if (engine.en_only_passed === false) lines.push(`  en-only requests: ${JSON.stringify(engine.en_only_requests)}`);
  if (engine.split_assets_passed === false) lines.push("  split assets: failed");
  if (engine.jsdelivr_passed === false) lines.push(`  jsdelivr requests: ${JSON.stringify(engine.jsdelivr_requests)}`);
  if (engine.runtime_network_requests?.length) lines.push(`  network during judge(): ${engine.runtime_network_requests.join(", ")}`);
  return lines.join("\n");
}

const { playwright, version: playwrightVersion } = loadPlaywright();
const crate = readCrate();
const manifest = JSON.parse(readFileSync(resolve(packageRoot, "package.json"), "utf8"));
assertWasmBindgen(crate.wasmBindgenVersion);
assertInputs();

const server = createServer((request, response) => {
  try {
    serve(request, response);
  } catch {
    response.writeHead(404).end("not found");
  }
});
await new Promise((listening) => server.listen(0, "127.0.0.1", listening));
const address = server.address();
if (!address || typeof address === "string") {
  server.close();
  throw new Error("the smoke server has no TCP address");
}
const origin = `http://127.0.0.1:${address.port}`;

const engines = [];
try {
  for (const engine of ENGINES) engines.push(await runEngine(playwright, engine, origin));
} finally {
  await new Promise((closed) => server.close(closed));
}

const wasm = compressedFileRecord(resolve(distribution, WASM));
const glue = compressedFileRecord(resolve(distribution, GLUE));
const packsManifest = JSON.parse(readFileSync(resolve(packs, "manifest.json"), "utf8"));
const packRecords = Object.keys(packsManifest.files).toSorted().map((name) => compressedFileRecord(resolve(packs, name)));
const englishOnly = packRecords.filter((record) => record.relative_path.endsWith("/en.pack"));
const englishRouted = packRecords.filter((record) => /\/en\.(pack|detect)$/.test(record.relative_path));
const status = engines.length === ENGINES.length && engines.every((engine) => engine.status === "passed") ? "passed" : "failed";
const report = {
  schema_version: SCHEMA_VERSION,
  evidence_status: "experimental",
  execution_environment: "actual_browser",
  status,
  package: { name: manifest.name, version: manifest.version, entry: "dist/browser.js" },
  playwright_version: playwrightVersion,
  wasm_bindgen_version: crate.wasmBindgenVersion,
  content_security_policy: CSP,
  engines,
  supplied_case_count: SUPPLIED_CASES.length,
  passed_case_count: passedEverywhere(engines, "cases"),
  auto_case_count: AUTO_CASES.length,
  passed_auto_case_count: passedEverywhere(engines, "auto_cases"),
  unknown_case_count: UNKNOWN_CASES.length,
  passed_unknown_case_count: passedEverywhere(engines, "unknown_cases"),
  package_case_count: engines[0]?.package_case_count ?? 0,
  passed_package_case_count: passedEverywhere(engines, "package_cases"),
  en_only_requests: engines[0]?.en_only_requests ?? [],
  en_only_passed: engines.every((engine) => engine.en_only_passed === true),
  split_assets_passed: engines.every((engine) => engine.split_assets_passed === true),
  jsdelivr_requests: engines[0]?.jsdelivr_requests ?? [],
  jsdelivr_passed: engines.every((engine) => engine.jsdelivr_passed === true),
  runtime_network_requests: [...new Set(engines.flatMap((engine) => engine.runtime_network_requests ?? []))],
  wasm,
  javascript_glue: glue,
  client: clientRecord(),
  packs: { files: Object.fromEntries(packRecords.map((record) => [record.relative_path.split("/").at(-1), record])), ...totals(packRecords) },
  browser_builds: {
    full: browserBuild(wasm, glue, packRecords),
    explicit_only: browserBuild(wasm, glue, englishOnly),
    english_routed: browserBuild(wasm, glue, englishRouted),
  },
};

mkdirSync(dirname(reportPath), { recursive: true });
writeFileSync(reportPath, canonicalJson(report), "utf8");

if (status !== "passed") {
  console.error(`status=failed report=${relativePath(reportPath)}`);
  for (const engine of engines.filter((entry) => entry.status !== "passed")) console.error(describeFailure(engine));
  process.exit(1);
}
const cases = report.passed_case_count + report.passed_auto_case_count + report.passed_unknown_case_count + report.passed_package_case_count;
const versions = engines.map((engine) => `${engine.engine} ${engine.version}`).join(", ");
console.log(`status=passed engines="${versions}" cases=${cases} en_only_brotli_bytes=${report.browser_builds.english_routed.brotli_total_bytes} full_brotli_bytes=${report.browser_builds.full.brotli_total_bytes} report=${relativePath(reportPath)}`);
