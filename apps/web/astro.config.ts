import sitemap from "@astrojs/sitemap";
import { defineConfig } from "astro/config";
import { fileURLToPath } from "node:url";
import blasphemAssets from "./integrations/blasphem-assets";

const site = process.env.SITE_URL ?? "https://blasphem.sospedra.me";
const packageDist = fileURLToPath(new URL("../../packages/blasphem/dist/", import.meta.url));

export default defineConfig({
  site,
  output: "static",
  compressHTML: true,
  integrations: [sitemap(), blasphemAssets({ distDir: packageDist })],
});
