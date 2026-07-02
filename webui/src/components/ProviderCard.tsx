import {
  Clock,
  RefreshCw,
  Pencil,
  Copy,
  BarChart3,
  Terminal,
  Trash2,
} from "lucide-react";
import { cn, isLlmCategory } from "@/lib/utils";
import type { Provider, Category } from "@/types/api";
import { formatRelative } from "@/lib/format";
import { ContextMenu } from "./ContextMenu";
import { ProviderIcon } from "./Icon";

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

  // Quota display: use remaining or used field
  const quotaField = provider.fields.find(
    (f) => f.key === "quota_remaining" || f.key === "quota_used"
  );
  const quotaNum = quotaField ? Number(quotaField.value) : null;
  const quotaUnit = "USD";

  const timeAgo = formatRelative(provider.updated_at * 1000, "suffix");

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
            className="text-sm font-semibold text-[var(--color-neutral)] truncate"
          >
            {provider.name}
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
          {/* Clock + time */}
          <div data-testid="clock-meta" className="flex items-center gap-1 text-[var(--color-muted)]">
            <Clock className="h-4 w-4" />
            <span data-testid="time-text" className="text-xs">
              {timeAgo}
            </span>
          </div>

          {/* Refresh button */}
          <button
            data-testid="refresh-btn"
            onClick={handleRefresh}
            className="text-[var(--color-muted)] hover:text-[var(--color-primary)] transition-colors"
            title="Refresh quota"
          >
            <RefreshCw className="h-4 w-4" />
          </button>

          {/* Quota */}
          {quotaNum !== null && (
            <div data-testid="quota" className="flex items-baseline gap-0.5">
              <span data-testid="quota-num" className="text-sm font-semibold text-[var(--color-success)]">
                {quotaNum}
              </span>
              <span data-testid="quota-unit" className="text-xs text-[var(--color-muted)]">
                {quotaUnit}
              </span>
            </div>
          )}

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