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
import { LeftRail } from "./components/LeftRail";
import { useTheme } from "./hooks/useTheme";
import { useProviders } from "./hooks/useProviders";
import { useCategories } from "./hooks/useCategories";
import { useLastAutoImport } from "./hooks/useUsage";
import { useUsageTick } from "./hooks/useUsageTick";
import { useToast } from "./components/Icon";
import {
  copyCredential,
  testAndRefresh,
  openForEdit,
  fetchQuota,
} from "@/lib/api";
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
  
  const [density, setDensity] = useState<"1" | "2">(
    () => (localStorage.getItem("keypilot.density") as "1" | "2") ?? "1"
  );
  const [activeProviderId, setActiveProviderId] = useState<number | null>(null);
  const [categoryFilter, setCategoryFilter] = useState<number | "all">("all");
  const [search, setSearch] = useState<string>("");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [currentPage, setCurrentPage] = useState<"credentials" | "usage">("credentials");
  const [addCredOpen, setAddCredOpen] = useState(false);
  const [usageFilterProviderName, setUsageFilterProviderName] = useState<string | null>(null);

  const queryClient = useQueryClient();

  
  useTheme(); 

  
  useEffect(() => {
    document.documentElement.setAttribute("data-density", density);
    localStorage.setItem("keypilot.density", density);
  }, [density]);

  const { showToast } = useToast();

  
  
  
  const { data: autoImport } = useLastAutoImport();

  
  
  
  
  useUsageTick();
  useEffect(() => {
    if (!autoImport) return;
    const { total_imported, total_errors } = autoImport;
    if (total_imported > 0 && total_errors === 0) {
      showToast(`已导入 ${total_imported} 行 token 用量`, "success");
    } else if (total_imported > 0 && total_errors > 0) {
      showToast(`已导入 ${total_imported} 行，${total_errors} 条解析失败`, "error");
    } else if (total_errors > 0) {
      const firstErr = autoImport.entries
        .flatMap((e) => e.errors)
        .slice(0, 1)[0];
      showToast(`导入失败：${firstErr ?? "未知错误"}`, "error");
    }
    
  }, [autoImport, showToast]);

  
  const handleTest = async (id: number) => {
    try {
      const result = await testAndRefresh({ id });
      queryClient.invalidateQueries({ queryKey: ["provider", id] });
      queryClient.invalidateQueries({ queryKey: ["providers"] });
      if (result.test === "ok") {
        showToast("连接成功", "success");
      } else {
        showToast(`连接失败：${result.test}`, "error");
      }
    } catch (e) {
      console.error("test and refresh failed", e);
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

  const handleCopy = async (id: number) => {
    try {
      const result = await copyCredential({ id });
      await navigator.clipboard.writeText(result.value);
      showToast("已复制", "success");
    } catch (e) {
      console.error("copy failed", e);
    }
  };

  const handleEdit = async (id: number) => {
    try {
      await openForEdit({ id });
      setActiveProviderId(id);
    } catch (e) {
      console.error("openForEdit failed", e);
      setActiveProviderId(id);
    }
  };

  const handleTokenUsage = (id: number) => {
    const provider = allProviders.find((p) => p.id === id);
    setUsageFilterProviderName(provider?.name ?? null);
    setCurrentPage("usage");
  };

  
  const { data: allProviders = [] } = useProviders();
  const { data: categories = [] } = useCategories();
  const filteredProviders = filterProviders(allProviders, search, categoryFilter);

  return (
    <div className="flex h-screen bg-background text-foreground">
      {}
      <LeftRail
        currentPage={currentPage}
        onPageChange={setCurrentPage}
        onSettingsClick={() => setSettingsOpen(true)}
      />

      {}
      <div className="flex-1 flex flex-col min-w-0">
        <Titlebar
          rightActions={
            <TopRightActions
              onSettingsClick={() => setSettingsOpen(true)}
              onAddClick={() => setAddCredOpen(true)}
            />
          }
        />

        {currentPage === "credentials" && (
          <TopBar
            search={search}
            onSearchChange={setSearch}
            categoryFilter={categoryFilter}
            onCategoryChange={setCategoryFilter}
            density={density}
            onDensityChange={setDensity}
            categories={categories}
          />
        )}

        {currentPage === "credentials" && (
          <div className="flex-1 overflow-y-auto pb-[56px] md:pb-0">
            <main className="flex-1">
              {categoryFilter !== "all" && (
                <SectionLabel>
                  {categories.find((c) => c.id === categoryFilter)?.name ?? ""}
                </SectionLabel>
              )}
              <ProviderGrid
                providers={filteredProviders}
                categories={categories}
                onSelectProvider={(id) => setActiveProviderId(id)}
                onAddClick={() => setAddCredOpen(true)}
                onCopy={handleCopy}
                onEdit={handleEdit}
                onTokenUsage={handleTokenUsage}
                onTest={handleTest}
              />
            </main>
          </div>
        )}
        {currentPage === "usage" && (
          
          <div className="flex-1 overflow-y-auto pb-[56px] md:pb-0">
            <UsagePage filterProviderName={usageFilterProviderName} />
          </div>
        )}
      </div>

      <ProviderDetailModal
        providerId={activeProviderId}
        categories={categories}
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
