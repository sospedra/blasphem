export type LanguageCode = "EN" | "ZH" | "ES" | "AR" | "MS" | "PT" | "FR" | "HI" | "RU" | "JA" | "DE" | "TR" | "VI" | "KO" | "IT";
export type Selection = LanguageCode | "AUTO";

export type Language = {
  code: LanguageCode;
  name: string;
  tag: string;
  direction: "ltr" | "rtl";
};

export const LANGUAGES: readonly Language[] = [
  { code: "EN", name: "English", tag: "en", direction: "ltr" },
  { code: "ZH", name: "Chinese", tag: "zh", direction: "ltr" },
  { code: "ES", name: "Spanish", tag: "es", direction: "ltr" },
  { code: "AR", name: "Arabic", tag: "ar", direction: "rtl" },
  { code: "MS", name: "Malay", tag: "ms", direction: "ltr" },
  { code: "PT", name: "Portuguese", tag: "pt", direction: "ltr" },
  { code: "FR", name: "French", tag: "fr", direction: "ltr" },
  { code: "HI", name: "Hindi", tag: "hi", direction: "ltr" },
  { code: "RU", name: "Russian", tag: "ru", direction: "ltr" },
  { code: "JA", name: "Japanese", tag: "ja", direction: "ltr" },
  { code: "DE", name: "German", tag: "de", direction: "ltr" },
  { code: "TR", name: "Turkish", tag: "tr", direction: "ltr" },
  { code: "VI", name: "Vietnamese", tag: "vi", direction: "ltr" },
  { code: "KO", name: "Korean", tag: "ko", direction: "ltr" },
  { code: "IT", name: "Italian", tag: "it", direction: "ltr" },
];

const ALIASES: Record<string, LanguageCode> = { ID: "MS" };
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
