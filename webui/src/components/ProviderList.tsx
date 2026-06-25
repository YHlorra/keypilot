import * as React from "react";
import { cn } from "@/lib/utils";
import { Icon, ProviderIcon, PRESET_COLORS } from "./Icon";
import type { Provider } from "@/types/api";

interface ProviderListProps {
  providers: Provider[];
  selectedProviderId: number | null;
  onSelectProvider: (id: number) => void;
  onAddProvider?: () => void;
}

export const ProviderList = React.memo(function ProviderList({
  providers,
  selectedProviderId,
  onSelectProvider,
  onAddProvider,
}: ProviderListProps) {
  if (providers.length === 0) {
    return (
      <div className="py-8 text-center text-sm text-muted-foreground">
        <p>暂无凭证</p>
        {onAddProvider && (
          <button
            type="button"
            onClick={onAddProvider}
            className="mt-2 text-primary hover:underline"
          >
            + 添加凭证
          </button>
        )}
      </div>
    );
  }

  return (
    <div className="space-y-1">
      {providers.map((provider) => {
        const isSelected = provider.id === selectedProviderId;
        const color = provider.preset ? PRESET_COLORS[provider.preset] : "#8e8e8e";

        return (
          <button
            key={provider.id}
            type="button"
            onClick={() => onSelectProvider(provider.id)}
            className={cn(
              "w-full flex items-center gap-2 px-3 py-2 rounded-md text-sm transition-colors text-left",
              isSelected
                ? "bg-primary/10 border border-primary/30"
                : "hover:bg-accent"
            )}
          >
            {/* Preset color dot */}
            <span
              className="w-2 h-2 rounded-full flex-shrink-0"
              style={{ backgroundColor: color }}
            />

            {/* Provider icon */}
            <ProviderIcon
              preset={provider.preset}
              name={provider.name}
              className="w-4 h-4 flex-shrink-0"
            />

            {/* Name */}
            <span className="flex-1 truncate">{provider.name}</span>

            {/* Pinned indicator */}
            {provider.pinned && <span className="text-xs">📌</span>}
          </button>
        );
      })}

      {/* Add provider button */}
      {onAddProvider && (
        <button
          type="button"
          onClick={onAddProvider}
          className="w-full flex items-center gap-2 px-3 py-2 rounded-md text-sm text-muted-foreground hover:bg-accent hover:text-foreground transition-colors"
        >
          <Icon name="plus" className="w-3.5 h-3.5" />
          <span>新建凭证</span>
        </button>
      )}
    </div>
  );
});
