import * as React from "react";
import { formatTokens } from "@/lib/format";

export interface ProviderRow {
  provider: string;
  totalTokens: number;
  requestCount: number;
  topModel: string;
  topModelTokens: number;
  share: number; // 0-1, this provider's tokens / total tokens in leaderboard
}

interface TokensLeaderboardProps {
  providers: ProviderRow[];
  isLoading?: boolean;
}

export const TokensLeaderboard = React.memo(function TokensLeaderboard({
  providers,
  isLoading,
}: TokensLeaderboardProps) {
  return (
    <div className="flex flex-col gap-3 rounded-sm border border-border bg-card px-4 py-3">
      <span className="text-xs text-muted-foreground font-medium uppercase tracking-wider">
        Tokens by provider
      </span>

      {isLoading ? (
        <div className="flex flex-col gap-2">
          {[...Array(8)].map((_, i) => (
            <div key={i} className="h-8 rounded animate-pulse bg-muted" />
          ))}
        </div>
      ) : providers.length === 0 ? (
        <span className="text-xs text-muted-foreground">No provider data yet</span>
      ) : (
        <div className="flex flex-col gap-3">
          {providers.map((row, idx) => (
            <div key={row.provider} className="grid grid-cols-[12px_1fr_auto] gap-x-[6px] items-center py-[3px] border-b border-border-soft last:border-0">
              {/* Rank */}
              <span className="text-[9px] text-muted-foreground font-medium text-right">
                {idx + 1}
              </span>

              {/* Provider + top model */}
              <div className="flex flex-col min-w-0">
                <span className="text-[11px] font-medium truncate" title={row.provider}>
                  {row.provider}
                </span>
                {row.topModel && (
                  <span className="text-[9px] text-muted-foreground truncate" title={row.topModel}>
                    {row.topModel}
                  </span>
                )}
              </div>

              {/* Tokens + bar */}
              <div className="flex items-center gap-[6px] shrink-0">
                <div className="w-[36px] h-[3px] bg-border rounded overflow-hidden">
                  <div
                    className="h-full rounded"
                    style={{
                      backgroundColor: "var(--color-primary)",
                      width: `${row.share * 100}%`,
                    }}
                  />
                </div>
                <span className="text-[11px] font-medium font-mono tabular-nums">
                  {formatTokens(row.totalTokens)}
                </span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
});