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
  /**
   * Tool x Model 二维聚合:agent_type -> model -> total_tokens.
   * 来自 PeriodsSummary.client_models。Task 10 会渲染实际表格;
   * 当前先打印到 console 并显示占位标题,确保 prop 流通但不变更现有布局。
   */
  clientModels?: Record<string, Record<string, number>>;
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
  clientModels,
}: UsageStatsSidebarProps) {
  // 占位:Task 10 在此渲染 Tool x Model 表格。当前只观察 prop 流通情况。
  React.useEffect(() => {
    if (clientModels && Object.keys(clientModels).length > 0) {
      // eslint-disable-next-line no-console
      console.log("[UsageStatsSidebar] clientModels received (render deferred to Task 10)", clientModels);
    }
  }, [clientModels]);

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

      {/* Tool x Model breakdown -- 占位标题,实际渲染留 Task 10 */}
      {clientModels !== undefined && Object.keys(clientModels).length > 0 && (
        <div className="flex flex-col rounded-sm border border-border bg-card px-4 py-3">
          <span className="text-xs text-muted-foreground font-medium uppercase tracking-wider">
            Tool x Model Breakdown
          </span>
          <span className="text-[10px] text-muted-foreground mt-1">(rendered in Task 10)</span>
        </div>
      )}
    </div>
  );
});
