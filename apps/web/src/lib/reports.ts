import { root } from "astro:config/server";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

export type Ratio = { numerator: number; denominator: number; value: number };
export type Matrix = { false_negative: number; false_positive: number; true_negative: number; true_positive: number };

export type ValidationLanguage = {
  language: string;
  split: string;
  gates: { false_warning_passed: boolean; has_true_positive: boolean; precision_passed: boolean };
  matrix: Matrix;
  metrics: {
    f1: number;
    false_warning_rate: number;
    precision: number;
    recall: number;
    specificity: number;
    projected_precision_1_percent: number;
    projected_precision_5_percent: number;
  };
};
export type ValidationReport = {
  evidence_status: string;
  split: string;
  languages: Record<string, ValidationLanguage>;
  pooled_matrix: Matrix;
};

export type PerformanceFixture = {
  input_bytes: number;
  samples: number;
  p50_nanoseconds: number;
  p95_nanoseconds: number;
  p99_nanoseconds: number;
  maximum_nanoseconds: number;
  checks_per_second: number;
  bytes_per_second: number;
  peak_rss_bytes: number;
  latency_gate_passed: boolean;
};
export type PerformanceReport = {
  evidence_status: string;
  computer: string;
  target_triple: string;
  rust_version: string;
  all_latency_gates_passed: boolean;
  peak_rss_bytes: number;
  fixtures: Record<string, PerformanceFixture>;
};

export type SizedFile = { bytes: number; relative_path: string; sha256: string };
export type SizeReport = {
  evidence_status: string;
  all_gates_passed: boolean;
  target_triple: string;
  artifacts: Record<string, SizedFile>;
  binary: SizedFile;
  hurtlex: Record<string, SizedFile>;
};

export type CompressedFile = { raw_bytes: number; gzip_bytes: number; brotli_bytes: number; relative_path: string; sha256: string };
export type BrowserBuild = {
  wasm: CompressedFile;
  javascript_glue: CompressedFile;
  raw_total_bytes: number;
  gzip_total_bytes: number;
  brotli_total_bytes: number;
};
export type BrowserReport = {
  evidence_status: string;
  status: string;
  browser_engine: string;
  browser_version: string;
  wasm_bindgen_version: string;
  supplied_case_count: number;
  passed_case_count: number;
  auto_case_count: number;
  passed_auto_case_count: number;
  unknown_case_count: number;
  passed_unknown_case_count: number;
  runtime_network_requests: string[];
  browser_builds: { full: BrowserBuild; explicit_only: BrowserBuild };
};

export type RouteCounts = {
  rows: number;
  correct: number;
  unknown: number;
  misrouted: number;
  known_route_precision: Ratio;
  route_accuracy: Ratio;
  unknown_rate: Ratio;
  misroute_rate: Ratio;
};
export type TimingGroup = {
  samples: number;
  p50_nanoseconds: number;
  p95_nanoseconds: number;
  p99_nanoseconds: number;
  maximum_nanoseconds: number;
  checks_per_second: number;
};
export type RoutingReport = {
  evidence_status: string;
  computer: string;
  target_triple: string;
  cold_initialization_nanoseconds: number;
  corpus: { rows: number; supported_rows: number; unsupported_rows: number };
  supported: RouteCounts;
  unsupported: { rows: number; falsely_routed: number; rejected_as_unknown: number; unsupported_rejection_rate: Ratio };
  languages: Record<string, RouteCounts>;
  timing: { groups: Record<"unicode_scalars_280" | "utf8_bytes_4096", TimingGroup> };
  limitations: string[];
};

export type ContractCase = { case_id: string; text: string; expected_nudge: boolean; passed: boolean };
export type ContractReport = {
  evidence_status: string;
  languages: Record<string, { language: string; passed: boolean; cases: ContractCase[] }>;
};

export type SmokeCase = ContractCase & {
  language: string;
  suite: string;
  ok: boolean;
  score: number;
  should_nudge: boolean;
  threshold: number;
};
export type SmokeReport = {
  evidence_status: string;
  languages: Record<string, { language: string; passed: boolean; cases: SmokeCase[] }>;
};

type Loose = Record<string, unknown>;

const reportsDir = fileURLToPath(new URL("../../reports/", root));

const reports: Loose[] = readdirSync(reportsDir)
  .filter((name) => name.endsWith(".json"))
  .map((name) => JSON.parse(readFileSync(join(reportsDir, name), "utf8")) as Loose);

function pick<T>(label: string, matches: (report: Loose) => boolean): T {
  const hits = reports.filter(matches);
  if (hits.length !== 1) throw new Error(`expected one ${label} report under reports/, found ${hits.length}`);
  return hits[0] as T;
}

export const validation = pick<ValidationReport>("validation", (report) => report.evidence_status === "calibration_evidence");
export const performance = pick<PerformanceReport>("performance", (report) => "all_latency_gates_passed" in report);
export const sizes = pick<SizeReport>("size", (report) => "artifacts" in report && "all_gates_passed" in report);
export const browser = pick<BrowserReport>("browser", (report) => report.execution_environment === "actual_browser");
export const routing = pick<RoutingReport>("routing", (report) => "c_parity" in report);
export const behavior = pick<ContractReport>("behavior", (report) => report.evidence_status === "behavior_contract_evidence");
export const smoke = pick<SmokeReport>("smoke", (report) => report.evidence_status === "native_cli_smoke_evidence");
