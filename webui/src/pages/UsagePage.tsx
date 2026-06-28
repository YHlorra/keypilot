import { useMemo, useState } from "react";
import { UsageTimeSeries } from "@/components/UsageTimeSeries";
import { UsageHeatmapCalendar } from "@/components/UsageHeatmapCalendar";
import { UsageDetailPanel } from "@/components/UsageDetailPanel";
import { ImportModal } from "@/components/ImportModal";
import { Button } from "@/components/ui/button";
import { useUsageSummary } from "@/hooks/useUsage";
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
  const [importModalOpen, setImportModalOpen] = useState(false);
  const [selectedPair, setSelectedPair] = useState<AgentPair | null>(null);

  // Trend filter (windowed by selectedRange)
  const trendFilter = useMemo((): UsageFilter => {
    const now = new Date();
    const start = new Date(now);
    start.setDate(start.getDate() - (selectedRange === "7d" ? 7 : 30));
    return {
      start_date: start.toISOString().split("T")[0],
      end_date: now.toISOString().split("T")[0],
      ...(filterProviderName ? { provider: filterProviderName } : {}),
    };
  }, [selectedRange, filterProviderName]);

  // Lifetime filter (no date range -- fetches all-time data)
  const lifetimeFilter = useMemo((): UsageFilter => {
    return filterProviderName ? { provider: filterProviderName } : {};
  }, [filterProviderName]);

  const { data: trendSummary, isLoading: trendLoading } = useUsageSummary(trendFilter);
  const { data: lifetimeSummary, isLoading: lifetimeLoading } = useUsageSummary(lifetimeFilter);

  // Compute derived stats from LIFETIME daily_series (all-time, Fix 3)
  const { todayTotal, last7dTotal, last30dTotal, dateMap, peakDay, peakDayLabel, activeDays } =
    useMemo(() => {
      const series = lifetimeSummary?.daily_series ?? [];
      const now = new Date();
      const todayISO = now.toISOString().split("T")[0];

      const last7d = new Date(now);
      last7d.setDate(last7d.getDate() - 7);
      const last7dISO = last7d.toISOString().split("T")[0];

      const last30d = new Date(now);
      last30d.setDate(last30d.getDate() - 30);
      const last30dISO = last30d.toISOString().split("T")[0];

      let todayTotal = 0;
      let last7dTotal = 0;
      let last30dTotal = 0;
      let peakDay = 0;
      let peakDayLabel = "";
      let activeDays = 0;
      const dateMap = new Map<string, number>();

      for (const point of series) {
        const count = point.request_count ?? 0;
        dateMap.set(point.date, count);

        if (point.date === todayISO) todayTotal += count;
        if (point.date >= last7dISO) last7dTotal += count;
        if (point.date >= last30dISO) last30dTotal += count;

        if (count > peakDay) {
          peakDay = count;
          peakDayLabel = point.date;
        }
        if (count > 0) activeDays++;
      }

      return { todayTotal, last7dTotal, last30dTotal, dateMap, peakDay, peakDayLabel, activeDays };
    }, [lifetimeSummary]);

  // Lifetime total (backend-computed from all-time query)
  const lifetimeTotal = lifetimeSummary?.total_requests ?? 0;

  // Period total (from windowed trend query, Fix 3 bonus)
  const periodTotal = trendSummary?.total_requests ?? 0;

  // Top agent pairs sorted by tokens (from trend window)
  const topAgentPairs = useMemo(() => {
    return [...(trendSummary?.agent_pairs ?? [])]
      .sort((a, b) => b.total_tokens - a.total_tokens)
      .slice(0, 5);
  }, [trendSummary]);

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-border">
        <div>
          <h1 className="text-lg font-semibold text-foreground" style={{ letterSpacing: "var(--tracking-tight)" }}>
            Usage
          </h1>
          <p className="text-sm text-muted-foreground mt-0.5">
            Data from language model calls, may be delayed
          </p>
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
          <Button size="sm" onClick={() => setImportModalOpen(true)}>
            Import
          </Button>
        </div>
      </div>

      {/* Page content */}
      <div className="flex-1 overflow-y-auto px-6 py-5 flex flex-col gap-6">
        {/* KPI cards */}
        <UsageKpiCards todayTotal={todayTotal} last7dTotal={last7dTotal} last30dTotal={last30dTotal} />

        {/* Body: trend chart + sidebar */}
        <div className="grid grid-cols-1 lg:grid-cols-[1fr_280px] gap-6">
          {/* Main column */}
          <div className="flex flex-col gap-6">
            {/* Trend chart */}
            <section>
              <div className="flex items-center justify-between mb-3">
                <h2 className="text-sm font-semibold text-foreground">Trend</h2>
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

              {trendLoading ? (
                <div className="h-48 animate-pulse bg-muted rounded" />
              ) : (trendSummary?.daily_series ?? []).length === 0 ? (
                <div className="h-48 flex items-center justify-center text-sm text-muted-foreground">
                  No data in selected range
                </div>
              ) : (
                <UsageTimeSeries
                  dailySeries={trendSummary?.daily_series ?? []}
                  range={selectedRange}
                  isLoading={false}
                />
              )}
            </section>

            {/* Activity heatmap */}
            <section>
              <h2 className="text-sm font-semibold text-foreground mb-3">Activity</h2>
              {lifetimeLoading ? (
                <div className="h-48 animate-pulse bg-muted rounded" />
              ) : (
                <UsageHeatmapCalendar dateMap={dateMap} />
              )}
            </section>
          </div>

          {/* Sidebar */}
          <aside>
            <UsageStatsSidebar
              lifetimeTotal={lifetimeTotal}
              periodTotal={periodTotal}
              selectedRange={selectedRange}
              peakDay={peakDay}
              peakDayLabel={peakDayLabel}
              activeDays={activeDays}
              topAgentPairs={topAgentPairs}
            />
          </aside>
        </div>
      </div>

      {/* UsageDetailPanel slide-in */}
      {selectedPair !== null && (
        <UsageDetailPanel agentPair={selectedPair} onClose={() => setSelectedPair(null)} />
      )}

      {/* ImportModal */}
      <ImportModal open={importModalOpen} onClose={() => setImportModalOpen(false)} />
    </div>
  );
}
