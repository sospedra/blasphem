import { NitroModules } from "react-native-nitro-modules";
import { createJudgeWith, createSingleton, normalizeJudgement, normalizeLocales, type Judge, type JudgeOptions, type Judgement, type Transport } from "./core/index.js";
import type { BlasphemAssets, BlasphemEngineBuilder } from "./specs/BlasphemEngine.nitro.js";
import { assetReader, bundledConfiguration } from "./native-assets.js";

export type { Judge, JudgeOptions, InitOptions, Judgement } from "./core/index.js";
export { LOCALES, type LocaleCode } from "./core/index.js";

function toArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer;
}

const nativeEngine: Transport["engine"] = async (entries, detectLanguage, grawlix) => {
  const builder = NitroModules.createHybridObject<BlasphemEngineBuilder>("BlasphemEngineBuilder");
  builder.configure(detectLanguage, grawlix);
  for (const entry of entries) {
    builder.add(entry.locale, toArrayBuffer(entry.pack), entry.packSha256, entry.detect ? toArrayBuffer(entry.detect) : undefined, entry.detectSha256 ?? undefined);
  }
  const engine = builder.build();
  return {
    locales: engine.locales,
    judge: (text: string): Judgement => normalizeJudgement(engine.judge(text)),
    free: (): void => engine.close(),
  };
};

/**
 * Loads bundled packs, or persistent CDN packs with `assets: "jsdelivr"`.
 * `judge()` stays synchronous after initialization.
 */
export async function createJudge(options: JudgeOptions): Promise<Judge> {
  normalizeLocales(options?.locales);
  const assets = NitroModules.createHybridObject<BlasphemAssets>("BlasphemAssets");
  const read = await assetReader(assets, options.assets);
  const detectLanguage = options.detectLanguage ?? true;
  return createJudgeWith({ name: "native", read, engine: nativeEngine }, { ...options, detectLanguage });
}

// The module-level judge: `init` once, `judge` on every keystroke.
const singleton = createSingleton(async (options) => {
  if (options.locales !== undefined) return createJudge(options as JudgeOptions);
  const assets = NitroModules.createHybridObject<BlasphemAssets>("BlasphemAssets");
  const config = await bundledConfiguration(assets);
  return createJudge({ ...config, ...options, locales: config.locales });
});
export const init = singleton.init;
export const judge = singleton.judge;
export const ready = singleton.ready;
export const close = singleton.close;
