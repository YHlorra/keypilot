import * as React from "react";
import { formatTokens, formatNumber } from "@/lib/format";

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
            <div key={row.provider} className="flex flex-col gap-1">
              <div className="flex items-start gap-2 min-w-0">
                {/* Rank */}
                <span className="text-xs text-muted-foreground font-mono shrink-0 w-4 text-right">
                  {idx + 1}
                </span>

                {/* Provider + top model */}
                <div className="flex flex-col min-w-0 flex-1">
                  <span className="text-xs font-semibold truncate" title={row.provider}>
                    {row.provider}
                  </span>
                  {row.topModel && (
                    <span className="text-[11px] text-muted-foreground truncate" title={row.topModel}>
                      {row.topModel}
                    </span>
                  )}
                </div>

                {/* Tokens + request count */}
                <div className="flex flex-col items-end shrink-0">
                  <span className="text-xs font-mono">{formatTokens(row.totalTokens)}</span>
                  <span className="text-[11px] text-muted-foreground">
                    {formatNumber(row.requestCount)} req
                  </span>
                </div>
              </div>

              {/* Share bar */}
              <div className="ml-6 h-[2px] rounded-full bg-border overflow-hidden">
                <div
                  className="h-full rounded-full"
                  style={{
                    backgroundColor: "var(--color-primary)",
                    width: `${row.share * 100}%`,
                  }}
                />
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
});