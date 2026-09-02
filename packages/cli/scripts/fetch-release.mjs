// Fills packages/cli/npm/<target>/bin/ from a published GitHub Release, so the
// npm platform packages ship the same bytes cargo-dist already built and
// checksummed. `dist-manifest.json` names every archive and its target triple.
import { execFileSync } from "node:child_process";
import { chmodSync, copyFileSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { TARGETS, binaryName } from "./targets.mjs";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function flag(name) {
  const index = process.argv.indexOf(`--${name}`);
  if (index === -1) throw new Error(`--${name} is required`);
  return process.argv[index + 1];
}

function download(tag, pattern, into) {
  execFileSync("gh", ["release", "download", tag, "--pattern", pattern, "--dir", into, "--clobber"], { stdio: "inherit" });
}

/** The archive cargo-dist built for one Rust target triple. */
function archiveName(manifest, triple) {
  const entries = Object.values(manifest.artifacts ?? {});
  const found = entries.find((entry) => entry.kind === "executable-zip" && (entry.target_triples ?? []).includes(triple));
  if (!found) throw new Error(`the release has no executable archive for ${triple}`);
  return found.name;
}

function extract(archive, into) {
  mkdirSync(into, { recursive: true });
  if (archive.endsWith(".zip")) {
    execFileSync("unzip", ["-q", "-o", archive, "-d", into], { stdio: "inherit" });
    return;
  }
  execFileSync("tar", ["-xf", archive, "-C", into], { stdio: "inherit" });
}

/** cargo-dist has shipped the executable both at the archive root and one directory down. */
function findBinary(root, name) {
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const path = join(root, entry.name);
    if (entry.isDirectory()) {
      const nested = findBinary(path, name);
      if (nested) return nested;
      continue;
    }
    if (entry.name === name) return path;
  }
  return null;
}

function install(tag, manifest, target, workspace) {
  const archive = archiveName(manifest, target.triple);
  download(tag, archive, workspace);
  const unpacked = resolve(workspace, target.name);
  extract(resolve(workspace, archive), unpacked);
  const executable = binaryName(target);
  const built = findBinary(unpacked, executable);
  if (!built) throw new Error(`${archive} holds no ${executable}`);
  const directory = resolve(packageRoot, "npm", target.name, "bin");
  mkdirSync(directory, { recursive: true });
  const destination = resolve(directory, executable);
  copyFileSync(built, destination);
  chmodSync(destination, 0o755);
  return statSync(destination).size;
}

const tag = flag("tag");
const workspace = mkdtempSync(resolve(tmpdir(), "blasphem-release-"));
download(tag, "dist-manifest.json", workspace);
const manifest = JSON.parse(readFileSync(resolve(workspace, "dist-manifest.json"), "utf8"));
let total = 0;
for (const target of TARGETS) total += install(tag, manifest, target, workspace);
rmSync(workspace, { recursive: true, force: true });
console.log(`status=fetched tag=${tag} targets=${TARGETS.length} total_mb=${(total / 1048576).toFixed(2)}`);
