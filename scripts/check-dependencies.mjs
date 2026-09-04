import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { basename, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parse as parseToml } from "smol-toml";
import { parse as parseYaml } from "yaml";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const exactVersion = /^\d+\.\d+\.\d+(?:-[\w.-]+)?(?:\+[\w.-]+)?$/;
const commitHash = /^[a-f0-9]{40}$/;
const cargoSections = new Set(["dependencies", "dev-dependencies", "build-dependencies"]);
const npmSections = ["dependencies", "devDependencies", "optionalDependencies", "peerDependencies"];
const pythonName = /^[\w.-]+(?:\[[\w.,-]+\])?$/;
const pythonVersion = /^\d[\w.!+-]*$/;
const goRequirement = /(?:^|\n)\s*(?:require\s+)?([\w./~+-]+)\s+(v[^\s)]+)/g;
const gradleCoordinate = /["']([\w.-]+:[\w.-]+):([^"']+)["']/g;
const gradleVersion = /\b(?:id|kotlin)(?:\([^\n)]*\)|\s+["'][^"'\n]+["'])\s+version\s+["']([^"'\n]+)["']/g;
const shellContinuation = /\\\r?\n\s*/g;
const npmInstall = /\bnpm install\s+([^\n]+)/g;
const cargoInstall = /\bcargo install\s+([^\n]+)/g;
const pipInstall = /\bpip install\s+([^\n]+)/g;
const versionFlag = /--version(?:=|\s+)["']?([^\s"']+)/;
const shellWord = /"[^"]*"|'[^']*'|\S+/g;
const envExpression = /\$\{\{\s*env\.(\w+)\s*\}\}/g;

function records(value) {
  if (value === null || typeof value !== "object") return [];
  return [value, ...Object.values(value).flatMap(records)];
}

function violations(file, entries, accepts) {
  return entries.filter(([, value]) => !accepts(value))
    .map(([name, value]) => `${file}: ${name} must be pinned, found ${JSON.stringify(value)}`);
}

function exactNpm(requirement, packageNames) {
  const [name, version] = requirement;
  if (version === "workspace:*") return packageNames.has(name);
  return exactVersion.test(version);
}

function checkNpm(file, manifest, packageNames) {
  const requirements = npmSections.flatMap(section => Object.entries(manifest[section] ?? {}));
  const overrides = records(manifest.overrides).flatMap(record => Object.entries(record)
    .filter(([, value]) => typeof value === "string"));
  return violations(file, [...requirements, ...overrides].map(entry => [entry[0], entry]), value => exactNpm(value, packageNames));
}

function exactCargo(dependency) {
  if (typeof dependency === "string") return dependency.startsWith("=") && exactVersion.test(dependency.slice(1));
  if (dependency.workspace === true) return true;
  if (dependency.git) return commitHash.test(dependency.rev ?? "");
  if (dependency.version) return exactCargo(dependency.version);
  return typeof dependency.path === "string";
}

function checkCargo(file, manifest) {
  const tables = records(manifest).flatMap(record => Object.entries(record)
    .filter(([key]) => cargoSections.has(key)).map(([, value]) => value));
  const overrides = [...Object.values(manifest.patch ?? {}), manifest.replace ?? {}];
  return violations(file, [...tables, ...overrides].flatMap(Object.entries), exactCargo);
}

function exactPython(requirement) {
  if (typeof requirement !== "string") return false;
  const parts = requirement.split(";")[0].trim().split("==");
  if (parts.length !== 2) return false;
  return pythonName.test(parts[0].trim()) && pythonVersion.test(parts[1].trim());
}

function checkPython(file, manifest) {
  const requirements = [
    ...(manifest["build-system"]?.requires ?? []),
    ...(manifest.project?.dependencies ?? []),
    ...Object.values(manifest.project?.["optional-dependencies"] ?? {}).flat(),
    ...Object.values(manifest["dependency-groups"] ?? {}).flat().filter(value => typeof value === "string"),
  ];
  return violations(file, requirements.map(value => ["requirement", value]), exactPython);
}

function requirementLines(source) {
  return source.split("\n").map(line => line.trim()).filter(line => {
    const annotation = line.startsWith("#") || line.startsWith("--hash=");
    return line.length > 0 && !annotation;
  }).map(line => line.replace(/\s*\\$/, ""));
}

function checkRequirements(file, source) {
  return violations(file, requirementLines(source).map(value => ["requirement", value]), exactPython);
}

function exactAction(reference) {
  if (reference.startsWith("./")) return true;
  if (reference.startsWith("docker://")) return /@sha256:[a-f0-9]{64}$/.test(reference);
  return commitHash.test(reference.split("@")[1] ?? "");
}

