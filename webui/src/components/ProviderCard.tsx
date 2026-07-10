import {
  RefreshCw,
  Pencil,
  Copy,
  BarChart3,
  Terminal,
  Trash2,
  ChevronDown,
  Pin,
} from "lucide-react";
import { cn, isLlmCategory } from "@/lib/utils";
import { formatRelativeShort, formatTimeOfDay } from "@/lib/format";
import type { Provider, Category, ExtraProtocol } from "@/types/api";
import { useCodingPlanQuota } from "@/hooks/useCodingPlanQuota";
import { useProviderQuota } from "@/hooks/useProviderQuota";
import { ContextMenu } from "./ContextMenu";
import { ProviderIcon } from "./Icon";
type Tone = "ok" | "warn" | "crit" | "none";

// Phase 5c: extras foldout. Types from @/types/api (ExtraEndpoint, ExtraProtocol).
// ponytail: protocol → human label kept local; if it grows, hoist to format.ts.
const EXTRA_PROTOCOL_LABEL: Record<ExtraProtocol, string> = {
  openai: "OpenAI 兼容",
  anthropic: "Anthropic 兼容",
  github: "GitHub",
  deepseek: "DeepSeek",
  balance: "仅余额",
};




const TONE_TEXT: Record<Tone, string> = {
  ok: "text-[var(--color-success)]",
  warn: "text-[var(--color-accent)]",
  crit: "text-[var(--color-destructive)]",
  none: "text-[var(--color-muted)]",
};

const TONE_STROKE: Record<Tone, string> = {
  ok: "var(--color-success)",
  warn: "var(--color-accent)",
  crit: "var(--color-destructive)",
  none: "var(--color-muted)",
};


function quotaTone(remaining: number | null): Tone {
  if (remaining === null) return "none";
  if (remaining > 50) return "ok";
  if (remaining >= 20) return "warn";
  return "crit";
}
const quotaTextTone = quotaTone;


const DONUT_CIRCUMFERENCE = 37.7;

function TooltipRow({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex items-baseline justify-between gap-3">
      <span className="text-[var(--color-muted)] shrink-0">{label}</span>
      <span className="tabular-nums text-right truncate text-[var(--color-neutral)]">{value}</span>
    </div>
  );
}

interface ProviderCardProps {
  provider: Provider;
  categories: Category[];
  selected: boolean;
  onClick: () => void;
  onRefresh: (e: React.MouseEvent) => void;
  onDelete: (id: number) => void;
  onCopy?: (id: number) => void;
  onEdit?: (id: number) => void;
  onTokenUsage?: (id: number) => void;
  onTest?: (id: number) => void;
  onPin?: (id: number) => void;
}

