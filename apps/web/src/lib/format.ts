const integer = new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 });

export function formatInt(value: number): string {
  return integer.format(value);
}

export function formatPercent(ratio: number, digits = 1): string {
  return `${(ratio * 100).toFixed(digits)}%`;
}

export function formatBytes(bytes: number): string {
  return `${formatInt(bytes)} B`;
}

export function formatMegabytes(bytes: number, digits = 2): string {
  return `${(bytes / 1_000_000).toFixed(digits)} MB`;
}

export function formatKibibytes(bytes: number): string {
  return `${Math.round(bytes / 1024)} KiB`;
}

export function formatMs(milliseconds: number, digits = 2): string {
  return `${milliseconds.toFixed(digits)} ms`;
}
