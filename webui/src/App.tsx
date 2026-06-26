import { useState, useEffect } from "react";
import { Titlebar } from "./components/Titlebar";
import { TopBar } from "./components/TopBar";
import { ProviderGrid } from "./components/ProviderGrid";
import { SectionLabel } from "./components/SectionLabel";
import { ProviderDetailModal } from "./components/ProviderDetailModal";
import { SettingsModal } from "./components/SettingsModal";
import { useTheme } from "./hooks/useTheme";
import { useProviders } from "./hooks/useProviders";
import { useCategories } from "./hooks/useCategories";
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

  // M-12 CRITICAL: theme effect runs BEFORE density effect to avoid flash
  useTheme(); // writes data-theme attribute to <html>

  // Density effect -- runs AFTER theme effect (order matters!)
  useEffect(() => {
    document.documentElement.setAttribute("data-density", density);
    localStorage.setItem("keypilot.density", density);
  }, [density]);

  // Handlers
  const handleTest = (id: number) => {
    console.log("test", id);
  };

  const handleFetchQuota = (id: number) => {
    console.log("fetch quota", id);
  };

  // Filtered provider list for ProviderGrid
  const { data: allProviders = [] } = useProviders();
  const { data: categories = [] } = useCategories();
  const filteredProviders = filterProviders(allProviders, search, categoryFilter);

  return (
    <div className="flex flex-col h-screen bg-background text-foreground">
      <Titlebar onSettingsClick={() => setSettingsOpen(true)} />

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
        <main className="flex-1 overflow-y-auto pt-[104px]">
          {categoryFilter !== "all" && (
            <SectionLabel>
              {categories.find((c) => c.id === categoryFilter)?.name ?? ""}
            </SectionLabel>
          )}
          <ProviderGrid
            density={density}
            providers={filteredProviders}
            onSelectProvider={(id) => setActiveProviderId(id)}
          />
        </main>
      )}
      {currentPage === "usage" && (
        <main className="flex-1 overflow-y-auto pt-[104px]">
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
    </div>
  );
}