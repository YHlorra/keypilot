/** Abbreviate token count: 1234 -> "1.2K" | 1.5M -> "1.5M" | 2B -> "2.0B" */
export function formatTokens(n: number): string {
  if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(1)}B`;
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

/** Format integer with locale separators: 3726 -> "3,726" */
export function formatNumber(n: number): string {
  if (n >= 1_000) return n.toLocaleString();
  return String(n);
}
