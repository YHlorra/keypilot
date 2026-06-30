import { useEffect, useMemo, useRef, useState } from "react";
import { formatTokens } from "@/lib/format";

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

// ---------------------------------------------------------------------------
// Layout constants (px — layout, not typography)
// ---------------------------------------------------------------------------
const PADDING = { top: 24, right: 24, bottom: 56, left: 64 };
const HEIGHT = 320;

// ---------------------------------------------------------------------------
// Date formatter
// ---------------------------------------------------------------------------
function formatDate(dateStr: string, range: UsageTimeSeriesProps["range"]): string {
  const [, month, day] = dateStr.split("-");
  if (range === "7d" || range === "30d") {
    return `${month}-${day}`;
  }
  return `${month}/${day}`;
}

// ---------------------------------------------------------------------------
// "Nice" tick algorithm: round rawStep to 1/2/5 × 10^n
// ---------------------------------------------------------------------------
function niceStep(rawStep: number): number {
  if (rawStep <= 0) return 1;
  const magnitude = Math.pow(10, Math.floor(Math.log10(rawStep)));
  const normalized = rawStep / magnitude;
  let nice: number;
  if (normalized <= 1) nice = 1;
  else if (normalized <= 2.5) nice = 2;
  else if (normalized <= 5) nice = 5;
  else nice = 10;
  return nice * magnitude;
}

