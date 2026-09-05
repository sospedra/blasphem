import type { BundleConfiguration, Manifest, ManifestFile } from "../dist/core/index.js";
export const packageRoot: string;
export function installedPacks(): string;
export function readVerified(root: string, name: string, expected: ManifestFile): Uint8Array;
export function selectedAssets(config: BundleConfiguration): { root: string; manifest: Manifest };
export function browserAssets(projectRoot: string, publicBase: string): {
  bundle: BundleConfiguration;
  entries: [string, Uint8Array][];
};
