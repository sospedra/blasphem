import { fail } from "./errors.js";
import { LOCALES, type LocaleCode } from "./locales.generated.js";

const CANONICAL = new Map<string, LocaleCode>();
for (const locale of LOCALES) {
  CANONICAL.set(locale.code, locale.code);
  for (const alias of locale.aliases) CANONICAL.set(alias, locale.code);
}

const ORDER = new Map<LocaleCode, number>(LOCALES.map((locale, index) => [locale.code, index]));

/** Lowercases, resolves aliases, rejects unknown codes, and returns registry order without repeats. */
export function normalizeLocales(input: unknown): LocaleCode[] {
  if (!Array.isArray(input) || input.length === 0) {
    throw fail("BLASPHEM_LOCALES_EMPTY", "pass at least one locale, such as [\"en\"]");
  }
  const codes = new Set<LocaleCode>();
  for (const raw of input) {
    const key = typeof raw === "string" ? raw.trim().toLowerCase() : "";
    const code = CANONICAL.get(key);
    if (code === undefined) throw fail("BLASPHEM_LOCALE_UNSUPPORTED", `unsupported locale ${JSON.stringify(raw)}`);
    codes.add(code);
  }
  return [...codes].sort((left, right) => (ORDER.get(left) ?? 0) - (ORDER.get(right) ?? 0));
}

export function packFile(code: LocaleCode): string {
  return `${code}.pack`;
}

export function detectFile(code: LocaleCode): string {
  return `${code}.detect`;
}
