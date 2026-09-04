import type { Judge, JudgeOptions, Judgement } from "./contract.js";
import { fail, fromEngineError } from "./errors.js";
import { detectFile, normalizeLocales, packFile } from "./locales.js";
import { MANIFEST_FILE, parseManifest, type Manifest } from "./manifest.js";
import type { EngineHandle, Entry, Transport } from "./transport.js";

async function read(transport: Transport, name: string): Promise<Uint8Array> {
  try {
    return await transport.read(name);
  } catch (error) {
    throw fail("BLASPHEM_FETCH_FAILED", `${name}: ${error instanceof Error ? error.message : String(error)}`);
  }
}

function requireFile(manifest: Manifest, name: string, locale: string): string {
  const record = manifest.files[name];
  if (record === undefined) throw fail("BLASPHEM_LOCALE_MISSING", `${MANIFEST_FILE} lists no ${name}; the packs do not include ${locale}`);
  return record.sha256;
}

async function loadEntry(transport: Transport, manifest: Manifest, locale: string, detectLanguage: boolean): Promise<Entry> {
  const packName = packFile(locale as never);
  const detectName = detectFile(locale as never);
  const packSha256 = requireFile(manifest, packName, locale);
  const detectSha256 = detectLanguage ? requireFile(manifest, detectName, locale) : null;
  const [pack, detect] = await Promise.all([
    read(transport, packName),
    detectLanguage ? read(transport, detectName) : Promise.resolve(null),
  ]);
  return { locale, pack, packSha256, detect, detectSha256 };
}

function wrap(handle: EngineHandle, transport: Transport["name"]): Judge {
  let open = true;
  const locales = Object.freeze([...handle.locales]);
  return {
    locales,
    transport,
    judge(text: string): Judgement {
      if (!open) throw fail("BLASPHEM_CLOSED", "the judge was closed");
      return handle.judge(text);
    },
    close(): void {
      if (!open) return;
      open = false;
      handle.free();
    },
  };
}

/**
 * Validates the options, reads the manifest, loads every file in parallel,
 * and starts the engine. Every failure throws before a judge exists.
 */
export async function createJudgeWith(transport: Transport, options: JudgeOptions): Promise<Judge> {
  const locales = normalizeLocales(options?.locales);
  const detectLanguage = options.detectLanguage ?? true;
  const grawlix = options.grawlix ?? false;
  const manifest = parseManifest(await read(transport, MANIFEST_FILE));
  const entries = await Promise.all(locales.map((locale) => loadEntry(transport, manifest, locale, detectLanguage)));
  let handle: EngineHandle;
  try {
    handle = await transport.engine(entries, detectLanguage, grawlix);
  } catch (error) {
    throw fromEngineError(error);
  }
  return wrap(handle, transport.name);
}
