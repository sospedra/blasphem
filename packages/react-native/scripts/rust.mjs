import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Builds the Rust engine for every mobile target: an iOS XCFramework the
 * podspec vendors and one static archive per Android ABI that CMake links.
 * Android archives need no NDK; only the static library is produced.
 */
const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(packageRoot, "../..");
const targetDir = resolve(projectRoot, "target/ffi");
const header = resolve(projectRoot, "crates/blasphem-ffi/include/blasphem.h");
const ARCHIVE = "libblasphem_ffi.a";

const IOS = [
  { triple: "aarch64-apple-ios", slice: "device" },
  { triple: "aarch64-apple-ios-sim", slice: "simulator" },
];
const ANDROID = [
  { triple: "aarch64-linux-android", abi: "arm64-v8a" },
  { triple: "armv7-linux-androideabi", abi: "armeabi-v7a" },
  { triple: "x86_64-linux-android", abi: "x86_64" },
];

function cargo(args) {
  execFileSync("cargo", [...args, "--manifest-path", resolve(projectRoot, "Cargo.toml")], {
    stdio: "inherit",
    env: { ...process.env, CARGO_TARGET_DIR: targetDir },
  });
}

function staticlib(triple) {
  cargo(["rustc", "--release", "--locked", "-p", "blasphem-ffi", "--target", triple, "--crate-type", "staticlib"]);
  const path = resolve(targetDir, triple, "release", ARCHIVE);
  if (!existsSync(path)) throw new Error(`cargo did not produce ${path}`);
  return path;
}

copyFileSync(header, resolve(packageRoot, "cpp/blasphem.h"));

const headers = mkdtempSync(resolve(tmpdir(), "blasphem-headers-"));
copyFileSync(header, resolve(headers, "blasphem.h"));
const xcframework = resolve(packageRoot, "ios/BlasphemFFI.xcframework");
rmSync(xcframework, { recursive: true, force: true });
const xcodebuildArgs = ["-create-xcframework"];
for (const { triple } of IOS) xcodebuildArgs.push("-library", staticlib(triple), "-headers", headers);
xcodebuildArgs.push("-output", xcframework);
execFileSync("xcodebuild", xcodebuildArgs, { stdio: "inherit" });
rmSync(headers, { recursive: true, force: true });

let androidBytes = 0;
for (const { triple, abi } of ANDROID) {
  const built = staticlib(triple);
  const directory = resolve(packageRoot, "android/libs", abi);
  mkdirSync(directory, { recursive: true });
  copyFileSync(built, resolve(directory, ARCHIVE));
  androidBytes += statSync(built).size;
}

console.log(`status=built xcframework=ios/BlasphemFFI.xcframework android_abis=${ANDROID.map((entry) => entry.abi).join(",")} android_archive_mb=${(androidBytes / 1048576).toFixed(2)}`);
