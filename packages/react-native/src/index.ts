import { NitroModules } from "react-native-nitro-modules";
import { createJudgeWith, createSingleton, type Judge, type JudgeOptions, type Judgement, type Transport } from "./core/index.js";
import type { BlasphemAssets, BlasphemEngineBuilder } from "./specs/BlasphemEngine.nitro.js";

export type { Judge, JudgeOptions, Judgement } from "./core/index.js";
export { LOCALES, type LocaleCode } from "./core/index.js";

function toArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer;
}

/** Packs come from the app bundle through Swift or Kotlin; the engine is C++ over the Rust core. */
const nativeTransport: Transport = {
  name: "native",
  async read(name) {
    const assets = NitroModules.createHybridObject<BlasphemAssets>("BlasphemAssets");
    return new Uint8Array(await assets.readBundled(name));
  },
  async engine(entries, detectLanguage, grawlix) {
    const builder = NitroModules.createHybridObject<BlasphemEngineBuilder>("BlasphemEngineBuilder");
    builder.configure(detectLanguage, grawlix);
    for (const entry of entries) {
      builder.add(entry.locale, toArrayBuffer(entry.pack), entry.packSha256, entry.detect ? toArrayBuffer(entry.detect) : undefined, entry.detectSha256 ?? undefined);
    }
    const engine = builder.build();
    return {
      locales: engine.locales,
      judge: (text: string): Judgement => {
        const verdict = engine.judge(text);
        return { safe: verdict.safe, score: verdict.score, locale: verdict.locale ?? null, grawlix: verdict.grawlix ?? null };
      },
      free: (): void => engine.close(),
    };
  },
};

/**
 * Builds a judge for the requested locales from the packs the app bundle
 * carries. `assets` is ignored on React Native. `judge()` is synchronous.
 */
export function createJudge(options: JudgeOptions): Promise<Judge> {
  return createJudgeWith(nativeTransport, options);
}

// The module-level judge: `init` once, `judge` on every keystroke.
const singleton = createSingleton(createJudge);
export const init = singleton.init;
export const judge = singleton.judge;
export const ready = singleton.ready;
export const close = singleton.close;
