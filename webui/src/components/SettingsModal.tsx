import * as React from "react";
import { useState, useCallback } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Loader2 } from "lucide-react";
import { Modal } from "./Modal";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { ThemeToggle } from "./ThemeToggle";
import { ConfirmDialog } from "./ConfirmDialog";
import { useCategories } from "@/hooks/useCategories";
import { useProviders } from "@/hooks/useProviders";
import { addCategory, deleteCategory } from "@/lib/api";
import { useToast } from "./Icon";

interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
}

/** Detect Tauri runtime -- in plain browser dev (vite), invoke() throws. */
function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI__" in window;
}

/** Format a mutation error so users see a friendly message, not a JS internals leak. */
function formatMutationError(e: unknown, action: string): string {
  if (!isTauriRuntime()) {
    return "无法连接桌面运行时,请在 KeyPilot 应用内操作";
  }
  if (e instanceof Error && e.message) {
    return `${action}失败: ${e.message}`;
  }
  return `${action}失败`;
}

export const SettingsModal = React.memo(function SettingsModal({ open, onClose }: SettingsModalProps) {
  const [newCategoryName, setNewCategoryName] = useState("");
  const [deleteCategoryFor, setDeleteCategoryFor] = useState<number | null>(null);
  const [migrateTo, setMigrateTo] = useState<number>(0);
  const [confirmDeleteOpen, setConfirmDeleteOpen] = useState(false);

  const { data: categories = [] } = useCategories();
  const { data: providers = [] } = useProviders();
  const queryClient = useQueryClient();
  const { showToast } = useToast();

  const addMutation = useMutation({
    mutationFn: (name: string) => addCategory({ name }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["categories"] });
      showToast("分类已添加", "success");
      setNewCategoryName("");
    },
    onError: (e) => {
      showToast(formatMutationError(e, "添加"), "error");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: ({ id, migrateTo }: { id: number; migrateTo: number }) =>
      deleteCategory({ id, migrate_to: migrateTo }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["providers"] });
      queryClient.invalidateQueries({ queryKey: ["categories"] });
      showToast("分类已删除", "success");
      setConfirmDeleteOpen(false);
      setDeleteCategoryFor(null);
    },
    onError: (e) => {
      showToast(formatMutationError(e, "删除"), "error");
    },
  });

  const handleAddCategory = useCallback(() => {
    const name = newCategoryName.trim();
    if (!name) return;
    addMutation.mutate(name);
  }, [newCategoryName, addMutation]);

  const handleDeleteClick = (id: number) => {
    setDeleteCategoryFor(id);
    const others = categories.filter((c) => c.id !== id);
    setMigrateTo(others[0]?.id ?? 0);
    setConfirmDeleteOpen(true);
  };

  const handleConfirmDelete = () => {
    if (deleteCategoryFor === null) return;
    deleteMutation.mutate({ id: deleteCategoryFor, migrateTo });
  };

  const handleMigrateTargetChange = (id: number) => {
    setMigrateTo(id);
  };

  // Providers count per category
  const providersInCategory = (id: number) => providers.filter((p) => p.category_id === id).length;

  const isAdding = addMutation.isPending;
  const isDeleting = deleteMutation.isPending;

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="设置"
      footer={
        <Button variant="outline" onClick={onClose}>
          关闭
        </Button>
      }
    >
      <div className="space-y-6">
        {/* Theme */}
        <div>
          <h3 className="text-lg font-semibold font-serif mb-3">主题</h3>
          <ThemeToggle />
        </div>

        {/* Categories */}
        <div>
          <h3 className="text-lg font-semibold font-serif mb-3">分类</h3>
          <div className="space-y-2">
            {categories.length === 0 && (
              <p className="text-xs text-muted-foreground py-2">暂无分类</p>
            )}
            {categories.map((cat) => {
              const count = providersInCategory(cat.id);
              return (
                <div key={cat.id} className="flex items-center justify-between py-1">
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
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => handleDeleteClick(cat.id)}
                      disabled={isDeleting}
                      className="text-destructive hover:text-destructive"
                    >
                      删除
                    </Button>
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
              className="flex-1 h-9 text-sm"
              disabled={isAdding}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !isAdding) handleAddCategory();
              }}
              data-testid="add-category-input"
            />
            <Button
              size="sm"
              variant="default"
              onClick={handleAddCategory}
              disabled={isAdding || !newCategoryName.trim()}
              data-testid="add-category-btn"
            >
              {isAdding ? (
                <>
                  <Loader2 className="h-3.5 w-3.5 mr-1 animate-spin" />
                  添加中
                </>
              ) : (
                "添加"
              )}
            </Button>
          </div>
        </div>

        {/* About */}
        <div>
          <h3 className="text-lg font-semibold font-serif mb-3">关于</h3>
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
        onClose={() => !isDeleting && setConfirmDeleteOpen(false)}
        onConfirm={handleConfirmDelete}
        title="删除分类"
        message={
          deleteCategoryFor !== null ? (
            <div className="space-y-2">
              <p>删除后，该分类下的 {providersInCategory(deleteCategoryFor)} 个凭证将被迁移到：</p>
              <Select
                value={String(migrateTo)}
                onValueChange={(v) => handleMigrateTargetChange(Number(v))}
                disabled={isDeleting}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {categories
                    .filter((c) => c.id !== deleteCategoryFor)
                    .map((c) => (
                      <SelectItem key={c.id} value={String(c.id)}>
                        {c.name}
                      </SelectItem>
                    ))}
                </SelectContent>
              </Select>
            </div>
          ) : (
            "确定要删除此分类吗？"
          )
        }
        confirmText={isDeleting ? "删除中…" : "删除"}
        variant="destructive"
      />
    </Modal>
  );
});