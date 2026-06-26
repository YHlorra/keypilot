import { Clock, RefreshCw } from "lucide-react";
import { formatDistanceToNow } from "date-fns";
import { cn } from "@/lib/utils";
import type { Provider } from "@/types/api";
import { ContextMenu } from "./ContextMenu";

interface ProviderCardProps {
  provider: Provider;
  selected: boolean;
  onClick: () => void;
  onRefresh: (e: React.MouseEvent) => void;
  onDelete: (id: number) => void;
}

const PRESET_TINTS: Record<string, { bg: string; label: string }> = {
  openai: { bg: "#46a758", label: "AI" },
  deepseek: { bg: "#1d4ed8", label: "DS" },
  anthropic: { bg: "#f76808", label: "AN" },
  github: { bg: "#6b6b65", label: "GH" },
  postgres: { bg: "#1d4ed8", label: "PG" },
  redis: { bg: "#b42318", label: "RE" },
};

function getFamilyTint(preset: string | null): { bg: string; label: string } {
  if (preset && PRESET_TINTS[preset]) return PRESET_TINTS[preset];
  return { bg: "var(--color-muted)", label: "" };
}

function GripIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
      <circle cx="4" cy="3" r="1.2" />
      <circle cx="8" cy="3" r="1.2" />
      <circle cx="4" cy="7" r="1.2" />
      <circle cx="8" cy="7" r="1.2" />
      <circle cx="4" cy="11" r="1.2" />
      <circle cx="8" cy="11" r="1.2" />
    </svg>
  );
}

export const ProviderCard = ({
  provider,
  selected,
  onClick,
  onRefresh,
  onDelete,
}: ProviderCardProps) => {
  const tint = getFamilyTint(provider.preset);
  const iconLabel = tint.label || provider.name.charAt(0).toUpperCase();

  // Derive a display URL from base_url field if present
  const baseUrlField = provider.fields.find((f) => f.key === "base_url");
  const displayUrl = baseUrlField?.value || "https://example.com";

  // Quota display: use remaining or used field
  const quotaField = provider.fields.find(
    (f) => f.key === "quota_remaining" || f.key === "quota_used"
  );
  const quotaNum = quotaField ? Number(quotaField.value) : null;
  const quotaUnit = provider.preset === "postgres" ? "GB" : "USD";

  const timeAgo = formatDistanceToNow(new Date(provider.updated_at * 1000), {
    addSuffix: true,
  });

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

  return (
    <ContextMenu providerId={provider.id} onDelete={onDelete}>
      <div
        data-testid="provider-card"
        data-provider-id={provider.id}
        data-selected={selected}
        onClick={onClick}
        onContextMenu={handleContextMenu}
        className={cn(
          "relative flex items-center gap-3 px-4 py-3 rounded-[8px] border cursor-pointer transition-colors select-none",
          "bg-[var(--color-surface)] border-[var(--color-border)]",
          "hover:border-[var(--color-primary)]",
          selected
            ? "bg-[var(--color-surface-sunken)] border-l-4 border-l-[var(--color-primary)] border-[var(--color-primary)]"
            : "border border-[var(--color-border)]"
        )}
      >
        {/* Drag handle */}
        <div
          data-testid="drag-handle"
          className="shrink-0 cursor-grab text-[var(--color-muted)] active:cursor-grabbing"
        >
          <GripIcon />
        </div>

        {/* Provider icon */}
        <div
          data-testid="provider-icon"
          className="shrink-0 w-8 h-8 rounded-full flex items-center justify-center text-white text-xs font-mono font-bold"
          style={{ backgroundColor: tint.bg }}
        >
          {iconLabel}
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
        </div>
      </div>
    </ContextMenu>
  );
};