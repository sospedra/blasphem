import { browserAssets } from "./assets.mjs";

const VIRTUAL_CONFIG = "\0blasphem:bundle";

/** Emits the consuming application's selected assets and resolved configuration. */
export default function blasphem() {
  let prepared;
  let publicBase;
  return {
    name: "blasphem-assets",
    enforce: "pre",
    config() {
      return { optimizeDeps: { exclude: ["blasphem"] } };
    },
    configResolved(config) {
      publicBase = `${config.base}blasphem/`;
      prepared = browserAssets(config.root, publicBase);
    },
    resolveId(source, importer) {
      if (source === "./bundle.generated.js" && importer?.endsWith("/browser-config.js")) return VIRTUAL_CONFIG;
    },
    load(id) {
      if (id === VIRTUAL_CONFIG) return `export const BUNDLE = ${JSON.stringify(prepared.bundle)};`;
    },
    generateBundle() {
      for (const [name, source] of prepared.entries) {
        this.emitFile({ type: "asset", fileName: `blasphem/${name}`, source });
      }
    },
    configureServer(server) {
      const assets = new Map(prepared.entries);
      server.middlewares.use((request, response, next) => {
        const path = request.url?.split("?")[0];
        if (!path?.startsWith(publicBase)) return next();
        const bytes = assets.get(path.slice(publicBase.length));
        if (!bytes) return next();
        response.setHeader("Content-Type", "application/octet-stream");
        response.end(bytes);
      });
    },
  };
}
