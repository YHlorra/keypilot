import * as React from "react";
import type { AgentPair } from "@/types/api";
import { StatCard } from "./UsageKpiCards";

interface UsageStatsSidebarProps {
  lifetimeTotal: number;
  periodTotal: number;
  selectedRange: string;
  peakDay: number;
  peakDayLabel?: string;
  activeDays: number;
  topAgentPairs?: AgentPair[];
}

function formatNumber(n: number): string {
  if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(2)}B`;
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
  if (n >= 1_000) return n.toLocaleString();
  return String(n);
}

export const UsageStatsSidebar = React.memo(function UsageStatsSidebar({
  lifetimeTotal,
  periodTotal,
  selectedRange,
  peakDay,
  peakDayLabel,
  activeDays,
  topAgentPairs = [],
}: UsageStatsSidebarProps) {
  return (
    <div className="flex flex-col gap-3">
      <StatCard label="All-time" value={formatNumber(lifetimeTotal)} />
      <StatCard label="Period" value={formatNumber(periodTotal)} subLabel={selectedRange} />
      <StatCard label="Peak day" value={formatNumber(peakDay)} subLabel={peakDayLabel} />
      <StatCard label="Active days" value={activeDays.toLocaleString()} />

      {topAgentPairs.length > 0 && (
        <div className="flex flex-col rounded-sm border border-border bg-card px-4 py-3">
          <span className="text-xs text-muted-foreground font-medium uppercase tracking-wider mb-3">
            Top agents
          </span>
          <div className="flex flex-col gap-2">
            {topAgentPairs.slice(0, 5).map((pair, idx) => (
              <div key={idx} className="flex items-center gap-2 min-w-0">
                <span className="text-xs text-muted-foreground shrink-0 w-4">{idx + 1}</span>
                <span className="text-xs font-medium truncate">{pair.agent_type}</span>
                <span className="text-xs text-muted-foreground font-mono shrink-0">
                  {formatNumber(pair.total_tokens)}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
});
