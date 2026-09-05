import { fail } from "./errors.js";
import { LOCALES as MODEL_LOCALES, type LocaleCode as ModelLocaleCode } from "./locales.generated.js";

export type LocaleCode = ModelLocaleCode | (typeof MODEL_LOCALES)[number]["aliases"][number];

/** Supported input languages. Model routing stays in the generated registry. */
export const LOCALES: readonly { code: LocaleCode }[] = MODEL_LOCALES.flatMap((locale) =>
  [...locale.aliases, locale.code].map((code) => ({ code })),
);

const CANONICAL = new Map<string, ModelLocaleCode>();
for (const locale of MODEL_LOCALES) {
  CANONICAL.set(locale.code, locale.code);
  for (const alias of locale.aliases) CANONICAL.set(alias, locale.code);
}

const ORDER = new Map<ModelLocaleCode, number>(MODEL_LOCALES.map((locale, index) => [locale.code, index]));

/** Lowercases, resolves aliases, rejects unknown codes, and returns registry order without repeats. */
export function normalizeLocales(input: unknown): ModelLocaleCode[] {
  if (input === "all") return MODEL_LOCALES.map(({ code }) => code);
  if (!Array.isArray(input) || input.length === 0) {
    throw fail("BLASPHEM_LOCALES_EMPTY", "pass at least one locale, such as [\"en\"]");
  }
  const codes = new Set<ModelLocaleCode>();
  for (const raw of input) {
    const key = typeof raw === "string" ? raw.trim().toLowerCase() : "";
    const code = CANONICAL.get(key);
    if (code === undefined) throw fail("BLASPHEM_LOCALE_UNSUPPORTED", `unsupported locale ${JSON.stringify(raw)}`);
    codes.add(code);
  }
  return [...codes].sort((left, right) => (ORDER.get(left) ?? 0) - (ORDER.get(right) ?? 0));
}

export function selectedFiles(locales: readonly ModelLocaleCode[], detectLanguage: boolean): string[] {
  return [...new Set(locales.flatMap((locale) => detectLanguage
    ? [packFile(locale), detectFile(locale)] : [packFile(locale)]))];
}

export function packFile(code: ModelLocaleCode): string {
  return `${code}.pack`;
}

export function detectFile(code: ModelLocaleCode): string {
  return `${code}.detect`;
}
