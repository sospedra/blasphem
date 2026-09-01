import { execFileSync } from "node:child_process";
import { chmodSync, copyFileSync, existsSync, mkdirSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { binaryName, hostTarget } from "./targets.mjs";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(packageRoot, "../..");

const target = hostTarget();
if (!target) throw new Error(`no @blasphem/cli package covers ${process.platform}-${process.arch}`);

// The same command `blasphem-train reproduce` runs, so the two share one artifact.
execFileSync(
  "cargo",
  ["build", "--release", "--locked", "--bin", "blasphem", "--manifest-path", resolve(projectRoot, "Cargo.toml")],
  { stdio: "inherit" },
);

const built = resolve(projectRoot, "target/release", binaryName(target));
if (!existsSync(built)) throw new Error(`cargo did not produce ${built}`);

const directory = resolve(packageRoot, "npm", target.name, "bin");
mkdirSync(directory, { recursive: true });
const destination = resolve(directory, binaryName(target));
copyFileSync(built, destination);
chmodSync(destination, 0o755);
console.log(`status=built target=${target.name} mb=${(statSync(destination).size / 1048576).toFixed(2)} path=npm/${target.name}/bin/${binaryName(target)}`);
