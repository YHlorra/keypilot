import type { AgentPair } from "@/types/api";
import { formatTokens } from "@/lib/format";

export interface AgentPairChartProps {
  pairs: AgentPair[];
  maxRows?: number;
}

function formatCost(costUsd: number): string {
  return `$${costUsd.toFixed(2)}`;
}

export function AgentPairChart({
  pairs,
  maxRows = 10,
}: AgentPairChartProps) {
  // Sort by total_tokens DESC
  const sorted = [...pairs].sort((a, b) => b.total_tokens - a.total_tokens);
  const shown = sorted.slice(0, maxRows);
  const remaining = sorted.slice(maxRows);
  const maxTotalTokens = shown.length > 0 ? shown[0].total_tokens : 1;

  return (
    <div className="flex flex-col gap-1">
      {shown.map((pair, idx) => {
        const barPct = (pair.total_tokens / maxTotalTokens) * 100;
        return (
          <div key={idx} className="flex items-center gap-3 min-w-0">
            {/* Agent type */}
            <span className="text-[14px] font-medium leading-none shrink-0 w-28 truncate">
              {pair.agent_type}
            </span>
            {/* Model */}
            <span className="text-[13px] text-muted-foreground font-mono leading-none shrink-0 w-32 truncate">
              {pair.model}
            </span>
            {/* Bar */}
            <div className="flex-1 min-w-0 h-2 bg-muted rounded-full overflow-hidden">
              <div
                className="h-full rounded-full"
                style={{
                  width: `${barPct}%`,
                  backgroundColor: "var(--color-primary)",
                }}
              />
            </div>
            {/* Tokens */}
            <span className="text-[13px] font-medium leading-none shrink-0 w-16 text-right">
              {formatTokens(pair.total_tokens)}
            </span>
            {/* Cost */}
            <span className="text-[13px] text-muted-foreground font-mono leading-none shrink-0 w-16 text-right">
              {formatCost(pair.total_cost_usd)}
            </span>
          </div>
        );
      })}
      {remaining.length > 0 && (
        <div className="flex items-center gap-3 min-w-0">
          <span className="text-[14px] text-muted-foreground leading-none shrink-0 w-28">
            ...
          </span>
          <span className="text-[13px] text-muted-foreground font-mono leading-none shrink-0 w-32">
            +{remaining.length} more
          </span>
          <div className="flex-1" />
          <span className="shrink-0 w-16" />
          <span className="shrink-0 w-16" />
        </div>
      )}
    </div>
  );
}