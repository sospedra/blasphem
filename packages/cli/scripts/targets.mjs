// The command-line binary ships for the same seven targets as the napi engine.
export { TARGETS, hostTarget } from "../../node/scripts/targets.mjs";

export function packageName(target) {
  return `@blasphem/cli-${target.name}`;
}

/** The executable name inside the platform package's bin directory. */
export function binaryName(target) {
  return target.os === "win32" ? "blasphem.exe" : "blasphem";
}
