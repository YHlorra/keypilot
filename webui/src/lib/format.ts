
export function formatTokens(n: number): string {
  if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(1)}B`;
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}


export function formatNumber(n: number): string {
  if (n >= 1_000) return n.toLocaleString();
  return String(n);
}














export function formatLocalDate(date: Date): string {
  if (!Number.isFinite(date.getTime())) return "1970-01-01";
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}






export function formatRelative(
  d: Date | number | string,
  mode: "suffix" | "bare" = "suffix"
): string {
  const ts =
    typeof d === "string" ? Date.parse(d) :
    d instanceof Date ? d.getTime() : d;
  const diffMs = ts - Date.now();
  const abs = Math.abs(diffMs);

  let unit: Intl.RelativeTimeFormatUnit = "second";
  if (abs >= 31_536_000_000) unit = "year";
  else if (abs >= 2_592_000_000) unit = "month";
  else if (abs >= 86_400_000) unit = "day";
  else if (abs >= 3_600_000) unit = "hour";
  else if (abs >= 60_000) unit = "minute";

  const value = Math.round(diffMs / (
    unit === "year" ? 31_536_000_000 :
    unit === "month" ? 2_592_000_000 :
    unit === "day" ? 86_400_000 :
    unit === "hour" ? 3_600_000 :
    60_000
  ));

  const formatted = new Intl.RelativeTimeFormat("en", { numeric: "auto" })
    .format(value, unit);

  if (mode === "bare") {
    
    return formatted.replace(/^(in )?/, "").replace(/ ago$/, "").replace(/^-/, "");
  }
  return formatted;
}