function words(command) {
  return [...command.matchAll(shellWord)].map(([word]) => word.replace(/^["']|["']$/g, ""));
}

function checkInstall(file, source) {
  const npm = [...source.matchAll(npmInstall)].flatMap(([, command]) => words(command)
    .filter(word => !word.startsWith("-")));
  const cargo = [...source.matchAll(cargoInstall)].map(([, command]) => command);
  return [
    ...violations(file, npm.map(value => ["npm install", value]), value => exactVersion.test(value.slice(value.lastIndexOf("@") + 1))),
    ...violations(file, cargo.map(value => ["cargo install", value]), command =>
      command.includes("--locked") && exactVersion.test(command.match(versionFlag)?.[1] ?? "")),
  ];
}

function pinnedPip(command) {
  const tokens = words(command);
  if (tokens.includes("--no-index")) return true;
  if (tokens.includes("--require-hashes")) return true;
  const requirements = tokens.filter(word => !word.startsWith("-"));
  return requirements.every(requirement => {
    if (requirement.startsWith("./")) return command.includes("--no-build-isolation");
    return exactPython(requirement.replaceAll("$VERSION", "0.0.0"));
  });
}

function checkWorkflow(file, workflow) {
  const nodes = records(workflow);
  const actions = nodes.filter(node => typeof node.uses === "string").map(node => ["uses", node.uses]);
  const runs = nodes.filter(node => typeof node.run === "string").map(node => node.run).join("\n")
    .replace(shellContinuation, " ").replace(envExpression, (match, name) => workflow.env?.[name] ?? match);
  const pip = [...runs.matchAll(pipInstall)].map(([, command]) => ["pip install", command]);
  const maturin = nodes.filter(node => node.uses?.startsWith("PyO3/maturin-action@"))
    .map(node => ["maturin-version", String(node.with?.["maturin-version"] ?? "")
      .replace(envExpression, (match, name) => workflow.env?.[name] ?? match)]);
  return [
    ...violations(file, actions, exactAction),
    ...violations(file, maturin, value => exactVersion.test(value)),
    ...violations(file, pip, pinnedPip),
    ...checkInstall(file, runs),
  ];
}

function checkGradle(file, source) {
  const coordinates = [...source.matchAll(gradleCoordinate)].map(([, name, version]) => [name, version]);
  const plugins = [...source.matchAll(gradleVersion)].map(([, version]) => ["plugin version", version]);
  return violations(file, [...coordinates, ...plugins], version => exactVersion.test(version) && !version.endsWith("-SNAPSHOT"));
}

function checkPnpm(file, workspace, packageNames) {
  const defaults = records(workspace).filter(record => Object.hasOwn(record, "saveExact"))
    .map(record => ["saveExact", record.saveExact]);
  const catalogs = [workspace.catalog ?? {}, workspace.overrides ?? {}, ...Object.values(workspace.catalogs ?? {})];
  return [
    ...violations(file, [["saveExact", workspace.saveExact], ...defaults], value => value === true),
    ...catalogs.flatMap(catalog => checkNpm(file, { dependencies: catalog }, packageNames)),
  ];
}

export function checkManifest(file, source, packageNames = new Set()) {
  const name = basename(file);
  switch (name) {
    case "package.json": return checkNpm(file, JSON.parse(source), packageNames);
    case "Cargo.toml": return checkCargo(file, parseToml(source));
    case "pyproject.toml": return checkPython(file, parseToml(source));
    case "pnpm-workspace.yaml": return checkPnpm(file, parseYaml(source), packageNames);
    case "go.mod": return violations(file, [...source.matchAll(goRequirement)]
      .map(([, module, version]) => [module, version]), version => exactVersion.test(version.slice(1)));
    case "build.gradle":
    case "build.gradle.kts": return checkGradle(file, source);
    case "dist-workspace.toml": return violations(file, Object.entries(parseToml(source).dist?.["github-action-commits"] ?? {}), value => commitHash.test(value));
    default: break;
  }
  if (name.startsWith("requirements") && [".in", ".txt"].some(extension => name.endsWith(extension))) return checkRequirements(file, source);
  if (file.startsWith(".github/") && [".yml", ".yaml"].some(extension => file.endsWith(extension))) return checkWorkflow(file, parseYaml(source));
  return [];
}

function missingLocks() {
  const locks = ["Cargo.lock", "crates/blasphem-python/Cargo.lock", "pnpm-lock.yaml", "packages/go/go.sum", "packages/python/requirements-build.txt"];
  return locks.filter(file => !existsSync(resolve(projectRoot, file))).map(file => `${file}: lockfile is missing`);
}

function checkPythonLock() {
  const input = requirementLines(readFileSync(resolve(projectRoot, "packages/python/requirements-build.in"), "utf8"));
  const locked = requirementLines(readFileSync(resolve(projectRoot, "packages/python/requirements-build.txt"), "utf8"));
  const backends = ["packages/python/pyproject.toml", "packages/python-packs/pyproject.toml"]
    .flatMap(file => parseToml(readFileSync(resolve(projectRoot, file), "utf8"))["build-system"].requires);
  return [...input, ...backends].filter(requirement => !locked.includes(requirement))
    .map(requirement => `packages/python/requirements-build.txt: missing ${requirement}`);
}

function main() {
  const paths = execFileSync("git", ["ls-files", "--cached", "--others", "--exclude-standard", "--deduplicate", "-z"], { cwd: projectRoot, encoding: "utf8" })
    .split("\0").filter(file => file.length > 0 && existsSync(resolve(projectRoot, file)));
  const manifests = paths.filter(file => basename(file) === "package.json");
  const packageNames = new Set(manifests.map(file => JSON.parse(readFileSync(resolve(projectRoot, file), "utf8")).name));
  const candidates = paths.filter(file => /(?:\.toml|\.gradle(?:\.kts)?|\.ya?ml|\/go\.mod|package\.json|requirements[^/]*\.(?:in|txt))$/.test(file));
  const errors = [...missingLocks(), ...candidates.flatMap(file => checkManifest(file, readFileSync(resolve(projectRoot, file), "utf8"), packageNames))];
  if (errors.length > 0) {
    console.error(errors.join("\n"));
    process.exitCode = 1;
    return;
  }
  const lockErrors = checkPythonLock();
  if (lockErrors.length > 0) {
    console.error(lockErrors.join("\n"));
    process.exitCode = 1;
    return;
  }
  console.log(`status=passed dependency_pins manifests=${candidates.length}`);
}

if (resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) main();
