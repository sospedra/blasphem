import { copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { dirname, extname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { canonicalPacks, copyPack, loadPacks, replaceDirectory } from "../../../scripts/packs.mjs";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(packageRoot, "../..");
const source = resolve(process.argv[2] ?? canonicalPacks);
const notice = resolve(projectRoot, "NOTICE");
const packs = resolve(packageRoot, "packs");
const manifest = '<manifest xmlns:android="http://schemas.android.com/apk/res/android" />\n';
const files = loadPacks(source).files;
const staged = mkdtempSync(resolve(packageRoot, ".packs-export-"));

function writeModule(file) {
  const kind = extname(file.name).slice(1);
  const code = file.name.slice(0, -kind.length - 1);
  const main = resolve(staged, code, kind, "src/main");
  mkdirSync(resolve(main, "resources/META-INF"), { recursive: true });
  copyPack(file, resolve(main, "assets/blasphem", file.name));
  copyFileSync(notice, resolve(main, "resources/META-INF/NOTICE"));
  writeFileSync(resolve(main, "AndroidManifest.xml"), manifest);
}

try {
  for (const file of files) writeModule(file);
  replaceDirectory(staged, packs);
} finally {
  rmSync(staged, { recursive: true, force: true });
}
const bytes = files.reduce((sum, file) => sum + file.bytes.length, 0);
console.log(`status=synced modules=${files.length} total_mb=${(bytes / 1048576).toFixed(2)} source=${source}`);
