const { createRequire } = require("node:module");
const { resolve } = require("node:path");
const { bundleAssets } = require("./scripts/bundle-assets.cjs");

module.exports = function withBlasphem(config) {
  const projectRoot = config._internal?.projectRoot ?? process.cwd();
  const { withDangerousMod } = createRequire(resolve(projectRoot, "package.json"))("expo/config-plugins");
  return ["ios", "android"].reduce((current, platform) => withDangerousMod(current, [
    platform,
    async mod => {
      await bundleAssets(platform, mod.modRequest.projectRoot);
      return mod;
    },
  ]), config);
};
