import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(packageRoot, "../..");
const output = resolve(packageRoot, "dist");

execFileSync(
  "cargo",
  [
    "run", "--release", "--locked", "-p", "blasphem-train",
    "--manifest-path", resolve(projectRoot, "Cargo.toml"),
    "--",
    "pack",
    "--model-manifest", resolve(projectRoot, "resources/models/multilingual-v2/manifest.json"),
    "--model-root", resolve(projectRoot, "resources/models/multilingual-v2"),
    "--language-model", resolve(projectRoot, "crates/blasphem-language/data/blasphem-language-15-v2.bin"),
    "--hurtlex-root", resolve(projectRoot, "data/clean-room-v1"),
    "--output", output,
  ],
  { stdio: "inherit", env: { ...process.env, CARGO_TARGET_DIR: resolve(projectRoot, "target") } },
);

const manifest = JSON.parse(readFileSync(resolve(output, "manifest.json"), "utf8"));
const total = Object.values(manifest.files).reduce((sum, file) => sum + file.bytes, 0);
console.log(`status=built files=${Object.keys(manifest.files).length} total_mb=${(total / 1048576).toFixed(2)}`);