export const ProviderCard = ({
  provider,
  categories,
  selected,
  onClick,
  onRefresh,
  onDelete,
  onCopy,
  onEdit,
  onTokenUsage,
  onTest,
  onPin,
}: ProviderCardProps) => {
const isLlm = isLlmCategory(provider.category_id, categories);

  // Phase 5c: extras from Provider type (REQ-CAT-022: extras only present for built-in presets with multi-endpoint catalog)
  const extras = provider.extras;

  const baseUrlField = provider.fields.find((f) => f.key === "base_url");
  const displayUrl = baseUrlField?.value || "https://example.com";

  
const codingPlanQuery = useCodingPlanQuota(provider.id);
  const codingPlan = codingPlanQuery.data;
  const pctFromTier = codingPlan?.success
    ? codingPlan.tiers[0]?.remaining_percent ?? null
    : null;

  
  
  const quotaQuery = useProviderQuota(provider.id);
  const snapshot = quotaQuery.data;
  const pctFromSnapshot =
    snapshot?.remaining != null && snapshot?.total != null && snapshot.total > 0
      ? (snapshot.remaining / snapshot.total) * 100
      : null;

  
  const updatedAtMs = quotaQuery.dataUpdatedAt || codingPlanQuery.dataUpdatedAt || undefined;
  const tier0 = codingPlan?.success ? codingPlan.tiers[0] : null;
  const resetMs = tier0?.resets_at_ms ?? snapshot?.reset_at ?? null;

  const pct = pctFromTier ?? pctFromSnapshot;
  const donutOffset =
    pct === null ? 0 : DONUT_CIRCUMFERENCE * (1 - Math.max(0, Math.min(100, pct)) / 100);
  const tone = quotaTone(pct);

  
  const balance = snapshot?.remaining != null ? snapshot : null;
  const balanceTone: Tone = pct !== null ? quotaTextTone(pct) : "ok";
  
  
  const balanceText = balance ? (balance.remaining ?? 0).toFixed(2) : null;

  const handleRefresh = (e: React.MouseEvent) => {
    e.stopPropagation();
    onRefresh(e);
  };

  const handleUrlClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    window.open(displayUrl, "_blank");
  };

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
  };

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    onCopy?.(provider.id);
  };

  const handleEdit = (e: React.MouseEvent) => {
    e.stopPropagation();
    onEdit?.(provider.id);
  };

  const handleTokenUsage = (e: React.MouseEvent) => {
    e.stopPropagation();
    onTokenUsage?.(provider.id);
  };

  const handleTest = (e: React.MouseEvent) => {
    e.stopPropagation();
    onTest?.(provider.id);
  };

  const handleDelete = (e: React.MouseEvent) => {
    e.stopPropagation();
    onDelete(provider.id);
  };

  const handlePin = (e: React.MouseEvent) => {
    e.stopPropagation();
    onPin?.(provider.id);
  };

  return (
    <ContextMenu providerId={provider.id} onDelete={onDelete}>
      <div
        data-testid="provider-card"
        data-provider-id={provider.id}
        data-selected={selected}
        onClick={onClick}
        onContextMenu={handleContextMenu}
        
        
        
        
        
        
        onPointerDown={(e) => e.stopPropagation()}
        className={cn(
          "relative flex items-center gap-3 px-4 py-3 rounded-[8px] border cursor-pointer transition-colors select-none",
          "bg-[var(--color-surface)] border-[var(--color-border)]",
          "hover:border-[var(--color-primary)]",
          selected
            ? "bg-[var(--color-surface-sunken)] border-l-4 border-l-[var(--color-primary)] border-[var(--color-primary)]"
            : "border border-[var(--color-border)]"
        )}
      >
        {}
        <div data-testid="provider-icon" className="shrink-0">
          <ProviderIcon
            preset={provider.preset}
            name={provider.name}
            icon={provider.icon}
            className="w-8 h-8 rounded"
          />
        </div>

        {}
        <div className="flex-1 min-w-0">
          <div
            data-testid="provider-name"
            className="flex items-center gap-2 min-w-0"
          >
            {pct !== null && (
              <div className="relative shrink-0 group/donut" data-testid="quota-tooltip-wrapper">
                <svg
                  data-testid="quota-donut"
                  viewBox="0 0 16 16"
                  width="16"
                  height="16"
                  className="block cursor-help"
                  aria-label={`${Math.round(pct)}% remaining`}
                  role="img"
                >
                  <circle
                    cx="8"
                    cy="8"
                    r="6"
                    fill="none"
                    stroke="var(--color-surface-elevated)"
                    strokeWidth="2.2"
                  />
                  <circle
                    cx="8"
                    cy="8"
                    r="6"
                    fill="none"
                    stroke={TONE_STROKE[tone]}
                    strokeWidth="2.2"
                    strokeDasharray={DONUT_CIRCUMFERENCE}
                    strokeDashoffset={donutOffset}
                    strokeLinecap="round"
                    style={{ transform: "rotate(-90deg)", transformOrigin: "50% 50%" }}
                  />
                </svg>
                <div
                  data-testid="quota-tooltip"
                  role="tooltip"
                  className={cn(
                    "absolute left-0 top-full mt-2 z-50 w-56 p-2.5 rounded-md",
                    "bg-[var(--color-popover)] border border-[var(--color-border)]",
                    "shadow-lg text-xs leading-relaxed",
                    "opacity-0 invisible group-hover/donut:opacity-100 group-hover/donut:visible",
                    "transition-opacity duration-150 pointer-events-none"
                  )}
                >
                  <div className="font-semibold text-[var(--color-neutral)] mb-1.5">
                    额度详情 · {Math.round(pct ?? 0)}%
                  </div>
                  {tier0 ? (
                    <>
                      <TooltipRow label="已用" value={`${tier0.used ?? "—"}${tier0.limit != null ? ` / ${tier0.limit}` : ""}`} />
                      {tier0.reset_description && (
                        <TooltipRow label="重置" value={tier0.reset_description} />
                      )}
                    </>
                  ) : snapshot ? (
                    <>
                      <TooltipRow label="已用" value={`${(snapshot.used ?? 0).toFixed(2)} ${snapshot.unit}`} />
                      <TooltipRow label="总额" value={snapshot.total != null ? `${snapshot.total.toFixed(2)} ${snapshot.unit}` : "—"} />
                      <TooltipRow label="剩余" value={balanceText != null ? `${balanceText} ${snapshot.unit}` : "—"} />
                    </>
                  ) : (
                    <div className="text-[var(--color-muted)]">暂无数据</div>
                  )}
                  <div className="border-t border-[var(--color-border)] mt-1.5 pt-1.5">
                    <TooltipRow label="更新于" value={formatTimeOfDay(updatedAtMs)} />
                    {resetMs && <TooltipRow label="重置于" value={formatTimeOfDay(resetMs)} />}
                  </div>
                </div>
              </div>
            )}
            {pct !== null && (
              <span
                data-testid="quota-updated"
                className="shrink-0 text-[10px] text-[var(--color-muted)] tabular-nums whitespace-nowrap hidden min-[420px]:inline"
                title={`更新于 ${formatTimeOfDay(updatedAtMs)}`}
              >
                {formatRelativeShort(updatedAtMs)}
              </span>
            )}
            <span className="text-sm font-semibold text-[var(--color-neutral)] truncate">
              {provider.name}
            </span>
          </div>
          <button
            data-testid="provider-url"
            onClick={handleUrlClick}
            className="text-xs text-[var(--color-link)] hover:underline truncate block text-left"
          >
            {displayUrl}
          </button>
        </div>

        {}
        <div className="shrink-0 flex items-center gap-3">
          {}
          {balance && (
            <div
              data-testid="quota-balance"
              data-tone={balanceTone}
              className={cn(
                "shrink-0 text-xs font-mono tabular-nums whitespace-nowrap",
                TONE_TEXT[balanceTone]
              )}
            >
              <span className="text-[var(--color-muted)] mr-1">剩余</span>
              {balanceText}
              <span className="text-[var(--color-muted)] ml-0.5">{balance.unit}</span>
            </div>
          )}

          {}
          <button
            data-testid="pin-btn"
            onClick={handlePin}
            className={cn(
              "p-1.5 rounded hover:bg-accent transition-colors",
              provider.pinned
                ? "text-[var(--color-primary)]"
                : "text-[var(--color-muted)] hover:text-[var(--color-primary)]"
            )}
            title={provider.pinned ? "取消钉住" : "钉住到托盘"}
          >
            <Pin className={cn("h-4 w-4", provider.pinned && "fill-current")} />
          </button>

          {}
          <button
            data-testid="refresh-btn"
            onClick={handleRefresh}
            className="text-[var(--color-muted)] hover:text-[var(--color-primary)] transition-colors"
            title="Refresh quota"
          >
            <RefreshCw className="h-4 w-4" />
          </button>

          {}
          {onEdit && (
            <button
              data-testid="edit-btn"
              onClick={handleEdit}
              className="p-1.5 rounded hover:bg-accent text-[var(--color-muted)] hover:text-[var(--color-foreground)] transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              title="编辑"
            >
              <Pencil className="h-4 w-4" />
            </button>
          )}

          {onCopy && (
            <button
              data-testid="copy-btn"
              onClick={handleCopy}
              className="p-1.5 rounded hover:bg-accent text-[var(--color-muted)] hover:text-[var(--color-foreground)] transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              title="复制"
            >
              <Copy className="h-4 w-4" />
            </button>
          )}

          {isLlm && onTokenUsage && (
            <button
              data-testid="usage-btn"
              onClick={handleTokenUsage}
              className="p-1.5 rounded hover:bg-accent text-[var(--color-muted)] hover:text-[var(--color-foreground)] transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              title="用量"
            >
              <BarChart3 className="h-4 w-4" />
            </button>
          )}

          {isLlm && onTest && (
            <button
              data-testid="test-btn"
              onClick={handleTest}
              className="p-1.5 rounded hover:bg-accent text-[var(--color-muted)] hover:text-[var(--color-foreground)] transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              title="测试连接"
            >
              <Terminal className="h-4 w-4" />
            </button>
          )}

          <button
            data-testid="delete-btn"
            onClick={handleDelete}
            className="p-1.5 rounded hover:bg-accent text-[var(--color-muted)] hover:text-[var(--color-foreground)] transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
            title="删除"
          >
            <Trash2 className="h-4 w-4" />
          </button>
        </div>
      </div>

      {}
      {extras && extras.length > 0 && (
        <details
          data-testid="extras-foldout"
          className="group mx-3 mt-1 mb-1 rounded-[6px] border border-[var(--color-border)] bg-[var(--color-surface-sunken)]"
        >
          <summary
            data-testid="extras-toggle"
            className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-[var(--color-muted)] cursor-pointer select-none list-none [&::-webkit-details-marker]:hidden marker:hidden hover:text-[var(--color-foreground)] transition-colors"
          >
            <ChevronDown className="h-3 w-3 transition-transform group-open:rotate-180" />
            <span>另有 {extras.length} 个协议端点</span>
          </summary>
          <div className="border-t border-[var(--color-border)] px-2.5 py-1.5 space-y-1">
            {extras.map((e) => (
              <div
                key={`${e.protocol}-${e.base_url}`}
                data-testid="extras-row"
                className="flex items-center gap-2 min-w-0"
              >
                <span className="shrink-0 text-[10px] font-medium text-[var(--color-foreground)] px-1.5 py-0.5 rounded border border-[var(--color-border)] bg-[var(--color-surface)]">
                  {EXTRA_PROTOCOL_LABEL[e.protocol] ?? e.protocol}
                </span>
                <span
                  title={e.base_url}
                  className="text-xs font-mono text-[var(--color-link)] truncate min-w-0"
                >
                  {e.base_url}
                </span>
              </div>
            ))}
          </div>
        </details>
      )}
    </ContextMenu>
  );
};