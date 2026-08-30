import { LOCALES } from "../../../../packages/core/src/locales.generated";

/** Uppercase codes, generated from the Rust `Language::ALL` through the core table. */
export type LanguageCode = Uppercase<(typeof LOCALES)[number]["code"]>;
export type Selection = LanguageCode | "AUTO";

export type Language = {
  code: LanguageCode;
  name: string;
  tag: string;
  direction: "ltr" | "rtl";
};

/** Display data only. Codes and aliases come from the generated table. */
const DISPLAY: Record<LanguageCode, { name: string; direction: "ltr" | "rtl" }> = {
  EN: { name: "English", direction: "ltr" },
  ZH: { name: "Chinese", direction: "ltr" },
  ES: { name: "Spanish", direction: "ltr" },
  AR: { name: "Arabic", direction: "rtl" },
  MS: { name: "Malay", direction: "ltr" },
  PT: { name: "Portuguese", direction: "ltr" },
  FR: { name: "French", direction: "ltr" },
  HI: { name: "Hindi", direction: "ltr" },
  RU: { name: "Russian", direction: "ltr" },
  JA: { name: "Japanese", direction: "ltr" },
  DE: { name: "German", direction: "ltr" },
  TR: { name: "Turkish", direction: "ltr" },
  VI: { name: "Vietnamese", direction: "ltr" },
  KO: { name: "Korean", direction: "ltr" },
  IT: { name: "Italian", direction: "ltr" },
};

export const LANGUAGES: readonly Language[] = LOCALES.map((locale) => {
  const code = locale.code.toUpperCase() as LanguageCode;
  return { code, name: DISPLAY[code].name, tag: locale.code, direction: DISPLAY[code].direction };
});

const ALIASES: Record<string, LanguageCode> = Object.fromEntries(
  LOCALES.flatMap((locale) => locale.aliases.map((alias) => [alias.toUpperCase(), locale.code.toUpperCase() as LanguageCode])),
);
const CODES = new Set<string>(LANGUAGES.map((language) => language.code));

export function normalizeSelection(raw: string): Selection | null {
  const upper = raw.trim().toUpperCase();
  if (upper === "AUTO") return "AUTO";
  const resolved = ALIASES[upper] ?? upper;
  return CODES.has(resolved) ? (resolved as LanguageCode) : null;
}

export function storageCode(code: LanguageCode): string {
  return code === "MS" ? "ID" : code;
}

export function languageByCode(code: LanguageCode): Language {
  const found = LANGUAGES.find((language) => language.code === code);
  if (!found) throw new Error(`unknown language ${code}`);
  return found;
}
