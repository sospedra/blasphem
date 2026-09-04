#!/usr/bin/env node
// Runs the blasphem command-line binary from the platform package for this
// machine. `npx blasphem judge "text"` reaches it because the package name and
// the bin name match. The binary embeds every locale, so it needs no packs.
import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

const require = createRequire(import.meta.url);

function libc() {
  const report = process.report?.getReport?.();
  return typeof report?.header?.glibcVersionRuntime === "string" ? "gnu" : "musl";
}

/** The platform key the binary is published under, or null when none exists. */
function target() {
  const { platform, arch } = process;
  if (platform === "darwin" && (arch === "arm64" || arch === "x64")) return `darwin-${arch}`;
  if (platform === "linux" && (arch === "arm64" || arch === "x64")) return `linux-${arch}-${libc()}`;
  if (platform === "win32" && arch === "x64") return "win32-x64-msvc";
  return null;
}

const name = target();
if (name === null) {
  console.error(`no @blasphem/cli package covers ${process.platform}-${process.arch}. Download a binary from https://github.com/sospedra/blasphem/releases`);
  process.exit(2);
}

let manifest;
try {
  manifest = require.resolve(`@blasphem/cli-${name}/package.json`);
} catch {
  console.error(`@blasphem/cli-${name} is not installed. Reinstall blasphem, or download a binary from https://github.com/sospedra/blasphem/releases`);
  process.exit(2);
}

const binary = join(dirname(manifest), "bin", process.platform === "win32" ? "blasphem.exe" : "blasphem");
const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(`cannot run ${binary}: ${result.error.message}`);
  process.exit(2);
}
if (result.signal) {
  process.kill(process.pid, result.signal);
}
process.exit(result.status ?? 2);
