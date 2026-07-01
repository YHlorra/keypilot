import { useMemo, useState } from "react";
import { UsageTimeSeries } from "@/components/UsageTimeSeries";
import { UsageHeatmapCalendar } from "@/components/UsageHeatmapCalendar";
import { useUsagePeriodsSummary } from "@/hooks/useUsage";
import { UsageKpiCards } from "@/components/UsageKpiCards";
import { TokensLeaderboard } from "@/components/TokensLeaderboard";
import type { UsageFilter } from "@/types/api";
import type { ProviderRow } from "@/components/TokensLeaderboard";
import { cn } from "@/lib/utils";

type RangeOption = "7d" | "30d";

const RANGE_OPTIONS: { value: RangeOption; label: string }[] = [
  { value: "7d", label: "7d" },
  { value: "30d", label: "30d" },
];

export interface UsagePageProps {
  filterProviderName?: string | null;
}

export default function UsagePage({ filterProviderName }: UsagePageProps) {
  const [selectedRange, setSelectedRange] = useState<RangeOption>("30d");

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
      map.set(point.date, point.total_tokens ?? 0);
    }
    return map;
  }, [allTimeSummary]);

  const monthLabel = useMemo(() => {
    const series = monthSummary?.daily_series ?? [];
    if (series.length === 0) return "";
    const days = new Set(series.map((p) => p.date)).size;
    return `${days} ${days === 1 ? "day" : "days"}`;
  }, [monthSummary]);

  const allTimeLabel = useMemo(() => {
    const series = allTimeSummary?.daily_series ?? [];
    if (series.length === 0) return "";
    if (series.length === 1) return "1 day";
    const dates = series.map((p) => p.date).sort();
    const first = new Date(dates[0]);
    const last = new Date(dates[dates.length - 1]);
    const spanDays = Math.max(1, Math.round((last.getTime() - first.getTime()) / 86400000) + 1);
    return `${spanDays} days`;
  }, [allTimeSummary]);

  // 兼容 selectedRange:仅用于 trend chart 显示 7d/30d 切片(从 month.daily_series 切片)
  const slicedTrendSeries = useMemo(() => {
    const series = trendDailySeries;
    if (series.length === 0) return [];
    const days = selectedRange === "7d" ? 7 : 30;
    return series.slice(-days);
  }, [trendDailySeries, selectedRange]);

  const providerLeaderboard = useMemo<ProviderRow[]>(() => {
    const pairs = monthSummary?.agent_pairs ?? [];
    if (pairs.length === 0) return [];
    const byProvider = new Map<string, { totalTokens: number; requestCount: number; topModel: string; topModelTokens: number }>();
    for (const p of pairs) {
      const cur = byProvider.get(p.provider) ?? { totalTokens: 0, requestCount: 0, topModel: "", topModelTokens: 0 };
      cur.totalTokens += p.total_tokens;
      cur.requestCount += p.request_count;
      if (p.total_tokens > cur.topModelTokens) {
        cur.topModel = p.model;
        cur.topModelTokens = p.total_tokens;
      }
      byProvider.set(p.provider, cur);
    }
    const rolled = [...byProvider.entries()].map(([provider, v]) => ({
      provider,
      totalTokens: v.totalTokens,
      requestCount: v.requestCount,
      topModel: v.topModel,
      topModelTokens: v.topModelTokens,
    }));
    const total = rolled.reduce((s, r) => s + r.totalTokens, 0);
    const withShare = rolled
      .map((r) => ({ ...r, share: total > 0 ? r.totalTokens / total : 0 }))
      .sort((a, b) => b.totalTokens - a.totalTokens)
      .slice(0, 8);
    return withShare;
  }, [monthSummary]);

  return (
    <div className="flex flex-col h-full">
      {/* Page content — single scroll context lives in App.tsx (line 215).
          Removed: overflow-y-auto (was creating the second nested scrollbar
          that combined with App.tsx's overflow-y-auto to render TWO horizontal
          bars at the bottom of the window — see docs/usage-page.html). */}
      <div className="flex-1 min-h-0 max-w-[1600px] mx-auto px-4 py-3 flex flex-col gap-4">
        <section>
          <div className="flex flex-col mb-3">
            <h2 className="text-sm font-semibold text-foreground">Activity</h2>
            <span className="text-[10px] text-muted-foreground mt-0.5">Last 26 weeks - token intensity</span>
          </div>
          {periodsLoading ? (
            <div className="h-48 animate-pulse bg-muted rounded" />
          ) : (
            <UsageHeatmapCalendar dateMap={heatmapDateMap} />
          )}
        </section>

        {/* Body: trend chart + leaderboard sidebar */}
          <div className="grid grid-cols-1 lg:grid-cols-[1fr_230px] gap-4">
          <div className="flex flex-col gap-6">
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
          </div>

          {/* Sidebar: tokens leaderboard only (4 components in spec, no agent+model breakdown) */}
          <aside className="lg:sticky lg:top-5 self-start flex flex-col gap-6">
            <TokensLeaderboard providers={providerLeaderboard} isLoading={periodsLoading} />
          </aside>
        </div>

        <UsageKpiCards
          today={todaySummary}
          month={monthSummary}
          allTime={allTimeSummary}
          monthLabel={monthLabel}
          allTimeLabel={allTimeLabel}
        />
      </div>
    </div>
  );
}