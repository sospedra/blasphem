const { createHash, randomUUID } = require("node:crypto");
const { existsSync, lstatSync, mkdirSync, readFileSync, realpathSync, renameSync, unlinkSync, writeFileSync } = require("node:fs");
const { createRequire } = require("node:module");
const { basename, dirname, resolve } = require("node:path");
const { pathToFileURL } = require("node:url");

const packageRoot = resolve(__dirname, "..");
const packageVersion = require("../package.json").version;

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function pathsFor(platform, projectRoot) {
  if (!["ios", "android"].includes(platform)) throw new Error("platform must be ios or android");
  const project = realpathSync(projectRoot);
  const identity = createHash("sha256").update(project).digest("hex").slice(0, 20);
  const base = resolve(packageRoot, "generated", platform, identity);
  const assetsRoot = platform === "android" ? resolve(base, "assets") : base;
  return { base, assetsRoot, directory: resolve(assetsRoot, "blasphem") };
}

async function selection(projectRoot) {
  const config = readJson(resolve(projectRoot, "package.json")).blasphem;
  const { resolveConfiguration } = await import(pathToFileURL(resolve(packageRoot, "dist/core/configuration.js")));
  return resolveConfiguration(config, packageVersion);
}

function installedPacks() {
  const requireFromLibrary = createRequire(__filename);
  const manifestPath = requireFromLibrary.resolve("@blasphem/packs/package.json");
  const installed = readJson(manifestPath).version;
  if (installed !== packageVersion) {
    throw new Error(`@blasphem/packs version ${installed} does not match @blasphem/react-native ${packageVersion}`);
  }
  return resolve(dirname(manifestPath), "dist");
}

function readVerifiedFile(root, name, manifest) {
  const expected = manifest.files[name];
  if (expected === undefined) throw new Error(`manifest.json does not include ${name}`);
  const bytes = readFileSync(resolve(root, name));
  const digest = createHash("sha256").update(bytes).digest("hex");
  if (bytes.length !== expected.bytes || digest !== expected.sha256) {
    throw new Error(`${name} does not match manifest.json`);
  }
  return [name, bytes];
}

async function selectedFiles(config) {
  const root = installedPacks();
  const manifestBytes = readFileSync(resolve(root, "manifest.json"));
  const { parseManifest } = await import(pathToFileURL(resolve(packageRoot, "dist/core/manifest.js")));
  const manifest = parseManifest(manifestBytes);
  const files = Object.fromEntries(config.files.map(name => {
    const record = manifest.files[name];
    if (!record) throw new Error(`The installed release lacks ${name}`);
    return [name, record];
  }));
  const entries = config.assets === "bundled" ? config.files.map(name => readVerifiedFile(root, name, manifest)) : [];
  const metadata = config.assets === "bundled" ? [["manifest.json", Buffer.from(`${JSON.stringify({ formatVersion: 1, files })}\n`)]] : [];
  const integrity = { bytes: manifestBytes.length, sha256: createHash("sha256").update(manifestBytes).digest("hex") };
  return [
    ...metadata,
    ["bundle.json", Buffer.from(`${JSON.stringify({ ...config, manifest: integrity })}\n`)],
    ["NOTICE", readFileSync(resolve(packageRoot, "NOTICE"))],
    ...entries,
  ];
}

function ownedFile(name) {
  if (typeof name !== "string" || name !== basename(name)) throw new Error("Invalid generated asset name");
  const known = ["manifest.json", "bundle.json", "NOTICE"].includes(name);
  const data = name.endsWith(".pack") || name.endsWith(".detect");
  if (!known && !data) throw new Error(`Invalid generated asset name: ${name}`);
  return name;
}

function previousFiles(base) {
  const marker = resolve(base, "ownership.json");
  if (!existsSync(marker)) return [];
  const files = readJson(marker);
  if (!Array.isArray(files)) throw new Error("Invalid generated asset ownership file");
  return files.map(ownedFile);
}

function writeChanged(path, bytes) {
  if (existsSync(path) && readFileSync(path).equals(bytes)) return;
  const temporary = `${path}.${randomUUID()}.tmp`;
  writeFileSync(temporary, bytes);
  renameSync(temporary, path);
}

function ensureDirectory(directory) {
  mkdirSync(directory, { recursive: true });
  if (lstatSync(directory).isSymbolicLink()) throw new Error("Generated asset directory must not be a symlink");
}

function synchronize(paths, entries) {
  const previous = previousFiles(paths.base);
  const names = entries.map(([name]) => name);
  ensureDirectory(paths.directory);
  for (const [name, bytes] of entries) writeChanged(resolve(paths.directory, name), bytes);
  for (const name of previous.filter(name => !names.includes(name))) {
    const path = resolve(paths.directory, name);
    if (existsSync(path)) unlinkSync(path);
  }
  writeChanged(resolve(paths.base, "ownership.json"), Buffer.from(`${JSON.stringify(names)}\n`));
  return names;
}

async function bundleAssets(platform, projectRoot) {
  const config = await selection(projectRoot);
  const entries = await selectedFiles(config);
  const paths = pathsFor(platform, projectRoot);
  const files = synchronize(paths, entries);
  return { ...paths, ...config, files };
}

async function main(args) {
  const [platform, projectRoot, option] = args;
  if (!projectRoot) throw new Error("Usage: bundle-assets.cjs <ios|android> <project-root> [--paths]");
  const result = option === "--paths" ? pathsFor(platform, projectRoot) : await bundleAssets(platform, projectRoot);
  process.stdout.write(`${JSON.stringify(result)}\n`);
}

module.exports = { bundleAssets, pathsFor };

if (require.main === module) {
  main(process.argv.slice(2)).catch(error => {
    process.stderr.write(`BLASPHEM_ASSETS: ${error.message}\n`);
    process.exitCode = 1;
  });
}
