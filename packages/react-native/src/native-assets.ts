import { fail, jsdelivrBases, MANIFEST_FILE, parseManifest, parseBundle, type BundleConfiguration, type JudgeOptions, type Transport } from "./core/index.js";
import type { BlasphemAssets } from "./specs/BlasphemEngine.nitro.js";
import { VERSION, MANIFEST_INTEGRITY } from "./version.generated.js";

export async function bundledConfiguration(assets: BlasphemAssets): Promise<BundleConfiguration> {
  const bytes = await assets.readBundled("bundle.json");
  return parseBundle(JSON.parse(new TextDecoder().decode(bytes)), VERSION);
}

export async function assetReader(assets: BlasphemAssets, source: JudgeOptions["assets"]): Promise<Transport["read"]> {
  if (source === undefined || source === "bundled") return async (name) => new Uint8Array(await assets.readBundled(name));
  if (source !== "jsdelivr" && source !== "remote") throw fail("BLASPHEM_ASSETS_REQUIRED", "React Native assets must be bundled or remote");
  const base = jsdelivrBases(VERSION).packs;
  const bytes = new Uint8Array(await assets.readDownloaded(`${base}/${MANIFEST_FILE}`, MANIFEST_INTEGRITY));
  const manifest = parseManifest(bytes);
  return async (name) => {
    if (name === MANIFEST_FILE) return bytes;
    const expected = manifest.files[name];
    if (!expected) throw fail("BLASPHEM_LOCALE_MISSING", `The manifest lists no ${name}`);
    return new Uint8Array(await assets.readDownloaded(`${base}/${name}`, expected));
  };
}
