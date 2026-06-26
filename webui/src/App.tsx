import { useState, useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { Titlebar } from "./components/Titlebar";
import { TopBar } from "./components/TopBar";
import { TopRightActions } from "./components/TopRightActions";
import { ProviderGrid } from "./components/ProviderGrid";
import { SectionLabel } from "./components/SectionLabel";
import { ProviderDetailModal } from "./components/ProviderDetailModal";
import { SettingsModal } from "./components/SettingsModal";
import { AddCredentialModal } from "./components/AddCredentialModal";
import { useTheme } from "./hooks/useTheme";
import { useProviders } from "./hooks/useProviders";
import { useCategories } from "./hooks/useCategories";
import { testConnection, fetchQuota } from "@/lib/api";
import UsagePage from "./pages/UsagePage";
import type { Provider } from "./types/api";

function filterProviders(
  providers: Provider[],
  search: string,
  categoryFilter: number | "all"
): Provider[] {
  let result = providers;
  if (categoryFilter !== "all") {
    result = result.filter((p) => p.category_id === categoryFilter);
  }
  if (search.trim()) {
    const q = search.toLowerCase();
    result = result.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        (p.preset && p.preset.toLowerCase().includes(q)) ||
        (p.notes && p.notes.toLowerCase().includes(q))
    );
  }
  return result;
}

export default function App() {
  // M6 state shape
  const [density, setDensity] = useState<"1" | "2">(
    () => (localStorage.getItem("keypilot.density") as "1" | "2") ?? "1"
  );
  const [activeProviderId, setActiveProviderId] = useState<number | null>(null);
  const [categoryFilter, setCategoryFilter] = useState<number | "all">("all");
  const [search, setSearch] = useState<string>("");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [currentPage, setCurrentPage] = useState<"credentials" | "usage">("credentials");
  const [addCredOpen, setAddCredOpen] = useState(false);

  const queryClient = useQueryClient();

  // M-12 CRITICAL: theme effect runs BEFORE density effect to avoid flash
  useTheme(); // writes data-theme attribute to <html>

  // Density effect -- runs AFTER theme effect (order matters!)
  useEffect(() => {
    document.documentElement.setAttribute("data-density", density);
    localStorage.setItem("keypilot.density", density);
  }, [density]);

  // Handlers
  const handleTest = async (id: number) => {
    try {
      await testConnection({ id });
      queryClient.invalidateQueries({ queryKey: ["provider", id] });
      queryClient.invalidateQueries({ queryKey: ["providers"] });
    } catch (e) {
      console.error("test connection failed", e);
    }
  };

  const handleFetchQuota = async (id: number) => {
    try {
      await fetchQuota({ id });
      queryClient.invalidateQueries({ queryKey: ["provider", id] });
    } catch (e) {
      console.error("fetch quota failed", e);
    }
  };

  // Filtered provider list for ProviderGrid
  const { data: allProviders = [] } = useProviders();
  const { data: categories = [] } = useCategories();
  const filteredProviders = filterProviders(allProviders, search, categoryFilter);

  return (
    <div className="flex flex-col h-screen bg-background text-foreground">
      <Titlebar
        rightActions={
          <TopRightActions
            onSettingsClick={() => setSettingsOpen(true)}
            onTokenUsageClick={() => setCurrentPage("usage")}
            onAddClick={() => setAddCredOpen(true)}
            isOnUsagePage={currentPage === "usage"}
          />
        }
      />

      <TopBar
        search={search}
        onSearchChange={setSearch}
        categoryFilter={categoryFilter}
        onCategoryChange={setCategoryFilter}
        density={density}
        onDensityChange={setDensity}
        currentPage={currentPage}
        onPageChange={setCurrentPage}
        categories={categories}
      />

      {currentPage === "credentials" && (
        <main className="flex-1 overflow-y-auto pt-[108px]">
          {categoryFilter !== "all" && (
            <SectionLabel>
              {categories.find((c) => c.id === categoryFilter)?.name ?? ""}
            </SectionLabel>
          )}
          <ProviderGrid
            density={density}
            providers={filteredProviders}
            onSelectProvider={(id) => setActiveProviderId(id)}
            onAddClick={() => setAddCredOpen(true)}
          />
        </main>
      )}
      {currentPage === "usage" && (
        <main className="flex-1 overflow-y-auto pt-[108px]">
          <UsagePage />
        </main>
      )}

      <ProviderDetailModal
        providerId={activeProviderId}
        onClose={() => setActiveProviderId(null)}
        onTest={handleTest}
        onFetchQuota={handleFetchQuota}
      />

      <SettingsModal open={settingsOpen} onClose={() => setSettingsOpen(false)} />

      <AddCredentialModal
        open={addCredOpen}
        onClose={() => setAddCredOpen(false)}
        defaultCategoryId={categoryFilter !== "all" ? categoryFilter : undefined}
      />
    </div>
  );
}