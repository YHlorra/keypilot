import * as React from "react";
import { useMemo } from "react";
import { formatTokens, formatLocalDate } from "@/lib/format";

interface HeatmapCalendarProps {
  
  dateMap: Map<string, number>;
}

const DAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MONTH_NAMES = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];


function buildCalendarGrid(dateMap: Map<string, number>): { date: string; count: number }[][] {
  const today = new Date();
  const endDate = new Date(today);
  
  const startDate = new Date(today);
  startDate.setDate(startDate.getDate() - 181); 
  
  const dayOfWeek = startDate.getDay(); 
  const offset = dayOfWeek === 0 ? -6 : 1 - dayOfWeek; 
  startDate.setDate(startDate.getDate() + offset);

  const weeks: { date: string; count: number }[][] = [];
  let current = new Date(startDate);

  while (current <= endDate) {
    const week: { date: string; count: number }[] = [];
    for (let day = 0; day < 7; day++) {
      const iso = formatLocalDate(current);
      week.push({ date: iso, count: dateMap.get(iso) ?? 0 });
      current.setDate(current.getDate() + 1);
    }
    weeks.push(week);
  }

  return weeks;
}


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


function intensityColor(count: number, maxCount: number): string {
  if (maxCount === 0 || count === 0) return "var(--color-border)";
  const ratio = count / maxCount;
  if (ratio < 0.2) return "color-mix(in srgb, var(--color-primary) 15%, var(--color-border))";
  if (ratio < 0.4) return "color-mix(in srgb, var(--color-primary) 35%, var(--color-border))";
  if (ratio < 0.6) return "color-mix(in srgb, var(--color-primary) 55%, var(--color-border))";
  if (ratio < 0.8) return "color-mix(in srgb, var(--color-primary) 80%, var(--color-border))";
  return "var(--color-primary)";
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

  const TOTAL_COLS = weeks.length;

  return (
    <div className="flex flex-col gap-2 min-w-0">
      {}
      <div
        className="grid w-full"
        style={{ gridTemplateColumns: `auto repeat(${TOTAL_COLS}, minmax(0, 1fr))`, gap: 5 }}
      >
        {}
        <div />

        {}
        <div className="relative h-4" style={{ gridColumn: `2 / span ${TOTAL_COLS}` }}>
          {monthLabelByCol.map((label, i) =>
            label ? (
              <span
                key={i}
                className="absolute text-xs leading-4 text-muted-foreground whitespace-nowrap"
                style={{ left: `${(i / TOTAL_COLS) * 100}%` }}
              >
                {label}
              </span>
            ) : null
          )}
        </div>

        {}
        {DAYS.map((day, d) => (
          <React.Fragment key={day}>
            {}
            <div
              className="text-xs text-muted-foreground text-right pr-2 flex items-center leading-tight whitespace-nowrap"
              style={{ minWidth: 36 }}
            >
              {day}
            </div>
            {}
            {weeks.map((week) => {
              const dayData = week[d];
              return (
                <div
                  key={dayData.date}
                  className="aspect-square w-full rounded-[2px] cursor-pointer"
                  style={{ background: intensityColor(dayData.count, maxCount) }}
                  onMouseEnter={(e) => handleMouseEnter(e, dayData.date, dayData.count)}
                  onMouseLeave={handleMouseLeave}
                />
              );
            })}
          </React.Fragment>
        ))}
      </div>

      {}
      <div className="flex items-center gap-2 mt-1">
        <span className="text-xs text-muted-foreground">Less</span>
        <div className="flex gap-1">
          {[
            "var(--color-border)",
            "color-mix(in srgb, var(--color-primary) 15%, var(--color-border))",
            "color-mix(in srgb, var(--color-primary) 35%, var(--color-border))",
            "color-mix(in srgb, var(--color-primary) 55%, var(--color-border))",
            "color-mix(in srgb, var(--color-primary) 80%, var(--color-border))",
            "var(--color-primary)",
          ].map((color, i) => (
            <div
              key={i}
              className="aspect-square w-full rounded-[2px]"
              style={{ background: color, minWidth: 14 }}
            />
          ))}
        </div>
        <span className="text-xs text-muted-foreground">More</span>
      </div>

      {}
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
