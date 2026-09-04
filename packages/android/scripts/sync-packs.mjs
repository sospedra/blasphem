import { copyFileSync, mkdirSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, extname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Copies every .pack and .detect file into its own Gradle module under
 * packs/<code>/<kind>/, with the NOTICE and a manifest. The copies are
 * gitignored; settings.gradle.kts includes one module per file present.
 * Usage: node scripts/sync-packs.mjs [packs directory]
 */
const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(packageRoot, "../..");
const source = resolve(process.argv[2] ?? resolve(projectRoot, "packages/packs/dist"));
const notice = resolve(projectRoot, "NOTICE");
const packs = resolve(packageRoot, "packs");
const MANIFEST = '<manifest xmlns:android="http://schemas.android.com/apk/res/android" />\n';
const KINDS = new Set([".pack", ".detect"]);

const files = readdirSync(source).filter((name) => KINDS.has(extname(name))).sort();
if (files.length === 0) throw new Error(`${source} holds no .pack or .detect file; run packages/packs/scripts/build.mjs first`);

rmSync(packs, { recursive: true, force: true });
for (const name of files) {
  const kind = extname(name).slice(1);
  const code = name.slice(0, -kind.length - 1);
  const main = resolve(packs, code, kind, "src/main");
  mkdirSync(resolve(main, "assets/blasphem"), { recursive: true });
  mkdirSync(resolve(main, "resources/META-INF"), { recursive: true });
  copyFileSync(resolve(source, name), resolve(main, "assets/blasphem", name));
  copyFileSync(notice, resolve(main, "resources/META-INF/NOTICE"));
  writeFileSync(resolve(main, "AndroidManifest.xml"), MANIFEST);
}

const bytes = files.reduce((sum, name) => sum + statSync(resolve(source, name)).size, 0);
console.log(`status=synced modules=${files.length} total_mb=${(bytes / 1048576).toFixed(2)} source=${source}`);
