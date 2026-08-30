/** The initial native set. `triple` is the Rust target, `name` the npm suffix. */
export const TARGETS = [
  { name: "darwin-arm64", triple: "aarch64-apple-darwin", os: "darwin", cpu: "arm64", libc: null },
  { name: "darwin-x64", triple: "x86_64-apple-darwin", os: "darwin", cpu: "x64", libc: null },
  { name: "linux-x64-gnu", triple: "x86_64-unknown-linux-gnu", os: "linux", cpu: "x64", libc: "glibc" },
  { name: "linux-arm64-gnu", triple: "aarch64-unknown-linux-gnu", os: "linux", cpu: "arm64", libc: "glibc" },
  { name: "linux-x64-musl", triple: "x86_64-unknown-linux-musl", os: "linux", cpu: "x64", libc: "musl" },
  { name: "linux-arm64-musl", triple: "aarch64-unknown-linux-musl", os: "linux", cpu: "arm64", libc: "musl" },
  { name: "win32-x64-msvc", triple: "x86_64-pc-windows-msvc", os: "win32", cpu: "x64", libc: null },
];

export function packageName(target) {
  return `@blasphem/node-${target.name}`;
}

export function binaryName(target) {
  return `blasphem.${target.name}.node`;
}

/** The target Node itself runs on, or null when no package covers it. */
export function hostTarget() {
  const { platform, arch } = process;
  const libc = platform === "linux" ? (isMusl() ? "musl" : "glibc") : null;
  return TARGETS.find((target) => target.os === platform && target.cpu === arch && target.libc === libc) ?? null;
}

function isMusl() {
  const header = process.report?.getReport?.()?.header;
  return typeof header?.glibcVersionRuntime !== "string";
}
