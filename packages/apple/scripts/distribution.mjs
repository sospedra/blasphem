import { execFileSync } from "node:child_process";
import { copyFileSync, cpSync, mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { parseArgs } from "node:util";
import { fileURLToPath } from "node:url";
import { canonicalPacks, copyPack, loadPacks } from "../../../scripts/packs.mjs";

/**
 * Renders the published Swift package: the manifest with the binary target by
 * URL and checksum, the wrapper, build plugin, and installed release resources,
 * LICENSE, NOTICE, README. With --output it stops there. With
 * --repo it clones the distribution repository, replaces its tree, commits
 * `Publish <version>`, pushes main, and pushes the tag v<version>.
 *
 *   node scripts/distribution.mjs --version 2.0.0 --checksum <sha256> --output /tmp/blasphem-swift
 *   node scripts/distribution.mjs --version 2.0.0 --checksum <sha256> --repo git@github.com:sospedra/blasphem-swift.git
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

function artifacts(directory) { return loadPacks(directory).files; }

function manifest(version, checksum) {
  return `// swift-tools-version: 5.9
import PackageDescription
let package = Package(
    name: "Blasphem",
    platforms: [.iOS("15.1"), .macOS(.v12)],
    products: [
        .library(name: "Blasphem", targets: ["Blasphem"]),
        .plugin(name: "BlasphemAssets", targets: ["BlasphemAssets"])
    ],
    targets: [
        .binaryTarget(name: "BlasphemFFI",
            url: "https://github.com/sospedra/blasphem/releases/download/v${version}/BlasphemFFI.xcframework.zip",
            checksum: "${checksum}"),
        .target(name: "Blasphem", dependencies: ["BlasphemFFI"]),
        .executableTarget(name: "BlasphemAssetGenerator", sources: ["main.swift", "Locales.generated.swift"], resources: [.copy("Resources")]),
        .plugin(name: "BlasphemAssets", capability: .buildTool(), dependencies: ["BlasphemAssetGenerator"])
    ]
)
`;
}

function render(tree, entries) {
  rmSync(tree, { recursive: true, force: true });
  mkdirSync(tree, { recursive: true });
  writeFileSync(resolve(tree, "Package.swift"), manifest(options.version, options.checksum));
  cpSync(resolve(packageRoot, "Sources"), resolve(tree, "Sources"), { recursive: true });
  copyFileSync(resolve(packageRoot, "Sources/Blasphem/Locales.generated.swift"), resolve(tree, "Sources/BlasphemAssetGenerator/Locales.generated.swift"));
  writeFileSync(resolve(tree, "Sources/Blasphem/Version.generated.swift"), `let blasphemEngineVersion = ${JSON.stringify(options.version)}\n`);
  cpSync(resolve(packageRoot, "Plugins"), resolve(tree, "Plugins"), { recursive: true });
  const directory = resolve(tree, "Sources/BlasphemAssetGenerator/Resources");
  mkdirSync(directory, { recursive: true });
  for (const file of entries) copyPack(file, resolve(directory, file.name));
  copyFileSync(resolve(options.packs, "manifest.json"), resolve(directory, "manifest.json"));
  copyFileSync(resolve(options.packs, "NOTICE"), resolve(directory, "NOTICE"));
  writeFileSync(resolve(directory, "version.txt"), options.version);
  for (const name of ["LICENSE", "NOTICE"]) copyFileSync(resolve(projectRoot, name), resolve(tree, name));
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
  console.log(`status=published tag=${tag} targets=4`);
}

const entries = artifacts(options.packs);
if (options.output) {
  render(resolve(options.output), entries);
  console.log(`status=rendered output=${resolve(options.output)} targets=4`);
} else {
  publish(entries);
}
