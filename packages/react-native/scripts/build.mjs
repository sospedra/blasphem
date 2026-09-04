import { execFileSync } from "node:child_process";
import { cpSync, readdirSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const coreSource = resolve(packageRoot, "../javascript-common/src");
const coreCopy = resolve(packageRoot, "src/core");
const distribution = resolve(packageRoot, "dist");

rmSync(distribution, { recursive: true, force: true });
rmSync(coreCopy, { recursive: true, force: true });
cpSync(coreSource, coreCopy, { recursive: true });
const coreFiles = readdirSync(coreCopy).filter((name) => name.endsWith(".ts")).length;
if (coreFiles === 0) throw new Error(`${coreSource} has no TypeScript sources`);

execFileSync("pnpm", ["exec", "tsc", "--project", resolve(packageRoot, "tsconfig.json")], { stdio: "inherit", cwd: packageRoot });
console.log(`status=built core_files=${coreFiles} entries=${readdirSync(distribution).filter((name) => name.endsWith(".js")).join(",")}`);
