import { useMemo, useState } from "react";
import { UsageTimeSeries } from "@/components/UsageTimeSeries";
import { UsageHeatmapCalendar } from "@/components/UsageHeatmapCalendar";
import { useUsagePeriodsSummary } from "@/hooks/useUsage";
import { UsageKpiCards } from "@/components/UsageKpiCards";
import { TokensLeaderboard } from "@/components/TokensLeaderboard";
import { CodingPlanQuotas } from "@/components/CodingPlanQuotas";
import { useProviders } from "@/hooks/useProviders";
import { SectionLabel } from "@/components/SectionLabel";
import type { UsageFilter } from "@/types/api";
import type { ProviderRow } from "@/components/TokensLeaderboard";
import { cn } from "@/lib/utils";

type RangeOption = "7d" | "30d";

const RANGE_OPTIONS: { value: RangeOption; label: string }[] = [
  { value: "7d", label: "7d" },
  { value: "30d", label: "30d" },
];




const CODING_PLAN_PRESETS = new Set<string>([
  "minimax-cn",
  "minimax-en",
  "minimax-cn-anthropic",
  "minimax-en-anthropic",
  "kimi",
  "kimi-anthropic",
  "zhipu",
  "zhipu-anthropic",
  "volcengine",
  "volcengine-anthropic",
  "zenmux",
  "mimo",
]);

export interface UsagePageProps {
  filterProviderName?: string | null;
}

export default function UsagePage({ filterProviderName }: UsagePageProps) {
  const [selectedRange, setSelectedRange] = useState<RangeOption>("30d");

  
  
  const periodsFilter = useMemo((): UsageFilter => {
    return filterProviderName ? { provider: filterProviderName } : {};
  }, [filterProviderName]);

  const { data: periodsData, isLoading: periodsLoading } = useUsagePeriodsSummary(periodsFilter);

  
  
  
  
  const { data: providers = [] } = useProviders();
  const codingPlanProviderId = useMemo<number | null>(() => {
    if (!filterProviderName) return null;
    const provider = providers.find((p) => p.name === filterProviderName);
    if (!provider) return null;
    const preset = provider.preset ?? "";
    if (!CODING_PLAN_PRESETS.has(preset)) return null;
    return provider.id;
  }, [filterProviderName, providers]);

  
  const todaySummary = periodsData?.periods.today;
  const monthSummary = periodsData?.periods.month;
  const allTimeSummary = periodsData?.periods.all_time;

  
  
  
  const trendDailySeries = allTimeSummary?.daily_series ?? [];

  
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
      .slice(0, 4);
    return withShare;
  }, [monthSummary]);

  return (
    <div className="flex flex-col h-full">
      {}
      <div className="flex-1 min-h-0 max-w-[1600px] mx-auto px-4 py-3 flex flex-col gap-4">
        <section>
          <div className="flex flex-col mb-3">
            <h2 className="text-xl font-semibold font-serif text-foreground">Activity</h2>
            <span className="text-xs text-muted-foreground mt-0.5">Last 26 weeks - token intensity</span>
          </div>
          {periodsLoading ? (
            <div className="h-48 animate-pulse bg-muted rounded" />
          ) : (
            <UsageHeatmapCalendar dateMap={heatmapDateMap} />
          )}
        </section>

        {}
          <div className="grid grid-cols-1 lg:grid-cols-[1fr_230px] gap-4">
          <div className="flex flex-col gap-6">
            <section>
              <div className="flex items-center justify-between mb-3">
                <div className="flex flex-col">
                  <h2 className="text-xl font-semibold font-serif text-foreground">Trend</h2>
                  <span className="text-xs text-muted-foreground mt-0.5">
                    {selectedRange === "7d" ? "Last 7 days" : "Last 30 days"} (rolling window)
                  </span>
                </div>
                {}
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

          {}
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

        {}
        {codingPlanProviderId != null && (
          <section className="flex flex-col gap-2">
            <SectionLabel>Coding Plan</SectionLabel>
            <div className="flex flex-col gap-3 rounded-sm border border-border bg-card px-4 py-3">
              <CodingPlanQuotas providerId={codingPlanProviderId} />
            </div>
          </section>
        )}
      </div>
    </div>
  );
}