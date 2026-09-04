import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { createJudgeWith, createSingleton, fail, resolvePacksDirectory, type Judge, type JudgeOptions, type Transport } from "./core/index.js";
import { buildNativeEngine, loadNative } from "./native.js";
import { buildWasmEngine } from "./wasm-engine.js";

export type { AssetBases, Judge, JudgeOptions, Judgement } from "./core/index.js";
export { JSDELIVR, LOCALES, jsdelivrBases, type LocaleCode } from "./core/index.js";
export { VERSION } from "./version.generated.js";

// This entry is Node-only, so `import.meta.url` is Node's own and tracers such
// as @vercel/nft follow the literal. Bundlers never see it: Next.js users list
// `blasphem` and `@blasphem/packs` in `serverExternalPackages`.
const OWN_WASM = new URL("./blasphem_bg.wasm", import.meta.url);
let wasmReady: Promise<unknown> | null = null;

type PackFiles = { FILES: Readonly<Record<string, URL>> };

/** The installed `@blasphem/packs`, through its traceable `files.js`. */
async function installedPacks(): Promise<PackFiles> {
  try {
    return (await import("@blasphem/packs/files")) as PackFiles;
  } catch (error) {
    throw fail("BLASPHEM_ASSETS_REQUIRED", `install @blasphem/packs or pass assets as a directory: ${error instanceof Error ? error.message : String(error)}`);
  }
}

function reader(directory: string | null): (name: string) => Promise<Uint8Array> {
  if (directory !== null) return (name) => readFile(join(directory, name)).then((buffer) => new Uint8Array(buffer));
  let files: Promise<PackFiles> | null = null;
  return async (name) => {
    const { FILES } = await (files ??= installedPacks());
    const url = FILES[name];
    if (url === undefined) throw new Error(`${name} is not part of the installed @blasphem/packs`);
    return new Uint8Array(await readFile(fileURLToPath(url)));
  };
}

function loadWasm(): Promise<unknown> {
  wasmReady ??= (async () => {
    const [glue, bytes] = await Promise.all([import("./blasphem.js"), readFile(OWN_WASM)]);
    return glue.default({ module_or_path: bytes });
  })();
  return wasmReady;
}

function nodeTransport(directory: string | null): Transport {
  const native = loadNative();
  return {
    name: native ? "native" : "wasm",
    read: reader(directory),
    async engine(entries, detectLanguage, grawlix) {
      if (native) return buildNativeEngine(native, entries, detectLanguage, grawlix);
      try {
        await loadWasm();
      } catch (error) {
        wasmReady = null;
        throw fail("BLASPHEM_FETCH_FAILED", `blasphem_bg.wasm: ${error instanceof Error ? error.message : String(error)}`);
      }
      return buildWasmEngine(entries, detectLanguage, grawlix);
    },
  };
}

/**
 * Builds a judge for the requested locales. Reads the packs from the installed
 * `@blasphem/packs`, or from `options.assets` when it names a directory, and
 * runs the native binary for this platform when its package is installed.
 */
export function createJudge(options: JudgeOptions): Promise<Judge> {
  return createJudgeWith(nodeTransport(resolvePacksDirectory(options?.assets)), options);
}

// The module-level judge: `init` once, `judge` on every keystroke.
const singleton = createSingleton(createJudge);
export const init = singleton.init;
export const judge = singleton.judge;
export const ready = singleton.ready;
export const close = singleton.close;
