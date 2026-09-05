import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fail, resolveConfiguration, type InitOptions, type JudgeOptions } from "./core/index.js";
import { VERSION } from "./version.generated.js";

async function packageConfiguration(directory: string): Promise<unknown> {
  const bytes = await readFile(join(directory, "package.json"), "utf8").catch((error: NodeJS.ErrnoException) => {
    if (error.code === "ENOENT") return null;
    throw error;
  });
  return bytes === null ? undefined : (JSON.parse(bytes) as { blasphem?: unknown }).blasphem;
}

export async function applicationConfiguration(directory: string): Promise<ReturnType<typeof resolveConfiguration>> {
  const parents = directory.split(/[\\/]/).length;
  let current = directory;
  for (let depth = 0; depth < parents; depth++) {
    const input = await packageConfiguration(current);
    if (input !== undefined) return resolveConfiguration(input, VERSION);
    current = dirname(current);
  }
  throw fail("BLASPHEM_ASSETS_REQUIRED", "Declare blasphem.locales in the application's package.json");
}

export async function nodeOptions(options: InitOptions): Promise<JudgeOptions> {
  if (options.locales !== undefined) return options as JudgeOptions;
  const config = await applicationConfiguration(process.cwd());
  return { ...config, ...options, locales: config.locales };
}
