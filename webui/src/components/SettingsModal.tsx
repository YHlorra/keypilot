import * as React from "react";
import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { Modal } from "./Modal";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { ThemeToggle } from "./ThemeToggle";
import { ConfirmDialog } from "./ConfirmDialog";
import { useCategories } from "@/hooks/useCategories";
import { useProviders } from "@/hooks/useProviders";
import { addCategory, deleteCategory } from "@/lib/api";

interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
}

export const SettingsModal = React.memo(function SettingsModal({ open, onClose }: SettingsModalProps) {
  const [newCategoryName, setNewCategoryName] = useState("");
  const [deleteCategoryFor, setDeleteCategoryFor] = useState<number | null>(null);
  const [migrateTo, setMigrateTo] = useState<number>(0);
  const [confirmDeleteOpen, setConfirmDeleteOpen] = useState(false);

  const { data: categories = [], refetch } = useCategories();
  const { data: providers = [] } = useProviders();
  const queryClient = useQueryClient();

  const handleAddCategory = async () => {
    if (!newCategoryName.trim()) return;
    try {
      await addCategory({ name: newCategoryName.trim() });
      await refetch();
      setNewCategoryName("");
    } catch (e) {
      console.error("add category failed", e);
    }
  };

  const handleDeleteClick = (id: number) => {
    setDeleteCategoryFor(id);
    // Default migrate target: first other category
    const others = categories.filter((c) => c.id !== id);
    setMigrateTo(others[0]?.id ?? 0);
    setConfirmDeleteOpen(true);
  };

  const handleConfirmDelete = async () => {
    if (deleteCategoryFor === null) return;
    try {
      await deleteCategory({ id: deleteCategoryFor, migrate_to: migrateTo });
      await queryClient.invalidateQueries({ queryKey: ["providers"] });
      await refetch();
      setConfirmDeleteOpen(false);
      setDeleteCategoryFor(null);
    } catch (e) {
      console.error("delete category failed", e);
    }
  };

  const handleMigrateTargetChange = (id: number) => {
    setMigrateTo(id);
  };

  // Providers count per category
  const providersInCategory = (id: number) => providers.filter((p) => p.category_id === id).length;

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="设置"
      footer={
        <Button variant="ghost" onClick={onClose}>
          关闭
        </Button>
      }
    >
      <div className="space-y-6">
        {/* Theme */}
        <div>
          <h3 className="text-sm font-medium mb-3">主题</h3>
          <ThemeToggle />
        </div>

        {/* Categories */}
        <div>
          <h3 className="text-sm font-medium mb-3">分类</h3>
          <div className="space-y-2">
            {categories.map((cat) => {
              const count = providersInCategory(cat.id);
              return (
                <div key={cat.id} className="flex items-center justify-between">
                  <span className="text-sm">
                    {cat.name}
                    {cat.is_default && (
                      <span className="ml-1 text-xs text-muted-foreground">(默认)</span>
                    )}
                    {count > 0 && (
                      <span className="ml-1 text-xs text-muted-foreground">({count})</span>
                    )}
                  </span>
                  {!cat.is_default && (
                    <button
                      type="button"
                      onClick={() => handleDeleteClick(cat.id)}
                      className="text-xs text-danger hover:underline"
                    >
                      删除
                    </button>
                  )}
                </div>
              );
            })}
          </div>
          {/* Add category input */}
          <div className="flex items-center gap-2 mt-3">
            <Input
              value={newCategoryName}
              onChange={(e) => setNewCategoryName(e.target.value)}
              placeholder="新分类名称"
              className="flex-1 h-8 text-sm"
              onKeyDown={(e) => {
                if (e.key === "Enter") handleAddCategory();
              }}
            />
            <Button size="sm" variant="ghost" onClick={handleAddCategory}>
              添加
            </Button>
          </div>
        </div>

        {/* About */}
        <div>
          <h3 className="text-sm font-medium mb-3">关于</h3>
          <div className="text-sm text-muted-foreground space-y-1">
            <p>KeyPilot V0.1</p>
            <p className="text-xs">轻量级 API 凭证管理</p>
            <a
              href="https://github.com/keypilot/keypilot"
              target="_blank"
              rel="noopener noreferrer"
              className="text-primary hover:underline"
            >
              GitHub
            </a>
          </div>
        </div>
      </div>

      {/* Delete category confirm dialog */}
      <ConfirmDialog
        open={confirmDeleteOpen}
        onClose={() => setConfirmDeleteOpen(false)}
        onConfirm={handleConfirmDelete}
        title="删除分类"
        message={
          deleteCategoryFor !== null ? (
            <div className="space-y-2">
              <p>删除后，该分类下的 {providersInCategory(deleteCategoryFor)} 个凭证将被迁移到：</p>
              <select
                value={migrateTo}
                onChange={(e) => handleMigrateTargetChange(Number(e.target.value))}
                className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm"
              >
                {categories
                  .filter((c) => c.id !== deleteCategoryFor)
                  .map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
              </select>
            </div>
          ) : (
            "确定要删除此分类吗？"
          )
        }
        confirmText="删除"
        variant="destructive"
      />
    </Modal>
  );
});
