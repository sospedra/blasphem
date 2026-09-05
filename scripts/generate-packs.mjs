import { execFileSync } from "node:child_process";
import { copyFileSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { canonicalPacks, loadPacks } from "./packs.mjs";

const root = fileURLToPath(new URL("../", import.meta.url));
const check = process.argv.includes("--check");
const output = check ? mkdtempSync(resolve(tmpdir(), "blasphem-packs-check-")) : canonicalPacks;

try {
  execFileSync("cargo", [
    "run", "--release", "--locked", "-p", "blasphem-train", "--", "pack",
    "--model-manifest", "resources/metadata/model-manifest.json",
    "--model-root", "resources/models",
    "--language-model", "crates/blasphem-language/data/blasphem-language-15.bin",
    "--lexicon-root", "resources/lexicon", "--output", output,
  ], { cwd: root, stdio: "inherit" });
  const generated = loadPacks(output);
  if (check) {
    const committed = loadPacks();
    if (!generated.manifestBytes.equals(committed.manifestBytes)) throw new Error("Canonical packs are stale; run pnpm packs:generate");
  } else {
    copyFileSync(resolve(root, "NOTICE"), resolve(canonicalPacks, "NOTICE"));
  }
  console.log(`status=${check ? "verified" : "generated"} files=${generated.files.length} source=resources/packs`);
} finally {
  if (check) rmSync(output, { recursive: true, force: true });
}