// ---------------------------------------------------------------------------
// Skeleton loading placeholder
// ---------------------------------------------------------------------------
function SkeletonChart({ height }: { height: number }) {
  return (
    <svg width="100%" height={height} className="text-muted">
      {Array.from({ length: 12 }, (_, i) => (
        <rect
          key={i}
          x={PADDING.left + i * ((100 - PADDING.left - PADDING.right) / 12)}
          y={PADDING.top + Math.sin(i * 1.5) * 20}
          width={6}
          height={40 + Math.cos(i * 2) * 20}
          fill="currentColor"
          opacity={0.2}
          className="animate-pulse"
        />
      ))}
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Empty state
// ---------------------------------------------------------------------------
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

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------
export function UsageTimeSeries({
  dailySeries,
  stacked = false,
  range,
  isLoading = false,
}: UsageTimeSeriesProps) {
  // ResizeObserver tracks container width in pixels
  const containerRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(800); // SSR fallback

  useEffect(() => {
    const ro = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setWidth(entry.contentRect.width);
      }
    });
    if (containerRef.current) {
      ro.observe(containerRef.current);
    }
    return () => ro.disconnect();
  }, []);

  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);
  const [tooltipPos, setTooltipPos] = useState({ x: 0, y: 0 });

  // Derived dimensions
  const innerWidth = width - PADDING.left - PADDING.right;
  const innerHeight = HEIGHT - PADDING.top - PADDING.bottom;

  // ---------------------------------------------------------------------------
  // Compute scales and paths
  // ---------------------------------------------------------------------------
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

    const xs = dailySeries.map(
      (_, i) => PADDING.left + (i / Math.max(dailySeries.length - 1, 1)) * innerWidth
    );
    const ys = dailySeries.map(
      (d) => PADDING.top + innerHeight - ((d.total_tokens - minVal) / rangeVal) * innerHeight
    );

    // Build line path (straight polyline — no Catmull-Rom)
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
        { key: "reasoning_tokens", color: "var(--color-accent)" },
      ];

      const bottomY = PADDING.top + innerHeight;
      const stackedYs: number[][] = tokenTypes.map(() =>
        dailySeries.map(() => bottomY)
      );

      tokenTypes.forEach(({ key }, layerIdx) => {
        dailySeries.forEach((d, i) => {
          const val = (d[key] as number) ?? 0;
          const prevTotal = stackedYs.slice(0, layerIdx).reduce(
            (sum: number, layer: number[]) => sum + (layer[i] as number),
            0
          );
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

  // ---------------------------------------------------------------------------
  // Y-axis ticks using "nice" step algorithm
  // ---------------------------------------------------------------------------
  const yTicks = useMemo(() => {
    if (maxValue === 0) return [];
    const rawStep = maxValue / 4;
    const step = niceStep(rawStep);
    const ticks: { value: number; y: number }[] = [];
    // Generate ticks from 0 up to maxValue
    for (let value = 0; value <= maxValue + step / 2; value += step) {
      const y = PADDING.top + innerHeight - (value / maxValue) * innerHeight;
      ticks.push({ value, y });
    }
    return ticks;
  }, [maxValue, innerHeight]);

  // ---------------------------------------------------------------------------
  // X-axis tick labels — stride = Math.ceil(n / 8)
  // ---------------------------------------------------------------------------
  const xTicks = useMemo(() => {
    if (dailySeries.length === 0) return [];
    const stride = Math.ceil(dailySeries.length / 8);
    return dailySeries
      .map((d, i) => ({ d, i }))
      .filter(({ i }) => i % stride === 0 || i === dailySeries.length - 1)
      .map(({ d, i }) => ({
        x: xScale[i],
        label: formatDate(d.date, range),
      }));
  }, [dailySeries, range, xScale]);

  // ---------------------------------------------------------------------------
  // Edge cases
  // ---------------------------------------------------------------------------
  if (isLoading) {
    return (
      <div ref={containerRef} className="w-full" style={{ height: HEIGHT }}>
        <SkeletonChart height={HEIGHT} />
      </div>
    );
  }

  if (dailySeries.length === 0) {
    return (
      <div ref={containerRef} className="w-full" style={{ height: HEIGHT }}>
        <EmptyState height={HEIGHT} />
      </div>
    );
  }

  // All-zero series
  if (maxValue === 0) {
    return (
      <div ref={containerRef} className="w-full" style={{ height: HEIGHT }}>
        <EmptyState height={HEIGHT} />
      </div>
    );
  }

  // ---------------------------------------------------------------------------
  // Interaction handlers
  // ---------------------------------------------------------------------------
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

  // Tooltip dimensions (approximate)
  const tooltipWidth = 120;
  const tooltipHeight = stacked && hoveredPoint?.input_tokens != null ? 88 : 64;
  // Clamp tooltip to container bounds
  const tooltipX = Math.min(tooltipPos.x + 12, width - tooltipWidth - 8);
  const tooltipY = Math.max(tooltipPos.y - tooltipHeight / 2, PADDING.top);

  // ---------------------------------------------------------------------------
  // 1-point series: skip line, render single dot + label
  // ---------------------------------------------------------------------------
  const isSinglePoint = dailySeries.length === 1;

  return (
    <div ref={containerRef} className="w-full relative" style={{ height: HEIGHT }}>
      {/* Pixel-coordinate SVG — no viewBox, no preserveAspectRatio */}
      <svg width={width} height={HEIGHT} className="overflow-visible">
        {/* Y grid lines */}
        {yTicks.map((tick, i) => (
          <line
            key={i}
            x1={PADDING.left}
            y1={tick.y}
            x2={width - PADDING.right}
            y2={tick.y}
            stroke="var(--color-border)"
            strokeWidth={1}
            opacity={0.3}
          />
        ))}

        {/* Y-axis "tokens" unit annotation — top-left of chart area */}
        <text
          x={PADDING.left}
          y={PADDING.top - 6}
          fontSize="var(--font-size-xs)"
          fill="var(--color-muted-foreground)"
        >
          tokens
        </text>

        {/* Y-axis labels */}
        {yTicks.map((tick, i) => (
          <text
            key={i}
            x={PADDING.left - 8}
            y={tick.y}
            textAnchor="end"
            dominantBaseline="middle"
            fontSize="var(--font-size-xs)"
            fill="var(--color-muted-foreground)"
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
            {!isSinglePoint && (
              <path
                d={areaPath}
                fill="var(--color-primary)"
                opacity={0.1}
              />
            )}
            {/* Line — skip for single point */}
            {!isSinglePoint && (
              <path
                d={linePath}
                fill="none"
                stroke="var(--color-primary)"
                strokeWidth={2}
                strokeLinejoin="round"
                strokeLinecap="round"
              />
            )}
          </>
        )}

        {/* Data points */}
        {xScale.map((x, i) => (
          <circle
            key={i}
            cx={x}
            cy={yScale[i]}
            r={hoveredIndex === i ? 5 : 3.5}
            fill="var(--color-primary)"
            stroke="var(--color-background)"
            strokeWidth={2}
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
            y={PADDING.top + innerHeight + 20}
            textAnchor="middle"
            fontSize="var(--font-size-xs)"
            fill="var(--color-muted-foreground)"
          >
            {tick.label}
          </text>
        ))}

        {/* Tooltip */}
        {hoveredPoint && (
          <g>
            {/* Tooltip background */}
            <rect
              x={tooltipX}
              y={tooltipY}
              width={tooltipWidth}
              height={tooltipHeight}
              fill="var(--color-background)"
              stroke="var(--color-border)"
              strokeWidth={1}
              rx={4}
            />
            <text
              x={tooltipX + tooltipWidth / 2}
              y={tooltipY + 12}
              textAnchor="middle"
              fontSize="var(--font-size-xs)"
              fill="var(--color-muted-foreground)"
            >
              {hoveredPoint.date}
            </text>
            <text
              x={tooltipX + tooltipWidth / 2}
              y={tooltipY + 28}
              textAnchor="middle"
              fontSize="var(--font-size-sm)"
              fontWeight="500"
              fill="var(--color-primary)"
            >
              {formatTokens(hoveredPoint.total_tokens)} tokens
            </text>
            <text
              x={tooltipX + tooltipWidth / 2}
              y={tooltipY + 44}
              textAnchor="middle"
              fontSize="var(--font-size-xs)"
              fill="var(--color-muted-foreground)"
            >
              ${hoveredPoint.total_cost_usd.toFixed(4)}
            </text>
            {!(stacked && hoveredPoint.input_tokens != null) && (
              <text
                x={tooltipX + tooltipWidth / 2}
                y={tooltipY + 60}
                textAnchor="middle"
                fontSize="var(--font-size-xs)"
                fill="var(--color-muted-foreground)"
              >
                {hoveredPoint.request_count} req
              </text>
            )}
            {stacked && hoveredPoint.input_tokens != null && (
              <>
                <text
                  x={tooltipX + tooltipWidth / 2}
                  y={tooltipY + 60}
                  textAnchor="middle"
                  fontSize="var(--font-size-xs)"
                  fill="var(--color-primary)"
                >
                  in:{formatTokens(hoveredPoint.input_tokens ?? 0)}
                </text>
                <text
                  x={tooltipX + tooltipWidth / 2}
                  y={tooltipY + 74}
                  textAnchor="middle"
                  fontSize="var(--font-size-xs)"
                  fill="var(--color-success)"
                >
                  out:{formatTokens(hoveredPoint.output_tokens ?? 0)}
                </text>
                <text
                  x={tooltipX + tooltipWidth / 2}
                  y={tooltipY + 88}
                  textAnchor="middle"
                  fontSize="var(--font-size-xs)"
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
