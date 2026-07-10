import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";
import { ProviderCard } from "./ProviderCard";
import { Button } from "./ui/button";
import type { Provider } from "@/types/api";
import { useProviders } from "@/hooks/useProviders";
import { useQueryClient } from "@tanstack/react-query";
import { deleteProviderViaAction, pinProvider, unpinProvider } from "@/lib/api";

interface ProviderGridProps {
  providers: Provider[];
  categories: import("@/types/api").Category[];
  selectedId?: number | null;
  onSelectProvider: (id: number) => void;
  onRefreshProvider?: (id: number) => void;
  onAddClick?: () => void;
  onCopy?: (id: number) => void;
  onEdit?: (id: number) => void;
  onTokenUsage?: (id: number) => void;
  onTest?: (id: number) => void;
}

function SkeletonCard({ testId }: { testId: string }) {
  return (
    <div
      data-testid={testId}
      className="flex items-center gap-3 px-4 py-3 rounded-[8px] border border-[var(--color-border)] bg-[var(--color-surface)] animate-pulse"
    >
      <div className="w-4 h-4 rounded bg-[var(--color-border)]" />
      <div className="w-8 h-8 rounded-full bg-[var(--color-border)]" />
      <div className="flex-1 space-y-2">
        <div className="h-3 w-24 rounded bg-[var(--color-border)]" />
        <div className="h-2 w-32 rounded bg-[var(--color-border)]" />
      </div>
    </div>
  );
}

export const ProviderGrid = ({
  providers,
  categories,
  selectedId,
  onSelectProvider,
  onRefreshProvider,
  onAddClick,
  onCopy,
  onEdit,
  onTokenUsage,
  onTest,
}: ProviderGridProps) => {
  const { isLoading, isError, refetch } = useProviders();
  const queryClient = useQueryClient();

  
  const [density, setDensity] = useState<"1" | "2">("1");

  useEffect(() => {
    const updateDensity = () => {
      const d = document.documentElement.getAttribute("data-density") as "1" | "2" | null;
      if (d === "1" || d === "2") setDensity(d);
    };

    updateDensity();
    const observer = new MutationObserver(updateDensity);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-density"] });
    return () => observer.disconnect();
  }, []);

  const handleDelete = async (id: number) => {
    try {
      await deleteProviderViaAction({ id });
      await queryClient.invalidateQueries({ queryKey: ["providers"] });
    } catch {
      
    }
  };

  const handlePin = async (id: number) => {
    const provider = providers.find((p) => p.id === id);
    try {
      if (provider?.pinned) {
        await unpinProvider({ id });
      } else {
        await pinProvider({ id });
      }
      await queryClient.invalidateQueries({ queryKey: ["providers"] });
    } catch {
      
    }
  };

  if (isLoading) {
    return (
      <div
        data-testid="provider-grid"
        className={cn(
          "grid gap-[18px] p-7",
          density === "1" ? "grid-cols-1" : "grid-cols-1 sm:grid-cols-2",
          "max-[640px]:grid-cols-1"
        )}
      >
        <SkeletonCard testId="skeleton-card-1" />
        <SkeletonCard testId="skeleton-card-2" />
      </div>
    );
  }

  if (isError) {
    return (
      <div data-testid="provider-grid" className="flex flex-col items-center gap-2 p-7">
        <div data-testid="error-state" className="text-sm text-[var(--color-muted)]">
          Failed to load credentials
        </div>
        <button
          data-testid="retry-btn"
          onClick={() => refetch()}
          className="text-xs text-[var(--color-link)] hover:underline"
        >
          Retry
        </button>
      </div>
    );
  }

  if (providers.length === 0) {
    return (
      <div data-testid="provider-grid" className="flex flex-col items-center gap-1 p-7">
        <div data-testid="empty-state" className="text-center">
          <h2
            data-testid="empty-title"
            className="font-serif text-xl text-[var(--color-neutral)]"
          >
            No credentials yet
          </h2>
          <p
            data-testid="empty-subtitle"
            className="text-sm text-[var(--color-muted)]"
          >
            Add your first credential to get started
          </p>
          {onAddClick && (
            <Button size="sm" onClick={onAddClick} className="mt-3">
              + 添加凭证
            </Button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div
      data-testid="provider-grid"
      data-density={density}
      className={cn(
        "grid gap-[18px] p-7",
        density === "1" ? "grid-cols-1" : "grid-cols-1 sm:grid-cols-2",
        "max-[640px]:grid-cols-1"
      )}
    >
      {providers.map((provider) => (
        <ProviderCard
          key={provider.id}
          provider={provider}
          categories={categories}
          selected={selectedId === provider.id}
          onClick={() => onSelectProvider(provider.id)}
          onRefresh={(e) => {
            e.stopPropagation();
            onRefreshProvider?.(provider.id);
          }}
          onDelete={handleDelete}
          onCopy={onCopy}
          onEdit={onEdit}
          onTokenUsage={onTokenUsage}
          onTest={onTest}
          onPin={handlePin}
        />
      ))}
    </div>
  );
};