import snapshot from "../data/evidence.json";

export type Matrix = { false_negative: number; false_positive: number; true_negative: number; true_positive: number };
export type BenchmarkResult = {
  matrix: Matrix;
  metrics: { precision: number | null; recall: number | null; f1: number | null; specificity: number | null };
};
type BenchmarkReport = Omit<typeof snapshot.benchmark, "test" | "validation"> & {
  test: { languages: Record<string, BenchmarkResult>; pooled: BenchmarkResult };
  validation: { languages: Record<string, BenchmarkResult>; pooled: BenchmarkResult };
};
export type PerformanceFixture = { p95_nanoseconds: number };
export type RoutingReport = Omit<typeof snapshot.routing, "languages"> & {
  languages: Record<string, typeof snapshot.routing.supported>;
};

export const snapshotPath = "apps/web/src/data/evidence.json";
export const benchmark: BenchmarkReport = snapshot.benchmark;
export const performance = snapshot.performance;
export const sizes = snapshot.sizes;
export const browser = snapshot.browser;
export const routing: RoutingReport = snapshot.routing;
export const behavior = snapshot.behavior;
export const smoke: Omit<typeof snapshot.smoke, "languages"> & {
  languages: Record<string, { cases: { text: string; expected_nudge: boolean }[] }>;
} = snapshot.smoke;
