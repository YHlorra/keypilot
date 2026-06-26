import { useMemo, useState } from "react";
import { AgentPairChart } from "@/components/AgentPairChart";
import { UsageTimeSeries } from "@/components/UsageTimeSeries";
import { UsageHeatmap, type HeatmapCell } from "@/components/UsageHeatmap";
import { UsageDetailPanel } from "@/components/UsageDetailPanel";
import { ImportModal } from "@/components/ImportModal";
import { Button } from "@/components/ui/button";
import { useUsageSummary } from "@/hooks/useUsage";
import { useUsageRecords } from "@/hooks/useUsage";
import type { AgentPair, UsageFilter } from "@/types/api";

type RangeOption = "7d" | "30d" | "90d" | "all";
type TabOption = "overview" | "daily" | "hourly";

const RANGE_OPTIONS: { value: RangeOption; label: string }[] = [
  { value: "7d", label: "7D" },
  { value: "30d", label: "30D" },
  { value: "90d", label: "90D" },
  { value: "all", label: "All" },
];

const TAB_OPTIONS: { value: TabOption; label: string }[] = [
  { value: "overview", label: "Overview" },
  { value: "daily", label: "Daily" },
  { value: "hourly", label: "Hourly" },
];

// Build heatmap data from filtered usage records
function buildHeatmapData(
  records: { occurred_at: string; agent_type?: string; usage_details: { input?: number; output?: number; cache_read?: number; cache_creation?: number; reasoning?: number } }[],
  date: string
): HeatmapCell[] {
  const cells: HeatmapCell[] = [];
  const dateRecords = records.filter((r) => r.occurred_at?.startsWith(date));
  const agentTypes = [...new Set(dateRecords.map((r) => r.agent_type || "unknown"))];

  for (const agentType of agentTypes) {
    for (let hour = 0; hour < 24; hour++) {
      const hourRecords = dateRecords.filter((r) => {
        if (r.agent_type !== agentType) return false;
        const d = new Date(r.occurred_at);
        return d.getHours() === hour;
      });

      const tokens = hourRecords.reduce(
        (sum, r) => sum + (r.usage_details.input ?? 0) + (r.usage_details.output ?? 0), 0
      );
      const costUsd = 0; // TODO: derive from pricing if available
      const requestCount = hourRecords.length;

      if (tokens > 0) {
        cells.push({ hour, agentType, tokens, costUsd, requestCount });
      }
    }
  }
  return cells;
}

export default function UsagePage() {
  const [selectedRange, setSelectedRange] = useState<RangeOption>("30d");
  const [selectedTab, setSelectedTab] = useState<TabOption>("overview");
  const [selectedPair, setSelectedPair] = useState<AgentPair | null>(null);
  const [importModalOpen, setImportModalOpen] = useState(false);

  // Build filter from selected range
  const filter = useMemo((): UsageFilter => {
    const now = new Date();
    let startDate: string | undefined;

    if (selectedRange === "7d") {
      const d = new Date(now);
      d.setDate(d.getDate() - 7);
      startDate = d.toISOString().split("T")[0];
    } else if (selectedRange === "30d") {
      const d = new Date(now);
      d.setDate(d.getDate() - 30);
      startDate = d.toISOString().split("T")[0];
    } else if (selectedRange === "90d") {
      const d = new Date(now);
      d.setDate(d.getDate() - 90);
      startDate = d.toISOString().split("T")[0];
    }
    // 'all' -- no start_date filter

    return {
      start_date: startDate,
      end_date: now.toISOString().split("T")[0],
    };
  }, [selectedRange]);

  const { data: summary, isLoading: summaryLoading } = useUsageSummary(filter);

  // Fetch records for heatmap (paginated, first 1000)
  const { data: recordsData } = useUsageRecords(filter, 1, 1000);
  const records = recordsData?.items ?? [];

  // Heatmap date = today
  const heatmapDate = new Date().toISOString().split("T")[0];
  const heatmapCells = useMemo(() => buildHeatmapData(records, heatmapDate), [records, heatmapDate]);

  // Build range filter for time series sub-components
  const seriesRange = selectedRange === "all" ? "90d" : selectedRange;

  return (
    <div className="flex flex-col h-full">
      {/* Header row: range chips + Import button */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        {/* Range chips */}
        <div className="inline-flex items-center rounded-pill border border-border p-0.5 gap-0.5">
          {RANGE_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              type="button"
              onClick={() => setSelectedRange(opt.value)}
              className={`inline-flex items-center px-3 py-1.5 rounded-pill text-xs font-medium transition-colors ${
                selectedRange === opt.value
                  ? "bg-[var(--color-primary)] text-[var(--color-secondary)]"
                  : "text-[var(--color-muted)] hover:text-[var(--color-neutral)]"
              }`}
            >
              {opt.label}
            </button>
          ))}
        </div>

        {/* Import button */}
        <Button size="sm" onClick={() => setImportModalOpen(true)}>
          Import
        </Button>
      </div>

      {/* Tab bar */}
      <div className="flex items-center gap-2 px-4 py-2 border-b border-border">
        {TAB_OPTIONS.map((opt) => (
          <button
            key={opt.value}
            type="button"
            onClick={() => setSelectedTab(opt.value)}
            className={`px-4 py-1.5 rounded text-sm font-medium transition-colors ${
              selectedTab === opt.value
                ? "bg-secondary text-secondary-foreground"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            {opt.label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div className="flex-1 overflow-y-auto px-4 py-4">
        {selectedTab === "overview" && (
          <div className="flex flex-col gap-4 h-full">
            {/* AgentPairChart -- top, 40% height */}
            <div className="flex-shrink-0" style={{ height: "40%" }}>
              {summaryLoading ? (
                <div className="h-full animate-pulse bg-muted rounded" />
              ) : (
                <AgentPairChart
                  pairs={summary?.agent_pairs ?? []}
                  maxRows={10}
                />
              )}
            </div>
            {/* UsageTimeSeries -- bottom, 60% height */}
            <div className="flex-1 min-h-0">
              {summaryLoading ? (
                <div className="h-full animate-pulse bg-muted rounded" />
              ) : (
                <UsageTimeSeries
                  dailySeries={summary?.daily_series ?? []}
                  range={seriesRange}
                  isLoading={summaryLoading}
                />
              )}
            </div>
          </div>
        )}

        {selectedTab === "daily" && (
          <div className="h-full">
            {summaryLoading ? (
              <div className="h-full animate-pulse bg-muted rounded" />
            ) : (
              <UsageTimeSeries
                dailySeries={summary?.daily_series ?? []}
                range={seriesRange}
                isLoading={summaryLoading}
              />
            )}
          </div>
        )}

        {selectedTab === "hourly" && (
          <div className="h-full">
            <UsageHeatmap data={heatmapCells} date={heatmapDate} loading={false} />
          </div>
        )}
      </div>

      {/* UsageDetailPanel slide-in */}
      {selectedPair !== null && (
        <UsageDetailPanel
          agentPair={selectedPair}
          onClose={() => setSelectedPair(null)}
        />
      )}

      {/* ImportModal */}
      <ImportModal
        open={importModalOpen}
        onClose={() => setImportModalOpen(false)}
      />
    </div>
  );
}
