import {
  RefreshCw,
  Pencil,
  Copy,
  BarChart3,
  Terminal,
  Trash2,
} from "lucide-react";
import { cn, isLlmCategory } from "@/lib/utils";
import type { Provider, Category } from "@/types/api";
import { useCodingPlanQuota } from "@/hooks/useCodingPlanQuota";
import { useProviderQuota } from "@/hooks/useProviderQuota";
import { ContextMenu } from "./ContextMenu";
import { ProviderIcon } from "./Icon";

type Tone = "ok" | "warn" | "crit" | "none";

// ponytail: TONE_BG was specified in the task but has no caller — the donut
// uses SVG stroke (TONE_STROKE) and the balance uses text color (TONE_TEXT).
// Strict-mode tsc rejects unused locals, so TONE_BG is intentionally omitted.
const TONE_TEXT: Record<Tone, string> = {
  ok: "text-[var(--color-success)]",
  warn: "text-[var(--color-accent)]",
  crit: "text-[var(--color-destructive)]",
  none: "text-[var(--color-muted)]",
};
// ponytail: SVG stroke can't use Tailwind utility classes — lookup the raw var.
const TONE_STROKE: Record<Tone, string> = {
  ok: "var(--color-success)",
  warn: "var(--color-accent)",
  crit: "var(--color-destructive)",
  none: "var(--color-muted)",
};

// ponytail: threshold >50 / >=20 mirrors CodingPlanQuotas's tone grading.
function quotaTone(remaining: number | null): Tone {
  if (remaining === null) return "none";
  if (remaining > 50) return "ok";
  if (remaining >= 20) return "warn";
  return "crit";
}
const quotaTextTone = quotaTone;

// ponytail: r=6, stroke=2.2 → circumference ≈ 37.70. Matches preview.
const DONUT_CIRCUMFERENCE = 37.7;

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
}: ProviderCardProps) => {
  const isLlm = isLlmCategory(provider.category_id, categories);

  // Derive a display URL from base_url field if present
  const baseUrlField = provider.fields.find((f) => f.key === "base_url");
  const displayUrl = baseUrlField?.value || "https://example.com";

  // Coding-plan primary tier % — preferred source for the donut.
  const { data: codingPlan } = useCodingPlanQuota(provider.id);
  const pctFromTier = codingPlan?.success
    ? codingPlan.tiers[0]?.remaining_percent ?? null
    : null;

  // QuotaSnapshot remaining/total % — fallback for the donut, and the only
  // source for the balance text. TanStack dedupes both hooks by providerId.
  const { data: snapshot } = useProviderQuota(provider.id);
  const pctFromSnapshot =
    snapshot?.remaining != null && snapshot?.total != null && snapshot.total > 0
      ? (snapshot.remaining / snapshot.total) * 100
      : null;

  const pct = pctFromTier ?? pctFromSnapshot;
  const donutOffset =
    pct === null ? 0 : DONUT_CIRCUMFERENCE * (1 - Math.max(0, Math.min(100, pct)) / 100);
  const tone = quotaTone(pct);

  // Balance lives in the right cluster; only renders when snapshot has a remaining value.
  const balance = snapshot?.remaining != null ? snapshot : null;
  const balanceTone: Tone = pct !== null ? quotaTextTone(pct) : "ok";
  // ponytail: `?? 0` is correct on edge cases — tsc doesn't narrow the
  // property across the ternary even though we just checked != null.
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

  return (
    <ContextMenu providerId={provider.id} onDelete={onDelete}>
      <div
        data-testid="provider-card"
        data-provider-id={provider.id}
        data-selected={selected}
        onClick={onClick}
        onContextMenu={handleContextMenu}
        // Radix DropdownMenu.Trigger (mjs:74-77) calls onOpenToggle() + event.preventDefault()
        // on left-click pointerdown. preventDefault on pointerdown cancels the subsequent click,
        // so the inline button onClick handlers never fire and the menu also opens. Stopping
        // pointerdown at the card prevents the Trigger from intercepting; the click event then
        // fires normally and onClick handlers run. Right-click (button === 2) still bubbles
        // because Radix opens the menu via the contextmenu event, not pointerdown.
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
        {/* Provider icon */}
        <div data-testid="provider-icon" className="shrink-0">
          <ProviderIcon
            preset={provider.preset}
            name={provider.name}
            icon={provider.icon}
            className="w-8 h-8 rounded"
          />
        </div>

        {/* Provider info */}
        <div className="flex-1 min-w-0">
          <div
            data-testid="provider-name"
            className="flex items-center gap-2 min-w-0"
          >
            {pct !== null && (
              <svg
                data-testid="quota-donut"
                viewBox="0 0 16 16"
                width="16"
                height="16"
                className="shrink-0"
                aria-label={`${Math.round(pct)}% remaining`}
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

        {/* Right meta cluster */}
        <div className="shrink-0 flex items-center gap-3">
          {/* Balance (only when QuotaSnapshot.remaining is set) */}
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

          {/* Refresh button */}
          <button
            data-testid="refresh-btn"
            onClick={handleRefresh}
            className="text-[var(--color-muted)] hover:text-[var(--color-primary)] transition-colors"
            title="Refresh quota"
          >
            <RefreshCw className="h-4 w-4" />
          </button>

          {/* Inline action buttons */}
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
    </ContextMenu>
  );
};