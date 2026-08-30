import initWasm from "./blasphem.js";
import { createJudgeWith, createSingleton, fail, resolveBrowserAssets, type AssetBases, type Judge, type JudgeOptions, type Transport } from "./core/index.js";
import { VERSIONS } from "./version.generated.js";
import { buildWasmEngine } from "./wasm-engine.js";

export type { AssetBases, Judge, JudgeOptions, Judgement } from "./core/index.js";
export { JSDELIVR, LOCALES, jsdelivrBases, type LocaleCode } from "./core/index.js";
export { VERSIONS } from "./version.generated.js";

const WASM_FILE = "blasphem_bg.wasm";
let wasmReady: Promise<unknown> | null = null;

async function fetchBytes(url: string): Promise<Uint8Array> {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url} answered ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

/**
 * Instantiates the wasm once per page. The glue never guesses a path; the caller names it.
 *
 * CSP: compiling WebAssembly needs `script-src 'wasm-unsafe-eval'` (or `'unsafe-eval'`
 * on engines older than Chrome 97, Firefox 102, Safari 16), and this fetch plus the pack
 * fetches need the `assets` origins in `connect-src`. Nothing here evaluates strings,
 * spawns a worker, or uses `blob:` URLs, so no other directive opens. See README.
 */
function loadWasm(url: string): Promise<unknown> {
  wasmReady ??= initWasm({ module_or_path: url });
  return wasmReady;
}

function browserTransport(bases: AssetBases): Transport {
  return {
    name: "wasm",
    read: (name) => fetchBytes(`${bases.packs}/${name}`),
    async engine(entries, detectLanguage, grawlix) {
      try {
        await loadWasm(`${bases.wasm}/${WASM_FILE}`);
      } catch (error) {
        wasmReady = null;
        throw fail("BLASPHEM_FETCH_FAILED", `${WASM_FILE}: ${error instanceof Error ? error.message : String(error)}`);
      }
      return buildWasmEngine(entries, detectLanguage, grawlix);
    },
  };
}

/**
 * Builds a judge for the requested locales. Fetches the wasm from the code
 * base and `manifest.json` plus one `.pack` and one `.detect` per locale from
 * the packs base. `assets` names both: one path, `"jsdelivr"`, or `{ wasm, packs }`.
 */
export function createJudge(options: JudgeOptions): Promise<Judge> {
  const bases = resolveBrowserAssets(options?.assets, VERSIONS);
  return createJudgeWith(browserTransport(bases), options);
}

// The module-level judge: `init` once, `judge` on every keystroke.
const singleton = createSingleton(createJudge);
export const init = singleton.init;
export const judge = singleton.judge;
export const ready = singleton.ready;
export const close = singleton.close;
