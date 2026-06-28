import * as React from "react";
import { useMemo } from "react";

interface HeatmapCalendarProps {
  /** Map of date string "YYYY-MM-DD" -> request count */
  dateMap: Map<string, number>;
}

// Heatmap intensity ramp using alpha on primary color
const INTENSITY_LEVELS = [0, 0.1, 0.3, 0.5, 0.7, 0.9] as const;

function getIntensityAlpha(count: number, maxCount: number): number {
  if (maxCount === 0 || count === 0) return 0.05;
  const ratio = count / maxCount;
  if (ratio < 0.2) return INTENSITY_LEVELS[1];
  if (ratio < 0.4) return INTENSITY_LEVELS[2];
  if (ratio < 0.6) return INTENSITY_LEVELS[3];
  if (ratio < 0.8) return INTENSITY_LEVELS[4];
  return INTENSITY_LEVELS[5];
}

const DAY_LABELS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

// Build a 7-row × N-column grid of { date, count } for the last ~26 weeks (182 days)
function buildCalendarGrid(dateMap: Map<string, number>): { date: string; count: number }[][] {
  const today = new Date();
  const endDate = new Date(today);
  // Start from the beginning of the week 26 weeks ago
  const startDate = new Date(today);
  startDate.setDate(startDate.getDate() - 181); // ~26 weeks
  // Adjust to start of week (Monday)
  const dayOfWeek = startDate.getDay(); // 0 = Sunday
  const offset = dayOfWeek === 0 ? -6 : 1 - dayOfWeek; // adjust to Monday
  startDate.setDate(startDate.getDate() + offset);

  const weeks: { date: string; count: number }[][] = [];
  let current = new Date(startDate);

  while (current <= endDate) {
    const week: { date: string; count: number }[] = [];
    for (let day = 0; day < 7; day++) {
      const iso = current.toISOString().split("T")[0];
      week.push({ date: iso, count: dateMap.get(iso) ?? 0 });
      current.setDate(current.getDate() + 1);
    }
    weeks.push(week);
  }

  return weeks;
}

// Get month labels from calendar weeks
function getMonthLabels(weeks: { date: string; count: number }[][]): { label: string; colIndex: number }[] {
  const labels: { label: string; colIndex: number }[] = [];
  let lastMonth = -1;

  weeks.forEach((week, i) => {
    const firstDay = new Date(week[0].date);
    const month = firstDay.getMonth();
    if (month !== lastMonth) {
      const monthNames = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
      labels.push({ label: monthNames[month], colIndex: i });
      lastMonth = month;
    }
  });

  return labels;
}

export const UsageHeatmapCalendar = React.memo(function UsageHeatmapCalendar({
  dateMap,
}: HeatmapCalendarProps) {
  const [tooltip, setTooltip] = React.useState<{ x: number; y: number; date: string; count: number } | null>(null);

  const weeks = useMemo(() => buildCalendarGrid(dateMap), [dateMap]);
  const monthLabels = useMemo(() => getMonthLabels(weeks), [weeks]);

  const maxCount = useMemo(() => {
    return Math.max(0, ...[...dateMap.values()]);
  }, [dateMap]);

  const handleMouseEnter = React.useCallback(
    (e: React.MouseEvent<HTMLDivElement>, date: string, count: number) => {
      const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
      setTooltip({ x: rect.left + rect.width / 2, y: rect.top, date, count });
    },
    []
  );

  const handleMouseLeave = React.useCallback(() => {
    setTooltip(null);
  }, []);

  if (dateMap.size === 0) {
    return (
      <div className="flex items-center justify-center h-32 text-sm text-muted-foreground">
        No activity yet
      </div>
    );
  }

  const CELL_SIZE = 13;
  const CELL_GAP = 3;

  return (
    <div className="flex flex-col gap-2">
      {/* Month labels */}
      <div className="flex" style={{ paddingLeft: 32 }}>
        {monthLabels.map(({ label, colIndex }, i) => (
          <div
            key={`${label}-${colIndex}`}
            className="text-xs text-muted-foreground"
            style={{
              marginLeft: colIndex === 0 ? 0 : (CELL_SIZE + CELL_GAP) * (colIndex - (monthLabels[i - 1]?.colIndex ?? 0) - 1) + (CELL_SIZE + CELL_GAP),
            }}
          >
            {label}
          </div>
        ))}
      </div>

      <div className="flex">
        {/* Day labels */}
        <div className="flex flex-col gap-[3px] mr-2">
          {DAY_LABELS.map((day) => (
            <div
              key={day}
              className="text-xs text-muted-foreground"
              style={{ height: CELL_SIZE, lineHeight: `${CELL_SIZE}px` }}
            >
              {day}
            </div>
          ))}
        </div>

        {/* Calendar grid */}
        <div className="flex gap-[3px] overflow-x-auto">
          {weeks.map((week, weekIdx) => (
            <div key={weekIdx} className="flex flex-col gap-[3px]">
              {week.map((day) => {
                const alpha = getIntensityAlpha(day.count, maxCount);
                return (
                  <div
                    key={day.date}
                    className="rounded-sm cursor-pointer transition-opacity hover:opacity-100"
                    style={{
                      width: CELL_SIZE,
                      height: CELL_SIZE,
                      backgroundColor: alpha === 0.05
                        ? "var(--color-border)"
                        : `color-mix(in srgb, var(--color-primary) ${alpha * 100}%, transparent)`,
                    }}
                    onMouseEnter={(e) => handleMouseEnter(e, day.date, day.count)}
                    onMouseLeave={handleMouseLeave}
                  />
                );
              })}
            </div>
          ))}
        </div>
      </div>

      {/* Intensity legend */}
      <div className="flex items-center gap-2 mt-1">
        <span className="text-xs text-muted-foreground">Less</span>
        <div className="flex gap-1">
          {[0.05, 0.1, 0.3, 0.5, 0.7, 0.9].map((alpha, i) => (
            <div
              key={i}
              className="rounded-sm"
              style={{
                width: CELL_SIZE,
                height: CELL_SIZE,
                backgroundColor: alpha === 0.05
                  ? "var(--color-border)"
                  : `color-mix(in srgb, var(--color-primary) ${alpha * 100}%, transparent)`,
              }}
            />
          ))}
        </div>
        <span className="text-xs text-muted-foreground">More</span>
      </div>

      {/* Tooltip */}
      {tooltip && (
        <div
          className="fixed z-50 bg-background border border-border rounded-sm shadow-lg px-2 py-1.5 text-xs pointer-events-none"
          style={{
            left: tooltip.x,
            top: tooltip.y - 8,
            transform: "translate(-50%, -100%)",
          }}
        >
          <div className="font-medium text-foreground">{tooltip.date}</div>
          <div className="text-muted-foreground">{tooltip.count.toLocaleString()} calls</div>
        </div>
      )}
    </div>
  );
});
