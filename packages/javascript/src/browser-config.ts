import { BUNDLE } from "./bundle.generated.js";
import { parseBundle, fail, type BundleConfiguration } from "./core/index.js";
import { VERSION } from "./version.generated.js";

export function browserConfiguration(): BundleConfiguration {
  const input = BUNDLE ?? (globalThis as { __BLASPHEM_CONFIG__?: unknown }).__BLASPHEM_CONFIG__;
  if (input === undefined) {
    throw fail("BLASPHEM_ASSETS_REQUIRED", "Use blasphem/vite or include the config.js emitted by blasphem-assets");
  }
  return parseBundle(input, VERSION);
}
