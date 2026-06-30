import * as React from "react";
import { useMemo } from "react";
import type { UsageSummary } from "@/types/api";
import { formatNumber } from "@/lib/format";

interface KpiCardProps {
  label: string;
  value: number;
  unit?: string;
  subLabel?: string;
  /** Pass true to slightly emphasize the middle "primary" card */
  emphasized?: boolean;
}

const KpiCard = React.memo(function KpiCard({ label, value, unit, subLabel, emphasized }: KpiCardProps) {
  return (
    <div
      className={`
        flex flex-col rounded-sm border border-border bg-card px-5 py-4
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
        {formatNumber(value)}
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
    <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
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
    </div>
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


