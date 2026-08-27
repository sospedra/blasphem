import { formatMegabytes, formatMs, formatPercent } from "./format";
import { LANGUAGES } from "./languages";
import { worstP95Ms } from "./metrics";
import { browser, performance, validation } from "./reports";

export type Claim = {
  numeral: string;
  value: string;
  label: string;
  evidence: string;
  glyph: "chalice" | "candle" | "censer" | "fleur" | "roseWindow" | "sacredHeart";
};

const precisions = Object.values(validation.languages).map((entry) => entry.metrics.precision);
const lowestPrecision = Math.min(...precisions);
const explicitBytes = browser.browser_builds.explicit_only.brotli_total_bytes;
const routedBytes = browser.browser_builds.full.brotli_total_bytes;
const worstShort = worstP95Ms(performance.fixtures, "-280");
const worstLong = worstP95Ms(performance.fixtures, "-4096");

export const CLAIMS: readonly Claim[] = [
  {
    numeral: "I",
    value: formatMegabytes(explicitBytes, 1),
    label: "when you name the tongue",
    evidence: `Brotli, without the language router. With AUTO routing: ${formatMegabytes(routedBytes, 1)}.`,
    glyph: "chalice",
  },
  {
    numeral: "II",
    value: formatMs(worstShort),
    label: "worst p95, 280 chars",
    evidence: `Native release build. A 4 KB message: ${formatMs(worstLong)}. Judge on every keystroke.`,
    glyph: "candle",
  },
  {
    numeral: "III",
    value: "No AI",
    label: "runtime",
    evidence: `Fixed integer tables. ${formatMegabytes(performance.peak_rss_bytes, 0)} peak resident.`,
    glyph: "censer",
  },
  {
    numeral: "IV",
    value: "Isomorphic",
    label: "browser and node",
    evidence: "One entry for both. Rust from the crate.",
    glyph: "fleur",
  },
  {
    numeral: "V",
    value: `${formatPercent(lowestPrecision, 0)}+`,
    label: "calibration precision",
    evidence: `Every one of the ${precisions.length} measured languages, validation split.`,
    glyph: "sacredHeart",
  },
  {
    numeral: "VI",
    value: String(LANGUAGES.length),
    label: "tongues",
    evidence: "Spoken by over 85% of the world.",
    glyph: "roseWindow",
  },
];
