import { LOCALES } from "../../../../packages/javascript-common/src/locales";
import { LOCALES as MODEL_LOCALES } from "../../../../packages/javascript-common/src/locales.generated";

/** Uppercase input codes from the shared public locale list. */
export type LanguageCode = Uppercase<(typeof LOCALES)[number]["code"]>;
export type Selection = LanguageCode | "AUTO";

export type Language = {
  code: LanguageCode;
  name: string;
  tag: string;
  direction: "ltr" | "rtl";
};

/** Display data only. Codes come from the shared public locale list. */
const DISPLAY: Record<LanguageCode, { name: string; direction: "ltr" | "rtl" }> = {
  EN: { name: "English", direction: "ltr" },
  ZH: { name: "Chinese", direction: "ltr" },
  ES: { name: "Spanish", direction: "ltr" },
  AR: { name: "Arabic", direction: "rtl" },
  ID: { name: "Indonesian", direction: "ltr" },
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

export const REPORT_LANGUAGES = MODEL_LOCALES.map((locale) => ({
  code: locale.code.toUpperCase(),
  name: [...locale.aliases, locale.code]
    .map((code) => `${DISPLAY[code.toUpperCase() as LanguageCode].name} · ${code}`)
    .join(" / "),
}));

const CODES = new Set<string>(LANGUAGES.map((language) => language.code));

export function normalizeSelection(raw: string): Selection | null {
  const upper = raw.trim().toUpperCase();
  if (upper === "AUTO") return "AUTO";
  return CODES.has(upper) ? (upper as LanguageCode) : null;
}

export function storageCode(code: LanguageCode): string {
  return code === "MS" ? "ID" : code;
}

export function reportCode(code: LanguageCode): string {
  return code === "ID" ? "MS" : code;
}

export function languageByCode(code: LanguageCode): Language {
  const found = LANGUAGES.find((language) => language.code === code);
  if (!found) throw new Error(`unknown language ${code}`);
  return found;
}
