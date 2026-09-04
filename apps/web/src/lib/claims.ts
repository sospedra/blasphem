import { formatMegabytes, formatMs, formatPercent } from "./format";
import { LANGUAGES } from "./languages";
import { worstP95Ms } from "./metrics";
import { benchmark, browser, performance, routing } from "./reports";

export type Claim = {
  numeral: string;
  value: string;
  label: string;
  evidence: string;
  glyph: "chalice" | "candle" | "censer" | "fleur" | "roseWindow" | "sacredHeart";
};

const explicitBytes = browser.browser_builds.explicit_only.brotli_total_bytes;
const routedBytes = browser.browser_builds.english_routed.brotli_total_bytes;
const worstShort = worstP95Ms(performance.fixtures, "-280");
const worstLong = worstP95Ms(performance.fixtures, "-4096");

export const CLAIMS: readonly Claim[] = [
  {
    numeral: "I",
    value: formatMegabytes(explicitBytes, 1),
    label: "English, without detection",
    evidence: `Brotli download. English with autodetect: ${formatMegabytes(routedBytes, 1)}.`,
    glyph: "chalice",
  },
  {
    numeral: "II",
    value: formatMs(worstShort),
    label: "short-message check",
    evidence: `Worst native p95, 280 characters. At 4 KB: ${formatMs(worstLong)}.`,
    glyph: "candle",
  },
  {
    numeral: "III",
    value: "Runs locally",
    label: "your text stays yours",
    evidence: "A compact model and rules. No API key or inference service.",
    glyph: "censer",
  },
  {
    numeral: "IV",
    value: "Client & server",
    label: "one Rust core",
    evidence: "Web, Swift, Android, React Native. Node.js, Python, Go, Rust.",
    glyph: "fleur",
  },
  {
    numeral: "V",
    value: benchmark.test.pooled.metrics.precision === null ? "Unmeasured" : formatPercent(benchmark.test.pooled.metrics.precision),
    label: "test precision",
    evidence: `Pooled test results. Recall: ${benchmark.test.pooled.metrics.recall === null ? "unmeasured" : formatPercent(benchmark.test.pooled.metrics.recall)}. See the benchmark below.`,
    glyph: "sacredHeart",
  },
  {
    numeral: "VI",
    value: String(LANGUAGES.length),
    label: "supported languages",
    evidence: `Estimated reach: ~80% of the world's population. ${formatPercent(routing.supported.known_route_precision.value, 2)} detection precision on assigned, supported-language sentences.`,
    glyph: "roseWindow",
  },
];
