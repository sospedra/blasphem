import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { TARGETS, binaryName, packageName } from "./targets.mjs";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const { version } = JSON.parse(readFileSync(resolve(packageRoot, "package.json"), "utf8"));

for (const target of TARGETS) {
  const directory = resolve(packageRoot, "npm", target.name);
  mkdirSync(directory, { recursive: true });
  const manifest = {
    name: packageName(target),
    version,
    private: true,
    description: `blasphem native engine for ${target.name}`,
    license: "Apache-2.0",
    os: [target.os],
    cpu: [target.cpu],
    ...(target.libc ? { libc: [target.libc] } : {}),
    main: binaryName(target),
    files: [binaryName(target)],
    engines: { node: ">= 20.6" },
  };
  writeFileSync(resolve(directory, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`wrote npm/${target.name}/package.json`);
}
