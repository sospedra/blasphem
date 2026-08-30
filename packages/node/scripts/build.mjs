import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { binaryName, hostTarget } from "./targets.mjs";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(packageRoot, "../..");
const targetDir = resolve(projectRoot, "target/npm-node");

const target = hostTarget();
if (!target) throw new Error(`no @blasphem/node package covers ${process.platform}-${process.arch}`);

execFileSync(
  "cargo",
  ["build", "--release", "--locked", "-p", "blasphem-napi", "--manifest-path", resolve(projectRoot, "Cargo.toml")],
  { stdio: "inherit", env: { ...process.env, CARGO_TARGET_DIR: targetDir } },
);

const libraryName = process.platform === "win32" ? "blasphem_napi.dll" : process.platform === "darwin" ? "libblasphem_napi.dylib" : "libblasphem_napi.so";
const built = resolve(targetDir, "release", libraryName);
if (!existsSync(built)) throw new Error(`cargo did not produce ${built}`);

const directory = resolve(packageRoot, "npm", target.name);
mkdirSync(directory, { recursive: true });
const destination = resolve(directory, binaryName(target));
copyFileSync(built, destination);
console.log(`status=built target=${target.name} bytes=${statSync(destination).size} path=npm/${target.name}/${binaryName(target)}`);
