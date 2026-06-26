import * as React from "react";
import { useUsageSummary } from "@/hooks/useUsage";
import { UsageTimeSeries } from "./UsageTimeSeries";
import { Icon } from "./Icon";
import type { AgentPair, TokenBreakdown } from "@/types/api";

// -------------------------------------------------------------------
// Pie chart helpers (extracted for testability)
// -------------------------------------------------------------------

export type PieSegment = {
  key: string;
  label: string;
  value: number;
  color: string;
};

export function calculateArcPath(
  startAngle: number,
  endAngle: number,
  radius: number = 80
): string {
  const cx = 100;
  const cy = 100;
  const startRad = (startAngle - 90) * (Math.PI / 180);
  const endRad = (endAngle - 90) * (Math.PI / 180);
  const x1 = cx + radius * Math.cos(startRad);
  const y1 = cy + radius * Math.sin(startRad);
  const x2 = cx + radius * Math.cos(endRad);
  const y2 = cy + radius * Math.sin(endRad);
  const largeArc = endAngle - startAngle > 180 ? 1 : 0;
  return `M ${cx} ${cy} L ${x1} ${y1} A ${radius} ${radius} 0 ${largeArc} 1 ${x2} ${y2} Z`;
}

export function buildPieSegments(breakdown: TokenBreakdown): PieSegment[] {
  const segments: PieSegment[] = [
    { key: "input", label: "Input", value: breakdown.input ?? 0, color: "#3b82f6" },
    { key: "output", label: "Output", value: breakdown.output ?? 0, color: "#22c55e" },
    { key: "cache_read", label: "Cache Read", value: breakdown.cache_read ?? 0, color: "#eab308" },
    { key: "cache_creation", label: "Cache Creation", value: breakdown.cache_creation ?? 0, color: "#f97316" },
    { key: "reasoning", label: "Reasoning", value: breakdown.reasoning ?? 0, color: "#a855f7" },
  ];
  return segments;
}

// -------------------------------------------------------------------
// Component
// -------------------------------------------------------------------

export interface UsageDetailPanelProps {
  agentPair: AgentPair | null;
  onClose: () => void;
  dateRange?: "7d" | "30d" | "90d";
}

function getDateRange(range: "7d" | "30d" | "90d"): { start_date: string; end_date: string } {
  const end = new Date();
  const start = new Date();
  const days = range === "7d" ? 7 : range === "30d" ? 30 : 90;
  start.setDate(start.getDate() - days);
  return {
    start_date: start.toISOString().split("T")[0],
    end_date: end.toISOString().split("T")[0],
  };
}

export function UsageDetailPanel({
  agentPair,
  onClose,
  dateRange = "30d",
}: UsageDetailPanelProps) {
  const [range, setRange] = React.useState<"7d" | "30d" | "90d">(dateRange);

  // Fetch daily series via useUsageSummary -- returns { daily_series }
  const { data: summary, isLoading } = useUsageSummary({
    agent_type: agentPair?.agent_type,
    model: agentPair?.model,
    ...getDateRange(range),
  });

  // Close on Escape key
  React.useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape" && agentPair !== null) onClose();
    };
    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [agentPair, onClose]);

  if (agentPair === null) return null;

  const total =
    (agentPair.token_breakdown.input ?? 0) +
    (agentPair.token_breakdown.output ?? 0) +
    (agentPair.token_breakdown.cache_read ?? 0) +
    (agentPair.token_breakdown.cache_creation ?? 0) +
    (agentPair.token_breakdown.reasoning ?? 0);

  const segments = buildPieSegments(agentPair.token_breakdown);
  let currentAngle = 0;

  return (
    <div
      data-testid="detail-panel-backdrop"
      onClick={onClose}
      className="fixed inset-0 z-50"
    >
      <div
        data-testid="detail-panel"
        className="absolute right-0 h-full w-[480px] bg-background border-l border-border overflow-y-auto"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-border">
          <div>
            <div data-testid="panel-title" className="font-semibold">
              {agentPair.agent_type} + {agentPair.model}
            </div>
            <div data-testid="panel-subtitle" className="text-sm text-muted-foreground">
              {agentPair.total_tokens.toLocaleString()} tokens · ${agentPair.total_cost_usd.toFixed(4)}
            </div>
          </div>
          <button
            onClick={onClose}
            data-testid="close-btn"
            className="p-1 rounded hover:bg-accent transition-colors"
          >
            <Icon name="x" className="w-4 h-4" />
          </button>
        </div>

        {/* Range Toggle */}
        <div data-testid="range-toggle" className="flex gap-2 px-4 py-3">
          {(["7d", "30d", "90d"] as const).map((r) => (
            <button
              key={r}
              data-testid={`range-btn-${r}`}
              onClick={() => setRange(r)}
              className={`px-3 py-1 rounded text-sm ${
                range === r
                  ? "bg-primary text-primary-foreground"
                  : "bg-secondary"
              }`}
            >
              {r}
            </button>
          ))}
        </div>

        {/* Section 1: Daily Time Series */}
        <div className="px-4 py-3">
          <h3 className="text-sm font-medium mb-2">Daily Usage</h3>
          <UsageTimeSeries
            dailySeries={summary?.daily_series ?? []}
            range={range}
            isLoading={isLoading}
          />
        </div>

        {/* Section 2: Token Breakdown Pie */}
        <div className="px-4 py-3">
          <h3 className="text-sm font-medium mb-2">Token Breakdown</h3>
          <svg
            data-testid="pie-chart"
            viewBox="0 0 200 200"
            className="w-48 h-48 mx-auto"
          >
            {segments.map((seg) => {
              if (total === 0 || seg.value === 0) return null;
              const angle = (seg.value / total) * 360;
              const path = calculateArcPath(currentAngle, currentAngle + angle);
              currentAngle += angle;
              return <path key={seg.key} d={path} fill={seg.color} />;
            })}
          </svg>
          <div data-testid="pie-legend" className="mt-4 space-y-2">
            {segments.map((seg) => {
              const pct = total > 0 ? ((seg.value / total) * 100).toFixed(1) : "0.0";
              return (
                <div
                  key={seg.key}
                  data-testid={`legend-${seg.key}`}
                  className="flex items-center gap-2 text-sm"
                >
                  <span
                    className="w-3 h-3 rounded"
                    style={{ backgroundColor: seg.color }}
                  />
                  <span>{seg.label}</span>
                  <span className="text-muted-foreground">{pct}%</span>
                  <span className="text-muted-foreground">
                    {seg.value.toLocaleString()}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
