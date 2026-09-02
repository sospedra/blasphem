import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { TARGETS, binaryName, hostTarget } from "./targets.mjs";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(packageRoot, "../..");
const targetDir = resolve(projectRoot, "target/npm-node");

/** `--target linux-x64-musl` cross-compiles one platform package. Without it the build serves this machine. */
function requested() {
  const index = process.argv.indexOf("--target");
  if (index === -1) return hostTarget();
  const name = process.argv[index + 1];
  const found = TARGETS.find((entry) => entry.name === name);
  if (!found) throw new Error(`unknown target ${name}. Known: ${TARGETS.map((entry) => entry.name).join(", ")}`);
  return found;
}

/** The file cargo writes, named for the target's operating system rather than this machine's. */
function libraryName(os) {
  if (os === "win32") return "blasphem_napi.dll";
  return os === "darwin" ? "libblasphem_napi.dylib" : "libblasphem_napi.so";
}

const target = requested();
if (!target) throw new Error(`no @blasphem/node package covers ${process.platform}-${process.arch}`);

// Always name the triple, so the output path is the same whether the build is native or cross.
execFileSync(
  "cargo",
  ["build", "--release", "--locked", "-p", "blasphem-napi", "--target", target.triple, "--manifest-path", resolve(projectRoot, "Cargo.toml")],
  { stdio: "inherit", env: { ...process.env, CARGO_TARGET_DIR: targetDir } },
);

const built = resolve(targetDir, target.triple, "release", libraryName(target.os));
if (!existsSync(built)) throw new Error(`cargo did not produce ${built}`);

const directory = resolve(packageRoot, "npm", target.name);
mkdirSync(directory, { recursive: true });
const destination = resolve(directory, binaryName(target));
copyFileSync(built, destination);
console.log(`status=built target=${target.name} bytes=${statSync(destination).size} path=npm/${target.name}/${binaryName(target)}`);
