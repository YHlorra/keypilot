import * as React from "react";
import { useMemo } from "react";
import { formatTokens } from "@/lib/format";

interface HeatmapCalendarProps {
  /** Map of date string "YYYY-MM-DD" -> token count */
  dateMap: Map<string, number>;
}

// Heatmap intensity ramp using alpha on primary color
const INTENSITY_LEVELS = [0, 0.1, 0.3, 0.5, 0.7, 0.9] as const;

const CELL_SIZE = 14;

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
const MONTH_NAMES = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

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

// Build a `string[]` indexed by week column: month name where the column starts a new month, else empty string.
function buildMonthLabelsByCol(weeks: { date: string; count: number }[][]): string[] {
  const labels = new Array<string>(weeks.length).fill('');
  let lastMonth = -1;
  weeks.forEach((week, i) => {
    const month = new Date(week[0].date).getMonth();
    if (month !== lastMonth) {
      labels[i] = MONTH_NAMES[month];
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
  const monthLabelByCol = useMemo(() => buildMonthLabelsByCol(weeks), [weeks]);

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

  return (
    <div className="flex flex-col gap-2 min-w-0">
      <div className="flex min-w-0 overflow-x-auto">
        {/* Day labels */}
        <div className="flex flex-col gap-[5px] mr-2 pt-4">
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
        <div className="flex flex-1 min-w-0 justify-between">
          {weeks.map((week, weekIdx) => (
            <div key={weekIdx} className="grid grid-rows-[16px_repeat(7,14px)] gap-[5px]">
              <span className="text-[10px] leading-4 whitespace-nowrap text-muted-foreground">
                {monthLabelByCol[weekIdx]}
              </span>
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
          <div className="text-muted-foreground">{formatTokens(tooltip.count)}</div>
        </div>
      )}
    </div>
  );
});
