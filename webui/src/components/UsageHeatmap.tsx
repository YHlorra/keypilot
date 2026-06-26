import * as React from "react";

export interface HeatmapCell {
  hour: number;           // 0-23
  agentType: string;
  tokens: number;
  costUsd: number;
  requestCount: number;
}

interface UsageHeatmapProps {
  data: HeatmapCell[];
  date: string;
  loading?: boolean;
}

function getCellOpacity(tokens: number, maxTokens: number): number {
  if (maxTokens === 0) return 0.05;
  const ratio = tokens / maxTokens;
  return 0.05 + ratio * 0.75; // 0.05 min, 0.8 max
}

function formatTokens(tokens: number): string {
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}M`;
  if (tokens >= 1_000) return `${(tokens / 1_000).toFixed(1)}K`;
  return tokens.toString();
}

function formatCost(costUsd: number): string {
  return `$${costUsd.toFixed(2)}`;
}

function formatHour(hour: number): string {
  return `${hour.toString().padStart(2, "0")}:00`;
}

const HOUR_LABELS = [0, 6, 12, 18];

export const UsageHeatmap = React.memo(function UsageHeatmap({
  data,
  date,
  loading = false,
}: UsageHeatmapProps) {
  const [tooltip, setTooltip] = React.useState<{
    visible: boolean;
    x: number;
    y: number;
    cell: HeatmapCell;
  } | null>(null);

  // Derive unique agent types preserving order
  const agentTypes = React.useMemo(() => {
    const seen = new Set<string>();
    for (const cell of data) {
      if (!seen.has(cell.agentType)) seen.add(cell.agentType);
    }
    return Array.from(seen);
  }, [data]);

  const maxTokens = React.useMemo(() => {
    return Math.max(0, ...data.map((c) => c.tokens));
  }, [data]);

  // Build lookup: `${hour}-${agentType}` -> cell
  const cellMap = React.useMemo(() => {
    const map = new Map<string, HeatmapCell>();
    for (const cell of data) {
      map.set(`${cell.hour}-${cell.agentType}`, cell);
    }
    return map;
  }, [data]);

  const handleMouseEnter = React.useCallback(
    (e: React.MouseEvent<HTMLDivElement>, cell: HeatmapCell) => {
      const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
      setTooltip({ visible: true, x: rect.left + rect.width / 2, y: rect.top, cell });
    },
    []
  );

  const handleMouseLeave = React.useCallback(() => {
    setTooltip(null);
  }, []);

  if (loading) {
    return (
      <div className="flex flex-col gap-2">
        <div className="text-sm text-muted-foreground">{date}</div>
        <div
          className="grid gap-0.5"
          style={{ gridTemplateColumns: "repeat(24, 1fr)" }}
        >
          {Array.from({ length: 72 }).map((_, i) => (
            <div
              key={i}
              data-skeleton="true"
              className="h-3 w-3 rounded-sm bg-muted animate-pulse"
            />
          ))}
        </div>
      </div>
    );
  }

  if (data.length === 0) {
    return (
      <div className="flex flex-col gap-2">
        <div className="text-sm text-muted-foreground">{date}</div>
        <div className="flex items-center justify-center h-32 text-muted-foreground text-sm">
          No hourly data for {date}
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {/* Date label */}
      <div className="text-sm text-muted-foreground">{date}</div>

      {/* Hour labels */}
      <div className="flex" style={{ paddingLeft: "80px" }}>
        {Array.from({ length: 24 }).map((_, hour) => {
          if (!HOUR_LABELS.includes(hour)) return <div key={hour} className="flex-1" />;
          return (
            <div key={hour} className="flex-1 text-xs text-muted-foreground text-center">
              {hour}
            </div>
          );
        })}
      </div>

      {/* Grid */}
      <div className="flex flex-col gap-0.5">
        {agentTypes.map((agentType) => (
          <div key={agentType} className="flex items-center gap-0.5">
            {/* Agent type label */}
            <div
              className="w-20 text-xs text-muted-foreground truncate pr-2"
              style={{ textAlign: "right" }}
            >
              {agentType}
            </div>

            {/* Cells for this agent type */}
            <div
              className="grid gap-0.5 flex-1"
              style={{ gridTemplateColumns: "repeat(24, 1fr)" }}
            >
              {Array.from({ length: 24 }).map((_, hour) => {
                const cell = cellMap.get(`${hour}-${agentType}`);
                const tokens = cell?.tokens ?? 0;
                const opacity = getCellOpacity(tokens, maxTokens);

                return (
                  <div
                    key={hour}
                    data-hour={hour}
                    data-agent-type={agentType}
                    className="h-3 rounded-sm cursor-pointer transition-opacity hover:opacity-100"
                    style={{
                      backgroundColor: `rgba(var(--color-primary-rgb, 0, 122, 255), ${opacity})`,
                    }}
                    onMouseEnter={(e) =>
                      cell && handleMouseEnter(e, cell)
                    }
                    onMouseLeave={handleMouseLeave}
                  />
                );
              })}
            </div>
          </div>
        ))}
      </div>

      {/* Tooltip */}
      {tooltip?.visible && (
        <div
          className="fixed z-50 bg-background border border-border rounded-md shadow-lg px-3 py-2 text-xs pointer-events-none"
          style={{
            left: tooltip.x,
            top: tooltip.y - 8,
            transform: "translate(-50%, -100%)",
          }}
        >
          <div className="font-medium text-foreground mb-1">
            {formatHour(tooltip.cell.hour)} -- {tooltip.cell.agentType}
          </div>
          <div className="text-muted-foreground">
            <div>Tokens: {formatTokens(tooltip.cell.tokens)}</div>
            <div>Cost: {formatCost(tooltip.cell.costUsd)}</div>
            <div>Requests: {tooltip.cell.requestCount}</div>
          </div>
        </div>
      )}
    </div>
  );
});