import init, { BlasphemJudge } from "./blasphem.js";

// The build writes the wasm-bindgen glue into src/ next to this file and
// copies it into dist/ next to the compiled output, so "./blasphem.js"
// resolves the same way at type-check time and at run time.

let ready: Promise<void> | null = null;

function isNode(): boolean {
  return typeof process !== "undefined" && process.versions?.node != null;
}

async function nodeBytes(): Promise<Uint8Array> {
  const { readFile } = await import("node:fs/promises");
  const { fileURLToPath } = await import("node:url");
  return readFile(fileURLToPath(new URL("./blasphem_bg.wasm", import.meta.url)));
}

async function start(): Promise<void> {
  if (!isNode()) {
    await init();
    return;
  }
  await init({ module_or_path: await nodeBytes() });
}

/** Loads the WebAssembly module once. Repeated calls share one promise. */
export function load(): Promise<void> {
  ready ??= start();
  return ready;
}

export { BlasphemJudge };
