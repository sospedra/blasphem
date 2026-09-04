import { LANGUAGES, reportCode, type LanguageCode } from "./languages";
import { smoke } from "./reports";

export type Sample = {
  code: LanguageCode;
  tag: string;
  direction: "ltr" | "rtl";
  name: string;
  kind: "toxic" | "clean";
  text: string;
};

export const SAMPLES: readonly Sample[] = LANGUAGES.flatMap((language) =>
  smoke.languages[reportCode(language.code)].cases
    .map((entry) => ({
      code: language.code,
      tag: language.tag,
      direction: language.direction,
      name: language.name,
      kind: entry.expected_nudge ? "toxic" : "clean",
      text: entry.text,
    })),
);
