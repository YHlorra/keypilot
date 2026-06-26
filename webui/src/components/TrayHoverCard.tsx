import * as React from "react";
import { useCallback } from "react";
import { Button } from "./ui/button";
import { Icon, useToast, ProviderIcon } from "./Icon";
import { cn } from "@/lib/utils";
import { useProviders } from "@/hooks/useProviders";
import { useQuota } from "@/hooks/useQuota";
import { quitApp } from "@/lib/api";
import type { Provider, QuotaSnapshot } from "@/types/api";

interface TrayHoverCardProps {
  onOpenMain: () => void;
  onSelectProvider: (providerId: number) => void;
  onClose: () => void;
}

interface ProviderQuotaRowProps {
  provider: Provider;
  onSelect: (providerId: number) => void;
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

function CompactQuotaBadge({ providerId, preset }: { providerId: number; preset: string | null }) {
  const { data, isLoading, isError, dataUpdatedAt } = useQuota(providerId);

  // Anthropic 不支持 quota
  if (preset === "anthropic") {
    return (
      <span className="text-xs text-muted-foreground">不支持</span>
    );
  }

  if (isLoading) {
    return (
      <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
        <Icon name="loader" className="w-3 h-3" />
        <span>加载中...</span>
      </span>
    );
  }

  if (isError || !data) {
    return <span className="text-xs text-danger">刷新失败</span>;
  }

  const quota = data as QuotaSnapshot;
  const levelColor = quota.level ? LEVEL_COLORS[quota.level] : "text-foreground";

  // Show remaining if available, otherwise show used
  const displayText = quota.remaining !== undefined && quota.remaining !== null
    ? `剩余: ${formatNumber(quota.remaining)} ${quota.unit}`
    : `已用: ${formatNumber(quota.used)} ${quota.unit}`;

  return (
    <div className="flex items-center gap-1.5">
      <span className={cn("text-xs font-mono", levelColor)}>{displayText}</span>
      <span className="text-xs text-muted-foreground">
        {formatRelativeTime(dataUpdatedAt ? Math.floor(dataUpdatedAt / 1000) : undefined)}
      </span>
    </div>
  );
}

function ProviderQuotaRow({ provider, onSelect }: ProviderQuotaRowProps) {
  return (
    <button
      type="button"
      onClick={() => onSelect(provider.id)}
      className="w-full flex items-center gap-3 px-3 py-2 rounded-md hover:bg-accent transition-colors text-left"
    >
      <ProviderIcon preset={provider.preset} name={provider.name} className="w-5 h-5 flex-shrink-0" />
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium truncate">{provider.name}</div>
        <CompactQuotaBadge providerId={provider.id} preset={provider.preset} />
      </div>
    </button>
  );
}

export const TrayHoverCard = React.memo(function TrayHoverCard({
  onOpenMain,
  onSelectProvider,
  onClose,
}: TrayHoverCardProps) {
  const { showToast } = useToast();
  const { data: allProviders = [], isLoading, isError } = useProviders();

  // Derive pinned providers from Provider.pinned field (Fix #12)
  const pinnedProviders = React.useMemo(() => {
    return allProviders.filter((p) => p.pinned);
  }, [allProviders]);

  const handleQuit = useCallback(async () => {
    try {
      await quitApp();
    } catch (err) {
      showToast("退出失败: " + String(err), "error");
    }
  }, [showToast]);

  const handleOpenMain = useCallback(() => {
    onOpenMain();
  }, [onOpenMain]);

  return (
    <div className="w-[280px] rounded-lg border border-border bg-popover overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-border bg-card">
        <div className="flex items-center gap-2">
          <Icon name="keyRound" className="w-4 h-4" />
          <span className="text-sm font-semibold">KeyPilot</span>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="p-1 rounded hover:bg-accent transition-colors"
          title="关闭"
        >
          <Icon name="x" className="w-3.5 h-3.5" />
        </button>
      </div>

      {/* Provider list */}
      <div className="max-h-[200px] overflow-y-auto">
        {isLoading ? (
          <div className="flex items-center justify-center py-8 gap-2 text-sm text-muted-foreground">
            <Icon name="loader" className="w-4 h-4" />
            <span>正在加载...</span>
          </div>
        ) : isError ? (
          <div className="flex items-center justify-center py-8 text-sm text-danger">
            <span>加载失败</span>
          </div>
        ) : pinnedProviders.length === 0 ? (
          <div className="py-6 px-3 text-sm text-muted-foreground text-center">
            <p>未钉住任何凭证。</p>
            <p className="text-xs mt-1">在主窗口右键凭证即可钉住。</p>
          </div>
        ) : (
          <div className="py-1">
            {pinnedProviders.map((provider) => (
              <ProviderQuotaRow
                key={provider.id}
                provider={provider}
                onSelect={onSelectProvider}
              />
            ))}
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="flex items-center gap-2 px-3 py-2 border-t border-border bg-card">
        <Button size="sm" variant="ghost" onClick={handleOpenMain} className="flex-1 text-xs h-7">
          打开主窗口
        </Button>
        <Button size="sm" variant="ghost" onClick={handleQuit} className="flex-1 text-xs h-7 text-danger hover:text-danger">
          退出
        </Button>
      </div>
    </div>
  );
});

// Re-export the CompactQuotaBadge for external use
export { CompactQuotaBadge };
