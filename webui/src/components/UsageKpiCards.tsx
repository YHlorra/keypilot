import * as React from "react";
import { useMemo } from "react";
import type { UsageSummary } from "@/types/api";
import { formatNumber, formatTokens } from "@/lib/format";

interface KpiCardProps {
  label: string;
  value: number;
  unit?: string;
  subLabel?: string;
  /** Pass true to slightly emphasize the middle "primary" card */
  emphasized?: boolean;
  /** Override the formatted display string for the value (e.g. formatTokens output) */
  formattedValue?: string;
}

const KpiCard = React.memo(function KpiCard({ label, value, unit, subLabel, emphasized, formattedValue }: KpiCardProps) {
  return (
    <div
      className={`
        flex flex-col rounded-sm border border-border bg-card px-3 py-2.5 min-h-[76px]
        ${emphasized ? "border-[var(--color-primary)]/40" : ""}
      `}
      style={emphasized ? { backgroundColor: "var(--color-surface-sunken)" } : {}}
    >
      <span className="text-xs text-muted-foreground font-medium uppercase tracking-wider mb-1">
        {label}
      </span>
      <span
        className="font-semibold text-foreground leading-none flex items-baseline gap-1"
        style={{ fontSize: "var(--font-size-2xl)", letterSpacing: "var(--tracking-tight)" }}
      >
        {formattedValue ?? formatNumber(value)}
        {unit && <span className="text-sm font-normal text-muted-foreground">{unit}</span>}
      </span>
      {subLabel && (
        <span className="text-xs text-muted-foreground mt-1">{subLabel}</span>
      )}
    </div>
  );
});

interface UsageKpiCardsProps {
  today?: UsageSummary;
  month?: UsageSummary;
  allTime?: UsageSummary;
  monthLabel?: string;
  allTimeLabel?: string;
}

export const UsageKpiCards = React.memo(function UsageKpiCards({
  today,
  month,
  allTime,
  monthLabel,
  allTimeLabel,
}: UsageKpiCardsProps) {
  const todayLabel = useMemo(() => {
    const now = new Date();
    return `${now.getMonth() + 1}/${now.getDate()}`;
  }, []);

  return (
    <div className="grid grid-cols-2 sm:grid-cols-4 gap-[10px]">
      <KpiCard label="Today" value={today?.total_requests ?? 0} unit="requests" subLabel={todayLabel} />
      <KpiCard
        label="This Month"
        value={month?.total_requests ?? 0}
        unit="requests"
        emphasized
        subLabel={monthLabel ?? ""}
      />
      <KpiCard
        label="All Time"
        value={allTime?.total_requests ?? 0}
        unit="requests"
        subLabel={allTimeLabel ?? ""}
      />
      <AvgDayCard dailySeries={month?.daily_series} />
    </div>
  );
});

// ponytail: 修复 /30 稀释;Q4=B 锁定为 Month-to-Date (MTD) 语义。
const AvgDayCard = React.memo(function AvgDayCard({
  dailySeries,
}: {
  dailySeries?: { date: string; total_tokens?: number }[];
}) {
  const { avg, deltaLabel } = useMemo(() => {
    if (!dailySeries || dailySeries.length === 0) {
      return { avg: 0, deltaLabel: "" };
    }
    const sorted = [...dailySeries].sort((a, b) => b.date.localeCompare(a.date));
    const days = sorted.length;
    const sum = sorted.reduce((s, d) => s + (d.total_tokens ?? 0), 0);
    return {
      avg: sum / days,
      deltaLabel: `${days} day${days === 1 ? "" : "s"} so far`,
    };
  }, [dailySeries]);

  return (
    <KpiCard
      label="AVG / DAY (MTD)"
      value={avg}
      formattedValue={formatTokens(avg)}
      subLabel={deltaLabel}
    />
  );
});

// Standalone stat card for sidebar
interface StatCardProps {
  label: string;
  value: string | number;
  unit?: string;
  subLabel?: string;
}

export const StatCard = React.memo(function StatCard({ label, value, unit, subLabel }: StatCardProps) {
  const displayValue = typeof value === "number" ? formatNumber(value) : value;
  return (
    <div className="flex flex-col rounded-sm border border-border bg-card px-4 py-3">
      <span className="text-xs text-muted-foreground font-medium uppercase tracking-wider mb-0.5">
        {label}
      </span>
      <span
        className="font-semibold text-foreground leading-none flex items-baseline gap-1"
        style={{ fontSize: "var(--font-size-lg)", letterSpacing: "var(--tracking-tight)" }}
      >
        {displayValue}
        {unit && <span className="text-xs font-normal text-muted-foreground">{unit}</span>}
      </span>
      {subLabel && <span className="text-xs text-muted-foreground mt-0.5">{subLabel}</span>}
    </div>
  );
});


