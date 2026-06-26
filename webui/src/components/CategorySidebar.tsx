import { useState, useCallback } from "react";
import { cn } from "@/lib/utils";
import { Icon, useToast } from "./Icon";
import { ProviderList } from "./ProviderList";
import { AddCredentialModal } from "./AddCredentialModal";
import { ConfirmDialog } from "./ConfirmDialog";
import { useCategories } from "@/hooks/useCategories";
import { useProviders, filterProviders } from "@/hooks/useProviders";
import { deleteCategory, addCategory } from "@/lib/api";
import type { Provider } from "@/types/api";

interface CategorySidebarProps {
  selectedCategoryId: number | null;
  onSelectCategory: (id: number | null) => void;
  selectedProviderId: number | null;
  onSelectProvider: (id: number) => void;
}

export function CategorySidebar({
  selectedCategoryId,
  onSelectCategory,
  selectedProviderId,
  onSelectProvider,
}: CategorySidebarProps) {
  const { data: categories = [], refetch: refetchCategories } = useCategories();
  const { data: providers = [], refetch: refetchProviders } = useProviders();
  const { showToast } = useToast();

  const [expandedCategories, setExpandedCategories] = useState<Set<number>>(
    new Set(categories.map((c) => c.id))
  );
  const [addCredentialOpen, setAddCredentialOpen] = useState(false);
  const [addCategoryOpen, setAddCategoryOpen] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<{ type: "category" | "provider"; id: number; name: string } | null>(null);
  const [newCategoryName, setNewCategoryName] = useState("");
  const [searchQuery, setSearchQuery] = useState("");

  const toggleCategory = useCallback((categoryId: number) => {
    setExpandedCategories((prev) => {
      const next = new Set(prev);
      if (next.has(categoryId)) {
        next.delete(categoryId);
      } else {
        next.add(categoryId);
      }
      return next;
    });
  }, []);

  const handleDeleteCategory = useCallback(async () => {
    if (!deleteConfirm || deleteConfirm.type !== "category") return;
    try {
      await deleteCategory({ id: deleteConfirm.id, migrate_to: 1 });
      await refetchCategories();
      await refetchProviders();
      showToast("分类已删除", "success");
    } catch {
      showToast("删除失败", "error");
    }
    setDeleteConfirm(null);
  }, [deleteConfirm, refetchCategories, refetchProviders, showToast]);

  const handleAddCategory = useCallback(async () => {
    if (!newCategoryName.trim()) {
      showToast("请输入分类名称", "error");
      return;
    }
    try {
      await addCategory({ name: newCategoryName.trim() });
      await refetchCategories();
      showToast("分类已添加", "success");
      setNewCategoryName("");
      setAddCategoryOpen(false);
    } catch {
      showToast("添加失败", "error");
    }
  }, [newCategoryName, refetchCategories, showToast]);

  const getProvidersByCategory = useCallback(
    (categoryId: number): Provider[] => {
      return providers.filter((p) => p.category_id === categoryId);
    },
    [providers]
  );

  const isSearching = searchQuery.trim() !== "";
  // Unified shape: always Array<{category, providers}>, no union types.
  const visibleGroups = categories.map((c) => ({
    category: c,
    providers: isSearching
      ? filterProviders(getProvidersByCategory(c.id), searchQuery)
      : getProvidersByCategory(c.id),
  }));
  const displayList = isSearching
    ? visibleGroups.filter((g) => g.providers.length > 0)
    : visibleGroups;
  const displayEmpty = displayList.length === 0;
  const displayMessage = isSearching ? "无匹配" : "暂无分类";

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="px-4 py-3 border-b border-border">
        <h2 className="font-semibold text-sm">凭证管理</h2>
      </div>

      {/* Search */}
      <div className="px-4 py-2 border-b border-border">
        <div className="relative">
          <Icon name="search" className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground pointer-events-none" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Escape" && setSearchQuery("")}
            placeholder="搜索凭证名 / 字段名..."
            className="w-full h-8 pl-8 pr-3 rounded-md border border-input bg-transparent text-sm focus:outline-none focus:ring-1 focus:ring-ring"
          />
        </div>
      </div>

      {/* Categories */}
      <div className="flex-1 overflow-y-auto py-2">
        {displayEmpty ? (
          <div className="px-4 py-8 text-center text-sm text-muted-foreground">
            <p>{displayMessage}</p>
          </div>
        ) : (
          displayList.map(({ category, providers: categoryProviders }) => {
            const isExpanded = isSearching || expandedCategories.has(category.id);
            const color = "#8e8e8e"; // Default color for categories

            return (
              <div key={category.id} className="mb-1">
                {/* Category header */}
                <button
                  type="button"
                  onClick={() => toggleCategory(category.id)}
                  className={cn(
                    "w-full flex items-center gap-2 px-4 py-2 text-sm font-medium hover:bg-accent transition-colors",
                    selectedCategoryId === category.id && "bg-accent"
                  )}
                >
                  {/* Expand/collapse icon */}
                  <span className="text-muted-foreground text-xs">
                    {isExpanded ? "▼" : "▶"}
                  </span>

                  {/* Color dot */}
                  <span
                    className="w-2 h-2 rounded-full"
                    style={{ backgroundColor: color }}
                  />

                  {/* Category name */}
                  <span className="flex-1 text-left truncate">{category.name}</span>

                  {/* Provider count */}
                  <span className="text-xs text-muted-foreground">
                    {categoryProviders.length}
                  </span>
                </button>

                {/* Provider list (collapsible) */}
                {isExpanded && (
                  <div className="ml-4 pl-3 border-l border-border">
                    <ProviderList
                      providers={categoryProviders}
                      selectedProviderId={selectedProviderId}
                      onSelectProvider={onSelectProvider}
                      onAddProvider={() => {
                        onSelectCategory(category.id);
                        setAddCredentialOpen(true);
                      }}
                    />
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>

      {/* Footer actions */}
      <div className="px-4 py-3 border-t border-border space-y-2">
        <button
          type="button"
          onClick={() => setAddCategoryOpen(true)}
          className="w-full flex items-center justify-center gap-2 px-3 py-2 rounded-md text-sm text-muted-foreground hover:bg-accent hover:text-foreground transition-colors"
        >
          <Icon name="plus" className="w-3.5 h-3.5" />
          <span>新建分类</span>
        </button>
      </div>

      {/* Add Credential Modal */}
      <AddCredentialModal
        open={addCredentialOpen}
        onClose={() => setAddCredentialOpen(false)}
        defaultCategoryId={selectedCategoryId ?? undefined}
      />

      {/* Add Category Modal */}
      <ConfirmDialog
        open={addCategoryOpen}
        onClose={() => {
          setAddCategoryOpen(false);
          setNewCategoryName("");
        }}
        onConfirm={handleAddCategory}
        title="新建分类"
        message={
          <div className="space-y-3">
            <p className="text-sm text-muted-foreground">输入分类名称</p>
            <input
              type="text"
              value={newCategoryName}
              onChange={(e) => setNewCategoryName(e.target.value)}
              placeholder="例如：LLM、数据库、开发工具..."
              className="w-full h-9 px-3 rounded-md border border-input bg-transparent text-sm"
              autoFocus
            />
          </div>
        }
        confirmText="添加"
        variant="default"
      />

      {/* Delete Confirm Dialog */}
      {deleteConfirm && (
        <ConfirmDialog
          open={true}
          onClose={() => setDeleteConfirm(null)}
          onConfirm={deleteConfirm.type === "category" ? handleDeleteCategory : () => {}}
          title={`删除 ${deleteConfirm.type === "category" ? "分类" : "凭证"}`}
          message={`确定要删除 "${deleteConfirm.name}" 吗？此操作无法撤销。`}
          confirmText="删除"
          variant="destructive"
        />
      )}
    </div>
  );
}
