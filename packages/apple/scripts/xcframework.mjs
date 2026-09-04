import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdtempSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Builds crates/blasphem-ffi as a static library for iOS devices, iOS
 * simulators, and macOS, then combines the three with blasphem.h and a module
 * map into BlasphemFFI.xcframework and zips it for the GitHub Release.
 */
const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(packageRoot, "../..");
const targetDir = resolve(projectRoot, "target/ffi");
const header = resolve(projectRoot, "crates/blasphem-ffi/include/blasphem.h");
const ARCHIVE = "libblasphem_ffi.a";
const MODULE_MAP = 'module BlasphemFFI {\n  header "blasphem.h"\n  export *\n}\n';

const SLICES = [
  { triple: "aarch64-apple-ios", slice: "ios-arm64" },
  { triple: "aarch64-apple-ios-sim", slice: "ios-arm64-simulator" },
  { triple: "aarch64-apple-darwin", slice: "macos-arm64" },
];

function staticlib(triple) {
  execFileSync(
    "cargo",
    ["rustc", "--release", "--locked", "-p", "blasphem-ffi", "--target", triple, "--crate-type", "staticlib", "--manifest-path", resolve(projectRoot, "Cargo.toml")],
    { stdio: "inherit", env: { ...process.env, CARGO_TARGET_DIR: targetDir } },
  );
  const path = resolve(targetDir, triple, "release", ARCHIVE);
  if (!existsSync(path)) throw new Error(`cargo did not produce ${path}`);
  return path;
}

function megabytes(bytes) {
  return (bytes / 1048576).toFixed(2);
}

const archives = SLICES.map((slice) => ({ ...slice, path: staticlib(slice.triple) }));

const headers = mkdtempSync(resolve(tmpdir(), "blasphem-headers-"));
copyFileSync(header, resolve(headers, "blasphem.h"));
writeFileSync(resolve(headers, "module.modulemap"), MODULE_MAP);

const xcframework = resolve(packageRoot, "BlasphemFFI.xcframework");
rmSync(xcframework, { recursive: true, force: true });
const libraries = archives.flatMap(({ path }) => ["-library", path, "-headers", headers]);
execFileSync("xcodebuild", ["-create-xcframework", ...libraries, "-output", xcframework], { stdio: "inherit" });
rmSync(headers, { recursive: true, force: true });
copyFileSync(resolve(projectRoot, "NOTICE"), resolve(xcframework, "NOTICE"));

const zip = `${xcframework}.zip`;
rmSync(zip, { force: true });
execFileSync("ditto", ["-c", "-k", "--keepParent", xcframework, zip], { stdio: "inherit" });

const sizes = archives.map(({ slice, path }) => `${slice}=${megabytes(statSync(path).size)}`).join(",");
console.log(`status=built xcframework=BlasphemFFI.xcframework zip_mb=${megabytes(statSync(zip).size)} archive_mb=${sizes}`);
