import { useMemo, useState } from "react";

// DailySeriesPoint -- extended with optional breakdown fields for stacked mode
export interface DailySeriesPoint {
  date: string; // "YYYY-MM-DD"
  total_tokens: number;
  total_cost_usd: number;
  request_count: number;
  input_tokens?: number;
  output_tokens?: number;
  cache_read_tokens?: number;
  reasoning_tokens?: number;
}

export interface UsageTimeSeriesProps {
  dailySeries: DailySeriesPoint[];
  stacked?: boolean; // default false
  range: "7d" | "30d" | "90d" | "all";
  isLoading?: boolean;
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${Math.round(n / 1_000)}K`;
  return String(n);
}

function formatDate(dateStr: string, range: UsageTimeSeriesProps["range"]): string {
  const [, month, day] = dateStr.split("-");
  if (range === "7d" || range === "30d") {
    return `${month}-${day}`;
  }
  return `${month}/${day}`;
}

function getXTickDensity(range: UsageTimeSeriesProps["range"], dataLength: number): number {
  switch (range) {
    case "7d":
      return 1; // every day
    case "30d":
      return Math.ceil(dataLength / 10); // ~every 2-3 days
    case "90d":
      return 7; // weekly
    case "all":
      return 14; // bi-weekly
  }
}

// Skeleton loading placeholder
function SkeletonChart({ height }: { height: number }) {
  const rects = Array.from({ length: 5 }, (_, i) => ({
    x: 40 + i * ((100 - 80) / 4),
    y: 30 + Math.sin(i * 1.5) * 20,
    width: 8,
    height: 40 + Math.cos(i * 2) * 20,
  }));

  return (
    <svg
      width="100%"
      height={height}
      viewBox={`0 0 100 ${height}`}
      preserveAspectRatio="none"
      className="text-muted"
    >
      {rects.map((r, i) => (
        <rect
          key={i}
          x={r.x}
          y={r.y}
          width={r.width}
          height={r.height}
          fill="currentColor"
          opacity={0.2}
          className="animate-pulse"
        />
      ))}
    </svg>
  );
}

// Empty state
function EmptyState({ height }: { height: number }) {
  return (
    <div
      className="flex items-center justify-center text-muted-foreground text-sm"
      style={{ height }}
    >
      No data in selected range
    </div>
  );
}

export function UsageTimeSeries({
  dailySeries,
  stacked = false,
  range,
  isLoading = false,
}: UsageTimeSeriesProps) {
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);
  const [tooltipPos, setTooltipPos] = useState({ x: 0, y: 0 });

  const PADDING = { top: 20, right: 20, bottom: 60, left: 60 };
  const DEFAULT_HEIGHT = 400;
  const innerWidth = 100 - PADDING.left - PADDING.right;
  const innerHeight = DEFAULT_HEIGHT - PADDING.top - PADDING.bottom;

  // Compute scales and paths
  const { xScale, yScale, maxValue, linePath, areaPath, stackedLayers } = useMemo(() => {
    if (dailySeries.length === 0) {
      return {
        xScale: [] as number[],
        yScale: [] as number[],
        maxValue: 0,
        linePath: "",
        areaPath: "",
        stackedLayers: [] as { path: string; color: string }[],
      };
    }

    const maxVal = Math.max(...dailySeries.map((d) => d.total_tokens));
    const minVal = 0;
    const rangeVal = maxVal - minVal || 1;

    const xs = dailySeries.map((_, i) => PADDING.left + (i / Math.max(dailySeries.length - 1, 1)) * innerWidth);
    const ys = dailySeries.map(
      (d) => PADDING.top + innerHeight - ((d.total_tokens - minVal) / rangeVal) * innerHeight
    );

    // Build line path
    const linePoints = xs.map((x, i) => `${x},${ys[i]}`).join(" L ");
    const line = `M ${linePoints}`;

    // Build area path (close to bottom)
    const area = `${line} L ${xs[xs.length - 1]},${PADDING.top + innerHeight} L ${xs[0]},${PADDING.top + innerHeight} Z`;

    // Stacked layers
    const layers: { path: string; color: string }[] = [];
    if (stacked) {
      const tokenTypes: Array<{ key: keyof DailySeriesPoint; color: string }> = [
        { key: "input_tokens", color: "var(--color-primary)" },
        { key: "output_tokens", color: "var(--color-success)" },
        { key: "cache_read_tokens", color: "var(--color-link)" },
        { key: "reasoning_tokens", color: "var(--color-muted)" },
      ];

      // For stacked, we stack from bottom (y = PADDING.top + innerHeight) upward
      const bottomY = PADDING.top + innerHeight;
      const stackedYs: number[][] = tokenTypes.map(() =>
        dailySeries.map(() => bottomY)
      );

      tokenTypes.forEach(({ key }, layerIdx) => {
        dailySeries.forEach((d, i) => {
          const val = (d[key] as number) ?? 0;
          const prevTotal = stackedYs.slice(0, layerIdx).reduce((sum: number, layer: number[]) => sum + (layer[i] as number), 0);
          stackedYs[layerIdx][i] = bottomY - ((prevTotal + val) / maxVal) * innerHeight;
        });

        const layerPoints = xs.map((x, i) => `${x},${stackedYs[layerIdx][i]}`).join(" L ");
        const layerPath = `M ${layerPoints} L ${xs[xs.length - 1]},${PADDING.top + innerHeight} L ${xs[0]},${PADDING.top + innerHeight} Z`;
        layers.push({ path: layerPath, color: tokenTypes[layerIdx].color });
      });
    }

    return {
      xScale: xs,
      yScale: ys,
      maxValue: maxVal,
      linePath: line,
      areaPath: area,
      stackedLayers: layers,
    };
  }, [dailySeries, stacked, innerWidth, innerHeight]);

  // Y-axis ticks (5 ticks)
  const yTicks = useMemo(() => {
    if (maxValue === 0) return [];
    const ticks: { value: number; y: number }[] = [];
    for (let i = 0; i <= 4; i++) {
      const value = (maxValue * i) / 4;
      const y = PADDING.top + innerHeight - (i / 4) * innerHeight;
      ticks.push({ value, y });
    }
    return ticks;
  }, [maxValue, innerHeight]);

  // X-axis tick labels
  const xTicks = useMemo(() => {
    if (dailySeries.length === 0) return [];
    const density = getXTickDensity(range, dailySeries.length);
    return dailySeries
      .filter((_, i) => i % density === 0 || i === dailySeries.length - 1)
      .map((d) => {
        const originalIndex = dailySeries.indexOf(d);
        return {
          x: xScale[originalIndex],
          label: formatDate(d.date, range),
        };
      });
  }, [dailySeries, range, xScale]);

  const height = DEFAULT_HEIGHT;

  if (isLoading) {
    return (
      <div className="w-full" style={{ height, padding: 16 }}>
        <SkeletonChart height={height} />
      </div>
    );
  }

  if (dailySeries.length === 0) {
    return (
      <div className="w-full" style={{ height, padding: 16 }}>
        <EmptyState height={height} />
      </div>
    );
  }

  const handleMouseEnter = (index: number, _event: React.MouseEvent<SVGCircleElement>) => {
    const circleX = xScale[index];
    const circleY = yScale[index];
    setTooltipPos({ x: circleX, y: circleY });
    setHoveredIndex(index);
  };

  const handleMouseLeave = () => {
    setHoveredIndex(null);
  };

  const hoveredPoint = hoveredIndex !== null ? dailySeries[hoveredIndex] : null;

  return (
    <div className="w-full relative" style={{ height, padding: 16 }}>
      <svg
        width="100%"
        height={height}
        viewBox={`0 0 100 ${height}`}
        preserveAspectRatio="none"
        className="overflow-visible"
      >
        {/* Y grid lines */}
        {yTicks.map((tick, i) => (
          <line
            key={i}
            x1={PADDING.left}
            y1={tick.y}
            x2={100 - PADDING.right}
            y2={tick.y}
            stroke="currentColor"
            strokeWidth="0.2"
            opacity={0.2}
          />
        ))}

        {/* Y-axis labels */}
        {yTicks.map((tick, i) => (
          <text
            key={i}
            x={PADDING.left - 2}
            y={tick.y}
            textAnchor="end"
            dominantBaseline="middle"
            fontSize="8"
            fill="currentColor"
          >
            {formatTokens(tick.value)}
          </text>
        ))}

        {/* Stacked layers or single area */}
        {stacked ? (
          stackedLayers.map((layer, i) => (
            <path
              key={i}
              d={layer.path}
              fill={layer.color}
              opacity={0.4}
            />
          ))
        ) : (
          <>
            {/* Area fill */}
            <path
              d={areaPath}
              fill="var(--color-primary)"
              opacity={0.1}
            />
            {/* Line */}
            <path
              d={linePath}
              fill="none"
              stroke="var(--color-primary)"
              strokeWidth="0.3"
              strokeLinejoin="round"
              strokeLinecap="round"
            />
          </>
        )}

        {/* Data points */}
        {xScale.map((x, i) => (
          <circle
            key={i}
            cx={x}
            cy={yScale[i]}
            r={hoveredIndex === i ? 0.6 : 0.4}
            fill="var(--color-primary)"
            stroke="var(--color-background)"
            strokeWidth="0.2"
            className="cursor-pointer transition-all duration-75"
            onMouseEnter={(e) => handleMouseEnter(i, e)}
            onMouseLeave={handleMouseLeave}
          />
        ))}

        {/* X-axis labels — rendered after chart paths so they paint on top */}
        {xTicks.map((tick, i) => (
          <text
            key={i}
            x={tick.x}
            y={PADDING.top + innerHeight + 12}
            textAnchor="middle"
            fontSize="7"
            fill="currentColor"
          >
            {tick.label}
          </text>
        ))}

        {/* Tooltip */}
        {hoveredPoint && (
          <g>
            {/* Tooltip background */}
            <rect
              x={Math.min(tooltipPos.x - 15, 100 - PADDING.right - 30)}
              y={Math.max(tooltipPos.y - 20, PADDING.top)}
              width={30}
              height={stacked && hoveredPoint.input_tokens ? 22 : 16}
              fill="var(--color-background)"
              stroke="var(--color-border)"
              strokeWidth="0.3"
              rx="1"
            />
            <text
              x={Math.min(tooltipPos.x - 15, 100 - PADDING.right - 30) + 15}
              y={Math.max(tooltipPos.y - 20, PADDING.top) + 4}
              textAnchor="middle"
              fontSize="3"
              fill="currentColor"
            >
              {hoveredPoint.date}
            </text>
            <text
              x={Math.min(tooltipPos.x - 15, 100 - PADDING.right - 30) + 15}
              y={Math.max(tooltipPos.y - 20, PADDING.top) + 8}
              textAnchor="middle"
              fontSize="3"
              fill="var(--color-primary)"
            >
              {formatTokens(hoveredPoint.total_tokens)} tokens
            </text>
            <text
              x={Math.min(tooltipPos.x - 15, 100 - PADDING.right - 30) + 15}
              y={Math.max(tooltipPos.y - 20, PADDING.top) + 12}
              textAnchor="middle"
              fontSize="3"
              fill="currentColor"
            >
              ${hoveredPoint.total_cost_usd.toFixed(4)}
            </text>
            {!(stacked && hoveredPoint.input_tokens) && (
              <text
                x={Math.min(tooltipPos.x - 15, 100 - PADDING.right - 30) + 15}
                y={Math.max(tooltipPos.y - 20, PADDING.top) + 16}
                textAnchor="middle"
                fontSize="3"
                fill="currentColor"
              >
                {hoveredPoint.request_count} req
              </text>
            )}
            {stacked && hoveredPoint.input_tokens && (
              <>
                <text
                  x={Math.min(tooltipPos.x - 15, 100 - PADDING.right - 30) + 15}
                  y={Math.max(tooltipPos.y - 20, PADDING.top) + 16}
                  textAnchor="middle"
                  fontSize="2.5"
                  fill="var(--color-primary)"
                >
                  in:{formatTokens(hoveredPoint.input_tokens ?? 0)}
                </text>
                <text
                  x={Math.min(tooltipPos.x - 15, 100 - PADDING.right - 30) + 15}
                  y={Math.max(tooltipPos.y - 20, PADDING.top) + 19}
                  textAnchor="middle"
                  fontSize="2.5"
                  fill="var(--color-success)"
                >
                  out:{formatTokens(hoveredPoint.output_tokens ?? 0)}
                </text>
                <text
                  x={Math.min(tooltipPos.x - 15, 100 - PADDING.right - 30) + 15}
                  y={Math.max(tooltipPos.y - 20, PADDING.top) + 22}
                  textAnchor="middle"
                  fontSize="2.5"
                  fill="var(--color-link)"
                >
                  cache:{formatTokens(hoveredPoint.cache_read_tokens ?? 0)}
                </text>
              </>
            )}
          </g>
        )}
      </svg>
    </div>
  );
}