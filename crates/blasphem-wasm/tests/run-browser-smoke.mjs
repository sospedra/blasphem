import { createHash } from "node:crypto";
import { createReadStream, existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { dirname, extname, resolve, sep } from "node:path";
import { createRequire } from "node:module";
import { brotliCompressSync, constants, gzipSync } from "node:zlib";

const [projectRootArgument, outputArgument, fullOutputArgument, explicitOutputArgument] = process.argv.slice(2);
if (!projectRootArgument || !outputArgument || !fullOutputArgument || !explicitOutputArgument) {
  throw new Error("usage: node run-browser-smoke.mjs PROJECT_ROOT OUTPUT FULL_WEB EXPLICIT_WEB");
}

const projectRoot = resolve(projectRootArgument);
const reportPath = resolve(outputArgument);
const fullOutput = resolve(fullOutputArgument);
const explicitOutput = resolve(explicitOutputArgument);
const wasmPath = resolve(fullOutput, "blasphem_bg.wasm");
const gluePath = resolve(fullOutput, "blasphem.js");
const explicitWasmPath = resolve(explicitOutput, "blasphem_bg.wasm");
const explicitGluePath = resolve(explicitOutput, "blasphem.js");
const require = createRequire(import.meta.url);

const CHROMIUM_HINT = "Run `npx playwright install chromium`.";

function loadPlaywright() {
  try {
    return require("playwright");
  } catch (error) {
    const hint = "Install it with `npm install playwright`.";
    throw new Error(`cannot resolve the playwright module: ${error.message}. ${hint}`);
  }
}

function pinnedChromiumPath(chromium) {
  let path;
  try {
    path = chromium.executablePath();
  } catch (error) {
    throw new Error(`playwright reports no pinned chromium: ${error.message}. ${CHROMIUM_HINT}`);
  }
  if (!path || !existsSync(path)) {
    throw new Error(`playwright has no installed chromium. ${CHROMIUM_HINT}`);
  }
  return path;
}

const { chromium } = loadPlaywright();
const chromePath = pinnedChromiumPath(chromium);

function contentType(path) {
  switch (extname(path)) {
    case ".html": return "text/html; charset=utf-8";
    case ".js": return "text/javascript; charset=utf-8";
    case ".wasm": return "application/wasm";
    default: return "application/octet-stream";
  }
}

function canonicalJson(value) {
  if (value === null || typeof value === "string" || typeof value === "boolean") {
    return JSON.stringify(value);
  }
  if (typeof value === "number") {
    if (!Number.isFinite(value)) {
      throw new Error("canonical JSON rejects non-finite numbers");
    }
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map(canonicalJson).join(",")}]`;
  }
  if (typeof value === "object") {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`)
      .join(",")}}`;
  }
  throw new Error(`canonical JSON rejects ${typeof value}`);
}

function relativePath(path) {
  if (!path.startsWith(`${projectRoot}${sep}`)) {
    throw new Error(`artifact is outside the project root: ${path}`);
  }
  return path.slice(projectRoot.length + 1);
}

function compressedFileRecord(path) {
  const bytes = readFileSync(path);
  return {
    relative_path: relativePath(path),
    sha256: createHash("sha256").update(bytes).digest("hex"),
    raw_bytes: bytes.length,
    gzip_bytes: gzipSync(bytes, { level: 9 }).length,
    brotli_bytes: brotliCompressSync(bytes, {
      params: { [constants.BROTLI_PARAM_QUALITY]: 11 },
    }).length,
  };
}

function browserBuildRecord(wasmFile, glueFile) {
  const wasm = compressedFileRecord(wasmFile);
  const javascriptGlue = compressedFileRecord(glueFile);
  return {
    wasm,
    javascript_glue: javascriptGlue,
    raw_total_bytes: wasm.raw_bytes + javascriptGlue.raw_bytes,
    gzip_total_bytes: wasm.gzip_bytes + javascriptGlue.gzip_bytes,
    brotli_total_bytes: wasm.brotli_bytes + javascriptGlue.brotli_bytes,
  };
}

const server = createServer((request, response) => {
  try {
    const pathname = decodeURIComponent(new URL(request.url, "http://127.0.0.1").pathname);
    const path = resolve(projectRoot, `.${pathname}`);
    if (path !== projectRoot && !path.startsWith(`${projectRoot}${sep}`)) {
      response.writeHead(403).end("forbidden");
      return;
    }
    const stat = statSync(path);
    if (!stat.isFile()) {
      response.writeHead(404).end("not found");
      return;
    }
    response.writeHead(200, {
      "Content-Type": contentType(path),
      "Content-Length": stat.size,
      "Cache-Control": "no-store",
    });
    createReadStream(path).pipe(response);
  } catch {
    response.writeHead(404).end("not found");
  }
});

await new Promise((resolveListen) => server.listen(0, "127.0.0.1", resolveListen));
const address = server.address();
if (!address || typeof address === "string") {
  server.close();
  throw new Error("the browser test server has no TCP address");
}

let browser;
let report;
try {
  browser = await chromium.launch({ executablePath: chromePath, headless: true });
  const page = await browser.newPage();
  await page.goto(
    `http://127.0.0.1:${address.port}/crates/blasphem-wasm/tests/browser-smoke.html`,
    { waitUntil: "load" },
  );
  await page.waitForFunction(
    () => window.__blasphemReport?.status !== "running",
    undefined,
    { timeout: 120_000 },
  );
  report = await page.evaluate(() => window.__blasphemReport);
  report.schema_version = 2;
  report.evidence_status = "experimental";
  report.execution_environment = "actual_browser";
  report.browser_engine = "chromium";
  report.browser_version = browser.version();
  report.user_agent = await page.evaluate(() => navigator.userAgent);
  report.webassembly_available = await page.evaluate(() => typeof WebAssembly === "object");
  report.wasm_bindgen_version = "0.2.127";

  const fullBuild = browserBuildRecord(wasmPath, gluePath);
  const explicitOnlyBuild = browserBuildRecord(explicitWasmPath, explicitGluePath);
  report.wasm = fullBuild.wasm;
  report.javascript_glue = fullBuild.javascript_glue;
  report.browser_builds = {
    full: fullBuild,
    explicit_only: explicitOnlyBuild,
  };
} catch (error) {
  report = {
    schema_version: 2,
    evidence_status: "experimental",
    execution_environment: "actual_browser",
    status: "failed",
    error: String(error?.stack ?? error),
  };
} finally {
  await browser?.close();
  await new Promise((resolveClose) => server.close(resolveClose));
}

mkdirSync(dirname(reportPath), { recursive: true });
writeFileSync(reportPath, canonicalJson(report), "utf8");
if (report.status !== "passed" || report.explicit_only_runtime?.passed !== true) {
  const completedCases = (report.passed_case_count ?? 0)
    + (report.passed_auto_case_count ?? 0)
    + (report.passed_unknown_case_count ?? 0);
  const totalCases = (report.supplied_case_count ?? 0)
    + (report.auto_case_count ?? 0)
    + (report.unknown_case_count ?? 0);
  throw new Error(`browser smoke failed: ${report.error ?? `${completedCases}/${totalCases} cases passed`}`);
}
const totalPassed = report.passed_case_count
  + report.passed_auto_case_count
  + report.passed_unknown_case_count
  + Number(report.explicit_only_runtime.passed);
console.log(`status=passed cases=${totalPassed} raw_bytes=${report.wasm.raw_bytes} gzip_bytes=${report.wasm.gzip_bytes} brotli_bytes=${report.wasm.brotli_bytes}`);
