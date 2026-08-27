import type { ContractReport, PerformanceFixture, RoutingReport } from "./reports";

const NANOSECONDS_PER_MILLISECOND = 1_000_000;

function median(values: readonly number[]): number {
  const sorted = values.toSorted((left, right) => left - right);
  const middle = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 1) return sorted[middle];
  return (sorted[middle - 1] + sorted[middle]) / 2;
}

function p95Values(fixtures: Record<string, PerformanceFixture>, suffix: string): number[] {
  return Object.entries(fixtures)
    .filter(([name]) => name.endsWith(suffix))
    .map(([, fixture]) => fixture.p95_nanoseconds);
}

export function medianP95Ms(fixtures: Record<string, PerformanceFixture>, suffix: string): number {
  return median(p95Values(fixtures, suffix)) / NANOSECONDS_PER_MILLISECOND;
}

export function worstP95Ms(fixtures: Record<string, PerformanceFixture>, suffix: string): number {
  return Math.max(...p95Values(fixtures, suffix)) / NANOSECONDS_PER_MILLISECOND;
}

export function fixtureCount(fixtures: Record<string, PerformanceFixture>, suffix: string): number {
  return p95Values(fixtures, suffix).length;
}

export function caseTotals(report: ContractReport): { total: number; passed: number } {
  const cases = Object.values(report.languages).flatMap((language) => language.cases);
  return { total: cases.length, passed: cases.filter((entry) => entry.passed).length };
}

export function routingTotals(report: RoutingReport): { knownPrecision: number; unknownRate: number; misrouteRate: number; rows: number } {
  return {
    knownPrecision: report.supported.known_route_precision.value,
    unknownRate: report.supported.unknown_rate.value,
    misrouteRate: report.supported.misroute_rate.value,
    rows: report.supported.rows,
  };
}

export function nanosecondsToMs(nanoseconds: number): number {
  return nanoseconds / NANOSECONDS_PER_MILLISECOND;
}
