import * as React from "react";
import { useQuery } from "@tanstack/react-query";
import { fetchQuota } from "@/lib/api";
import type { FetchQuotaRequest, QuotaSnapshot } from "@/types/api";
import { Icon } from "./Icon";
import { cn } from "@/lib/utils";

interface QuotaBadgeProps {
  providerId: number | null;
  preset: string | null;
  className?: string;
}

const LEVEL_COLORS: Record<string, string> = {
  green: "text-success",
  amber: "text-warning",
  red: "text-danger",
  ruby: "text-critical",
};

function formatNumber(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(2);
  if (n >= 1_000) return (n / 1_000).toFixed(2);
  return n.toFixed(2);
}

function formatRelativeTime(timestamp: number | undefined): string {
  if (!timestamp) return "从未刷新";
  const diff = Math.floor(Date.now() / 1000) - timestamp;
  if (diff < 60) return "刚刚";
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`;
  return `${Math.floor(diff / 86400)} 天前`;
}

export const QuotaBadge = React.memo(function QuotaBadge({ providerId, preset, className }: QuotaBadgeProps) {
  const { data, isLoading, isError, error, refetch, dataUpdatedAt } = useQuery({
    queryKey: ["quota", providerId],
    queryFn: () => fetchQuota({ id: providerId! } as FetchQuotaRequest),
    enabled: providerId !== null && preset !== "anthropic", // Anthropic 不支持 quota
    staleTime: 5 * 60 * 1000, // 5 分钟
  });

  // Anthropic 不显示 quota
  if (preset === "anthropic") {
    return null;
  }

  if (!providerId) return null;

  if (isLoading) {
    return (
      <div className={cn("inline-flex items-center gap-1.5 text-sm text-muted-foreground", className)}>
        <Icon name="loader" className="w-3.5 h-3.5" />
        <span>加载中...</span>
      </div>
    );
  }

  if (isError) {
    return (
      <div
        className={cn("inline-flex items-center gap-1.5 text-sm text-danger", className)}
        title={error instanceof Error ? error.message : "获取配额失败"}
      >
        <span>—</span>
        <span className="text-xs">刷新失败</span>
      </div>
    );
  }

  if (!data) return null;

  const quota = data as QuotaSnapshot;
  const levelColor = quota.level ? LEVEL_COLORS[quota.level] : "text-foreground";
  const displayText = quota.total !== null
    ? `已用: ${formatNumber(quota.used)} / ${formatNumber(quota.total)} ${quota.unit}`
    : `已用: ${formatNumber(quota.used)} ${quota.unit}`;

  return (
    <div className={cn("inline-flex items-center gap-2", className)}>
      <span className={cn("text-sm font-mono", levelColor)}>{displayText}</span>
      <button
        type="button"
        onClick={() => refetch()}
        className="p-1 rounded hover:bg-accent transition-colors"
        title={`上次刷新: ${formatRelativeTime(dataUpdatedAt ? Math.floor(dataUpdatedAt / 1000) : undefined)}`}
      >
        <Icon name="refresh" className="w-3.5 h-3.5" />
      </button>
      <span className="text-xs text-muted-foreground">
        {formatRelativeTime(dataUpdatedAt ? Math.floor(dataUpdatedAt / 1000) : undefined)}
      </span>
    </div>
  );
});
