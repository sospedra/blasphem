import initWasm from "./blasphem.js";
import { createJudgeWith, createSingleton, fail, normalizeLocales, parseManifest, resolveBrowserAssets, type BundleConfiguration, type Judge, type JudgeOptions, type Transport } from "./core/index.js";
import { VERSION, WASM_INTEGRITY, MANIFEST_INTEGRITY } from "./version.generated.js";
import { buildWasmEngine } from "./wasm-engine.js";
import { browserConfiguration } from "./browser-config.js";
import { fetchBytes, readRemoteFile } from "./browser-cache.js";

export type { AssetBases, Judge, JudgeOptions, InitOptions, Judgement } from "./core/index.js";
export { JSDELIVR, LOCALES, jsdelivrBases, type LocaleCode } from "./core/index.js";
export { VERSION } from "./version.generated.js";

const WASM_FILE = "blasphem_bg.wasm";
let wasmReady: Promise<unknown> | null = null;

function loadWasm(read: Transport["read"]): Promise<unknown> {
  wasmReady ??= read(WASM_FILE).then((bytes) => initWasm({ module_or_path: bytes }));
  return wasmReady;
}

function assetUrl(name: string, source: { options: JudgeOptions; config?: BundleConfiguration }): string {
  const delivery = source.options.assets === "jsdelivr" ? "remote" : source.options.assets ?? "bundled";
  const configured = delivery === source.config?.assets ? source.config?.assetUrls?.[name] : undefined;
  if (configured) return configured;
  const bases = resolveBrowserAssets(source.options.assets, VERSION);
  const base = name === WASM_FILE ? bases.wasm : bases.packs;
  return `${base}/${name}`;
}

async function remoteReader(source: { options: JudgeOptions; config?: BundleConfiguration }): Promise<Transport["read"]> {
  const [manifestBytes, wasmBytes] = await Promise.all([
    readRemoteFile(assetUrl("manifest.json", source), { version: VERSION, expected: MANIFEST_INTEGRITY }),
    readRemoteFile(assetUrl(WASM_FILE, source), { version: VERSION, expected: WASM_INTEGRITY }),
  ]);
  const manifest = parseManifest(manifestBytes);
  return async (name) => {
    if (name === "manifest.json") return manifestBytes;
    if (name === WASM_FILE) return wasmBytes;
    const expected = manifest.files[name];
    if (!expected) throw fail("BLASPHEM_LOCALE_MISSING", `The release manifest lists no ${name}`);
    return readRemoteFile(assetUrl(name, source), { version: VERSION, expected });
  };
}

function browserTransport(read: Transport["read"]): Transport {
  return {
    name: "wasm", read,
    async engine(entries, detectLanguage, grawlix) {
      try {
        await loadWasm(read);
      } catch (error) {
        wasmReady = null;
        throw fail("BLASPHEM_FETCH_FAILED", `${WASM_FILE}: ${error instanceof Error ? error.message : String(error)}`);
      }
      return buildWasmEngine(entries, detectLanguage, grawlix);
    },
  };
}

async function configuredJudge(options: JudgeOptions, config?: BundleConfiguration): Promise<Judge> {
  options = { ...options, locales: normalizeLocales(options?.locales) };
  const source = { options, config };
  const remote = options.assets === "remote" || options.assets === "jsdelivr";
  const read = remote ? await remoteReader(source) : (name: string) => fetchBytes(assetUrl(name, source));
  return createJudgeWith(browserTransport(read), options);
}

/** Creates an independent judge with an explicit selection or custom asset source. */
export async function createJudge(options: JudgeOptions): Promise<Judge> {
  if (options.assets === undefined || options.assets === "bundled") {
    const config = browserConfiguration();
    return configuredJudge({ ...config, ...options }, config);
  }
  return configuredJudge(options);
}

const singleton = createSingleton((options) => {
  if (options.locales !== undefined) return createJudge(options as JudgeOptions);
  const config = browserConfiguration();
  return configuredJudge({ ...config, ...options, locales: config.locales }, config);
});
export const init = singleton.init;
export const judge = singleton.judge;
export const ready = singleton.ready;
export const close = singleton.close;
