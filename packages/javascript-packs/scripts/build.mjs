import { writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { exportPacks, loadPacks } from "../../../scripts/packs.mjs";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = resolve(packageRoot, "dist");
const packs = loadPacks();
exportPacks(output, packs);

const entries = packs.files.map(({ name }) =>
  `  ${JSON.stringify(name)}: new URL(${JSON.stringify("./" + name)}, import.meta.url),`);
writeFileSync(resolve(output, "files.js"), `export const MANIFEST = new URL("./manifest.json", import.meta.url);
export const FILES = {
  "manifest.json": MANIFEST,
${entries.join("\n")}
};
`);
writeFileSync(resolve(output, "files.d.ts"), "export declare const MANIFEST: URL;\nexport declare const FILES: Readonly<Record<string, URL>>;\n");
const total = packs.files.reduce((sum, file) => sum + file.bytes.length, 0);
console.log(`status=exported files=${packs.files.length} total_mb=${(total / 1048576).toFixed(2)} source=resources/packs`);
