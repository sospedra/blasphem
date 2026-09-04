import { execFileSync } from "node:child_process";
import { copyFileSync, cpSync, mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, extname, resolve } from "node:path";
import { parseArgs } from "node:util";
import { fileURLToPath } from "node:url";
import { canonicalPacks, copyPack, loadPacks } from "../../../scripts/packs.mjs";

/**
 * Renders the published Swift package: the manifest with the binary target by
 * URL and checksum, the wrapper sources, one resource target per .pack and
 * .detect file, LICENSE, NOTICE, README. With --output it stops there. With
 * --repo it clones the distribution repository, replaces its tree, commits
 * `Publish <version>`, pushes main, and pushes the tag v<version>.
 *
 *   node scripts/distribution.mjs --version 0.1.0 --checksum <sha256> --output /tmp/blasphem-swift
 *   node scripts/distribution.mjs --version 0.1.0 --checksum <sha256> --repo git@github.com:sospedra/blasphem-swift.git
 */
const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(packageRoot, "../..");

const { values: options } = parseArgs({
  options: {
    version: { type: "string" },
    checksum: { type: "string" },
    packs: { type: "string", default: canonicalPacks },
    output: { type: "string" },
    repo: { type: "string" },
  },
});
if (!options.version || !options.checksum) throw new Error("--version and --checksum are required");
if (!options.output === !options.repo) throw new Error("pass exactly one of --output or --repo");

const KINDS = { pack: "BlasphemPack", detect: "BlasphemDetect" };

function targetName(kind, code) {
  return `${KINDS[kind]}${code.toUpperCase()}`;
}

function artifacts(directory) {
  const files = loadPacks(directory).files;
  const entries = files.map((file) => ({ ...file, kind: extname(file.name).slice(1), code: file.name.slice(0, file.name.lastIndexOf(".")) }));
  return Object.keys(KINDS).flatMap((kind) => entries.filter((entry) => entry.kind === kind));
}

function manifest(version, checksum, entries) {
  const url = `https://github.com/sospedra/blasphem/releases/download/v${version}/BlasphemFFI.xcframework.zip`;
  const resources = entries.map(({ name, kind, code }) => `        resource("${targetName(kind, code)}", "${name}"),`);
  const products = entries.map(({ kind, code }) => `        .library(name: "${targetName(kind, code)}", targets: ["${targetName(kind, code)}"]),`);
  return `// swift-tools-version: 5.9
// Rendered by packages/apple/scripts/distribution.mjs in sospedra/blasphem for ${version}. Do not edit.
import PackageDescription

/// One target per data file. The app links the products it ships; Xcode copies their resource bundles.
func resource(_ name: String, _ file: String) -> Target {
    .target(name: name, path: "Sources/\\(name)", resources: [.copy("Resources/\\(file)")])
}

let package = Package(
    name: "Blasphem",
    platforms: [.iOS("15.1"), .macOS(.v12)],
    products: [
        .library(name: "Blasphem", targets: ["Blasphem"]),
${products.join("\n")}
    ],
    targets: [
        .binaryTarget(
            name: "BlasphemFFI",
            url: "${url}",
            checksum: "${checksum}"
        ),
        .target(name: "Blasphem", dependencies: ["BlasphemFFI"]),
${resources.join("\n")}
    ]
)
`;
}

function render(tree, entries) {
  rmSync(tree, { recursive: true, force: true });
  mkdirSync(tree, { recursive: true });
  writeFileSync(resolve(tree, "Package.swift"), manifest(options.version, options.checksum, entries));
  cpSync(resolve(packageRoot, "Sources/Blasphem"), resolve(tree, "Sources/Blasphem"), { recursive: true });
  for (const file of entries) {
    const { name, kind, code } = file;
    const target = targetName(kind, code);
    const directory = resolve(tree, "Sources", target);
    mkdirSync(resolve(directory, "Resources"), { recursive: true });
    writeFileSync(resolve(directory, `${target}.swift`), `/// Carries Resources/${name}. A target needs a source; nothing references this type.\npublic enum ${target} {}\n`);
    copyPack(file, resolve(directory, "Resources", name));
  }
  copyFileSync(resolve(projectRoot, "LICENSE"), resolve(tree, "LICENSE"));
  copyFileSync(resolve(projectRoot, "NOTICE"), resolve(tree, "NOTICE"));
  copyFileSync(resolve(packageRoot, "README.md"), resolve(tree, "README.md"));
}

function git(cwd, args) {
  return execFileSync("git", args, { cwd, stdio: ["ignore", "pipe", "inherit"], encoding: "utf8" }).trim();
}

function tagExists(cwd, tag) {
  try {
    git(cwd, ["ls-remote", "--exit-code", "--tags", "origin", `refs/tags/${tag}`]);
    return true;
  } catch {
    return false;
  }
}

function publish(entries) {
  const tag = `v${options.version}`;
  const clone = mkdtempSync(resolve(tmpdir(), "blasphem-swift-"));
  git(clone, ["clone", "--depth", "1", options.repo, "."]);
  if (tagExists(clone, tag)) {
    console.log(`status=exists tag=${tag}`);
    return;
  }
  for (const entry of readdirSync(clone).filter((name) => name !== ".git")) rmSync(resolve(clone, entry), { recursive: true, force: true });
  const staging = mkdtempSync(resolve(tmpdir(), "blasphem-swift-tree-"));
  render(staging, entries);
  cpSync(staging, clone, { recursive: true });
  const identity = ["-c", "user.name=blasphem publish", "-c", "user.email=publish@blasphem.sospedra.me"];
  git(clone, ["checkout", "-B", "main"]);
  git(clone, ["add", "--all"]);
  git(clone, [...identity, "commit", "--allow-empty", "--message", `Publish ${options.version}`]);
  git(clone, ["push", "origin", "main"]);
  git(clone, [...identity, "tag", "--annotate", tag, "--message", `Blasphem ${options.version}`]);
  git(clone, ["push", "origin", tag]);
  console.log(`status=published tag=${tag} targets=${entries.length + 2}`);
}

const entries = artifacts(options.packs);
if (options.output) {
  render(resolve(options.output), entries);
  console.log(`status=rendered output=${resolve(options.output)} targets=${entries.length + 2}`);
} else {
  publish(entries);
}
