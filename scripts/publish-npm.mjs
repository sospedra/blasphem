// Publishes every public npm package in one pass.
//
// npm, not pnpm, runs the publish, because npm mints the Trusted Publishing
// OIDC token. npm does not understand pnpm's `workspace:` protocol, so this
// script pins those ranges to the one workspace version first. Re-running is
// safe: a version already on the registry is skipped, so a partial failure
// resumes where it stopped.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { TARGETS, packageName as cliPackage } from "../packages/cli/scripts/targets.mjs";
import { packageName as nodePackage } from "../packages/node/scripts/targets.mjs";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const dryRun = process.argv.includes("--dry-run");
const DEPENDENCY_FIELDS = ["dependencies", "devDependencies", "optionalDependencies", "peerDependencies"];

/** Platform packages first: `blasphem` names them in optionalDependencies. */
function plan() {
  const platforms = TARGETS.flatMap((target) => [
    { name: nodePackage(target), directory: `packages/node/npm/${target.name}` },
    { name: cliPackage(target), directory: `packages/cli/npm/${target.name}` },
  ]);
  return [
    ...platforms,
    { name: "@blasphem/packs", directory: "packages/javascript-packs" },
    { name: "blasphem", directory: "packages/javascript" },
    { name: "@blasphem/react-native", directory: "packages/react-native" },
  ];
}

function manifestPath(entry) {
  return resolve(projectRoot, entry.directory, "package.json");
}

function readManifest(entry) {
  return JSON.parse(readFileSync(manifestPath(entry), "utf8"));
}

/** Rewrites `workspace:*` to the exact version. The registry rejects the protocol. */
function pin(entry, version) {
  const manifest = readManifest(entry);
  let pinned = 0;
  for (const field of DEPENDENCY_FIELDS) {
    const block = manifest[field];
    if (!block) continue;
    for (const [name, range] of Object.entries(block)) {
      if (!range.startsWith("workspace:")) continue;
      block[name] = version;
      pinned += 1;
    }
  }
  if (pinned > 0) writeFileSync(manifestPath(entry), `${JSON.stringify(manifest, null, 2)}\n`);
  return pinned;
}

function published(entry, version) {
  try {
    execFileSync("npm", ["view", `${entry.name}@${version}`, "version"], { stdio: "pipe" });
    return true;
  } catch {
    return false;
  }
}

function publish(entry, tag) {
  const args = ["publish", "--tag", tag, "--access", "public"];
  if (dryRun) args.push("--dry-run");
  execFileSync("npm", args, { stdio: "inherit", cwd: resolve(projectRoot, entry.directory) });
}

const version = JSON.parse(readFileSync(resolve(projectRoot, "packages/javascript/package.json"), "utf8")).version;
const tag = version.includes("-") ? "next" : "latest";
const entries = plan();
let pinnedTotal = 0;
let skipped = 0;

for (const entry of entries) {
  if (!existsSync(manifestPath(entry))) throw new Error(`${entry.directory}/package.json is missing; run the build first`);
  pinnedTotal += pin(entry, version);
}
for (const entry of entries) {
  if (!dryRun && published(entry, version)) {
    console.log(`skip ${entry.name}@${version}, already on the registry`);
    skipped += 1;
    continue;
  }
  publish(entry, tag);
}
console.log(`status=published version=${version} tag=${tag} packages=${entries.length - skipped} skipped=${skipped} pinned=${pinnedTotal}`);
