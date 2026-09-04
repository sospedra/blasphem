import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { TARGETS, packageName } from "./targets.mjs";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const { version } = JSON.parse(readFileSync(resolve(packageRoot, "package.json"), "utf8"));

for (const target of TARGETS) {
  const directory = resolve(packageRoot, "npm", target.name);
  mkdirSync(directory, { recursive: true });
  const manifest = {
    name: packageName(target),
    version,
    description: `blasphem command-line binary for ${target.name}, with the language data embedded. Blasphem hashes word and character n-grams into sparse feature vectors. A linear classifier trained offline scores them with 16-bit weights. Lexicons and context rules contribute to the verdict. Detection runs locally without neural networks or cloud inference.`,
    license: "Apache-2.0 AND CC-BY-NC-SA-4.0",
    repository: { type: "git", url: "git+https://github.com/sospedra/blasphem.git", directory: `packages/cli/npm/${target.name}` },
    publishConfig: { access: "public", provenance: true },
    os: [target.os],
    cpu: [target.cpu],
    ...(target.libc ? { libc: [target.libc] } : {}),
    files: ["bin", "NOTICE"],
    scripts: { prepack: "node ../../../../scripts/copy-notice.mjs" },
    engines: { node: ">= 20.6" },
  };
  writeFileSync(resolve(directory, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`wrote npm/${target.name}/package.json`);
}
