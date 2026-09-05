import type { Judge, JudgeOptions, Judgement } from "./contract.js";
import { fail, fromEngineError } from "./errors.js";
import { detectFile, normalizeLocales, packFile } from "./locales.js";
import { MANIFEST_FILE, parseManifest, type Manifest, type ManifestFile } from "./manifest.js";
import type { EngineHandle, Entry, Transport } from "./transport.js";

async function read(transport: Transport, name: string): Promise<Uint8Array> {
  try {
    return await transport.read(name);
  } catch (error) {
    throw fail("BLASPHEM_FETCH_FAILED", `${name}: ${error instanceof Error ? error.message : String(error)}`);
  }
}

function requireFile(manifest: Manifest, name: string, locale: string): ManifestFile {
  const record = manifest.files[name];
  if (record === undefined) throw fail("BLASPHEM_LOCALE_MISSING", `${MANIFEST_FILE} lists no ${name}; the packs do not include ${locale}`);
  return record;
}

async function readFile(transport: Transport, name: string, expected: ManifestFile): Promise<Uint8Array> {
  const bytes = await read(transport, name);
  if (bytes.byteLength !== expected.bytes) throw fail("BLASPHEM_DIGEST_MISMATCH", `${name} has the wrong length`);
  return bytes;
}

async function loadEntry(transport: Transport, manifest: Manifest, selection: { locale: string; detectLanguage: boolean }): Promise<Entry> {
  const { locale, detectLanguage } = selection;
  const packName = packFile(locale as never);
  const detectName = detectFile(locale as never);
  const packRecord = requireFile(manifest, packName, locale);
  const detectRecord = detectLanguage ? requireFile(manifest, detectName, locale) : null;
  const [pack, detect] = await Promise.all([
    readFile(transport, packName, packRecord),
    detectRecord ? readFile(transport, detectName, detectRecord) : Promise.resolve(null),
  ]);
  return { locale, pack, packSha256: packRecord.sha256, detect, detectSha256: detectRecord?.sha256 ?? null };
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
  if (typeof detectLanguage !== "boolean" || typeof grawlix !== "boolean") {
    throw fail("BLASPHEM_PACK_INVALID", "detectLanguage and grawlix must be booleans");
  }
  const manifest = parseManifest(await read(transport, MANIFEST_FILE));
  const entries = await Promise.all(locales.map((locale) => loadEntry(transport, manifest, { locale, detectLanguage })));
  let handle: EngineHandle;
  try {
    handle = await transport.engine(entries, detectLanguage, grawlix);
  } catch (error) {
    throw fromEngineError(error);
  }
  return wrap(handle, transport.name);
}
