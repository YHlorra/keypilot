import { useMemo, useState } from "react";
import { UsageTimeSeries } from "@/components/UsageTimeSeries";
import { UsageHeatmapCalendar } from "@/components/UsageHeatmapCalendar";
import { UsageDetailPanel } from "@/components/UsageDetailPanel";
import { useUsagePeriodsSummary } from "@/hooks/useUsage";
import { UsageKpiCards } from "@/components/UsageKpiCards";
import { UsageStatsSidebar } from "@/components/UsageStatsSidebar";
import type { AgentPair, UsageFilter } from "@/types/api";
import { cn } from "@/lib/utils";

type RangeOption = "7d" | "30d";

const RANGE_OPTIONS: { value: RangeOption; label: string }[] = [
  { value: "7d", label: "7d" },
  { value: "30d", label: "30d" },
];

export interface UsagePageProps {
  filterProviderName?: string | null;
  onClearFilter?: () => void;
}

export default function UsagePage({ filterProviderName, onClearFilter }: UsagePageProps) {
  const [selectedRange, setSelectedRange] = useState<RangeOption>("30d");
  const [selectedPair, setSelectedPair] = useState<AgentPair | null>(null);

  // 单 IPC:一次拿 today/month/allTime + client_models + limits
  // 注:filter 只用于 provider 维度过滤,日期由后端按 period_windows 算
  const periodsFilter = useMemo((): UsageFilter => {
    return filterProviderName ? { provider: filterProviderName } : {};
  }, [filterProviderName]);

  const { data: periodsData, isLoading: periodsLoading } = useUsagePeriodsSummary(periodsFilter);

  // 三周期直接读
  const todaySummary = periodsData?.periods.today;
  const monthSummary = periodsData?.periods.month;
  const allTimeSummary = periodsData?.periods.all_time;

  // Trend chart 用 today+month 拼出窗口数据
  // (或直接用 month 的 daily_series 显示当月趋势)
  const trendDailySeries = monthSummary?.daily_series ?? [];

  // Lifetime daily_series 用于 heatmap(all-time)
  const heatmapDateMap = useMemo(() => {
    const series = allTimeSummary?.daily_series ?? [];
    const map = new Map<string, number>();
    for (const point of series) {
      map.set(point.date, point.request_count ?? 0);
    }
    return map;
  }, [allTimeSummary]);

  // Peak day / active days (from all-time)
  const { peakDay, peakDayLabel, activeDays } = useMemo(() => {
    const series = allTimeSummary?.daily_series ?? [];
    let peak = 0;
    let peakLabel = "";
    let active = 0;
    for (const point of series) {
      const count = point.request_count ?? 0;
      if (count > peak) {
        peak = count;
        peakLabel = point.date;
      }
      if (count > 0) active++;
    }
    return { peakDay: peak, peakDayLabel: peakLabel, activeDays: active };
  }, [allTimeSummary]);

  // Top agent pairs (from month summary)
  const topAgentPairs = useMemo(() => {
    return [...(monthSummary?.agent_pairs ?? [])]
      .sort((a, b) => b.total_tokens - a.total_tokens)
      .slice(0, 5);
  }, [monthSummary]);

  const monthTotal = monthSummary?.total_requests ?? 0;
  const allTimeTotal = allTimeSummary?.total_requests ?? 0;

  // 兼容 selectedRange:仅用于 trend chart 显示 7d/30d 切片(从 month.daily_series 切片)
  const slicedTrendSeries = useMemo(() => {
    const series = trendDailySeries;
    if (series.length === 0) return [];
    const days = selectedRange === "7d" ? 7 : 30;
    return series.slice(-days);
  }, [trendDailySeries, selectedRange]);

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex-shrink-0 min-h-[64px] flex items-center justify-between px-6 py-4 border-b border-border">
        <div>
          <h1 className="text-lg font-semibold text-white" style={{ letterSpacing: "var(--tracking-tight)" }}>
            Usage
          </h1>
        </div>
        <div className="flex items-center gap-2">
          {filterProviderName != null && (
            <button
              onClick={onClearFilter}
              className="inline-flex items-center gap-1 px-3 py-1.5 rounded-pill border border-border text-xs text-muted-foreground hover:text-foreground transition-colors"
              title="Clear provider filter"
            >
              Clear filter
            </button>
          )}
        </div>
      </div>

      {/* Page content */}
      <div className="flex-1 min-h-0 overflow-y-auto px-6 py-5 flex flex-col gap-6">
        {/* KPI cards: today / month / all-time 三档 */}
        <UsageKpiCards
          today={todaySummary}
          month={monthSummary}
          allTime={allTimeSummary}
        />

        {/* Body: trend chart + sidebar */}
        <div className="grid grid-cols-1 lg:grid-cols-[1fr_280px] gap-6">
          {/* Main column */}
          <div className="flex flex-col gap-6">
            {/* Trend chart */}
            <section>
              <div className="flex items-center justify-between mb-3">
                <div className="flex flex-col">
                  <h2 className="text-sm font-semibold text-foreground">Trend</h2>
                  <span className="text-[10px] text-muted-foreground mt-0.5">
                    {selectedRange === "7d" ? "Last 7 days" : "Last 30 days"} (from month daily series)
                  </span>
                </div>
                {/* Range toggle */}
                <div className="inline-flex items-center rounded-pill border border-border p-0.5 gap-0.5">
                  {RANGE_OPTIONS.map((opt) => (
                    <button
                      key={opt.value}
                      type="button"
                      onClick={() => setSelectedRange(opt.value)}
                      className={cn(
                        "inline-flex items-center px-3 py-1 rounded-pill text-xs font-medium transition-colors",
                        selectedRange === opt.value
                          ? "bg-[var(--color-primary)] text-[var(--color-secondary)]"
                          : "text-[var(--color-muted)] hover:text-[var(--color-neutral)]"
                      )}
                    >
                      {opt.label}
                    </button>
                  ))}
                </div>
              </div>

              {periodsLoading ? (
                <div className="h-48 animate-pulse bg-muted rounded" />
              ) : slicedTrendSeries.length === 0 ? (
                <div className="h-48 flex items-center justify-center text-sm text-muted-foreground">
                  No data in selected range
                </div>
              ) : (
                <UsageTimeSeries
                  dailySeries={slicedTrendSeries}
                  range={selectedRange}
                  isLoading={false}
                />
              )}
            </section>

            {/* Activity heatmap */}
            <section>
              <div className="flex flex-col mb-3">
                <h2 className="text-sm font-semibold text-foreground">Activity</h2>
                <span className="text-[10px] text-muted-foreground mt-0.5">Last 26 weeks (all-time, not affected by range)</span>
              </div>
              {periodsLoading ? (
                <div className="h-48 animate-pulse bg-muted rounded" />
              ) : (
                <UsageHeatmapCalendar dateMap={heatmapDateMap} />
              )}
            </section>
          </div>

          {/* Sidebar */}
          <aside>
            <UsageStatsSidebar
              lifetimeTotal={allTimeTotal}
              periodTotal={monthTotal}
              selectedRange={selectedRange}
              peakDay={peakDay}
              peakDayLabel={peakDayLabel}
              activeDays={activeDays}
              topAgentPairs={topAgentPairs}
              clientModels={periodsData?.client_models}
            />
          </aside>
        </div>
      </div>

      {/* UsageDetailPanel slide-in */}
      {selectedPair !== null && (
        <UsageDetailPanel agentPair={selectedPair} onClose={() => setSelectedPair(null)} />
      )}
    </div>
  );
}
