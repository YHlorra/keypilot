import * as React from "react";
import { useState, useCallback, useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Trash2, Eye, EyeOff, Copy, ExternalLink, Terminal, Pencil, X, RefreshCw, Loader2 } from "lucide-react";
import { formatRelative } from "@/lib/format";
import { Modal } from "./Modal";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { ConfirmDialog } from "./ConfirmDialog";
import { Icon, useToast, ProviderIcon } from "./Icon";
import { AddKvModal } from "./AddKvModal";
import { CodingPlanQuotas } from "./CodingPlanQuotas";
import { getProvider, updateProvider, deleteProvider } from "@/lib/api";
import type { GetProviderRequest, UpdateProviderRequest, Visibility, Category } from "@/types/api";
import { isLlmCategory } from "@/lib/utils";

interface ProviderDetailModalProps {
  providerId: number | null;
  categories: Category[];
  onClose: () => void;
  onTest: (id: number) => void;
  onFetchQuota: (id: number) => void;
}

type EditMode = "view" | "edit";


function maskValue(value: string): string {
  if (value.length <= 6) return "••••••";
  return value.slice(0, 3) + "•••••" + value.slice(-3);
}

export const ProviderDetailModal = React.memo(function ProviderDetailModal({
  providerId,
  categories,
  onClose,
  onTest,
  onFetchQuota,
}: ProviderDetailModalProps) {
  const queryClient = useQueryClient();
  const { showToast } = useToast();
  const [editMode, setEditMode] = useState<EditMode>("view");
  const [editName, setEditName] = useState("");
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
  const [addKvOpen, setAddKvOpen] = useState(false);
  const [revealedFields, setRevealedFields] = useState<Set<number>>(new Set());
  const [testPending, setTestPending] = useState(false);

  
  const { data: provider, isLoading } = useQuery({
    queryKey: ["provider", providerId],
    queryFn: () => getProvider({ id: providerId! } as GetProviderRequest),
    enabled: providerId !== null,
  });

  
  const updateMutation = useMutation({
    mutationFn: (req: UpdateProviderRequest) => updateProvider(req),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["provider", providerId] });
      queryClient.invalidateQueries({ queryKey: ["providers"] });
    },
    onError: () => {
      showToast("更新失败", "error");
    },
  });

  
  const deleteMutation = useMutation({
    mutationFn: () => deleteProvider({ id: providerId! }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["providers"] });
      showToast("凭证已删除", "success");
      setDeleteConfirmOpen(false);
      onClose();
    },
    onError: () => {
      showToast("删除失败", "error");
    },
  });

  const handleStartEdit = useCallback(() => {
    if (provider) {
      setEditName(provider.name);
      setEditMode("edit");
    }
  }, [provider]);

  const handleCancelEdit = useCallback(() => {
    setEditMode("view");
    setEditName("");
  }, []);

  const handleClose = useCallback(() => {
    
    if (editMode === "edit" && provider && editName.trim()) {
      updateMutation.mutate({
        id: provider.id,
        name: editName.trim(),
        notes: provider.notes,
      });
    }
    setEditMode("view");
    onClose();
  }, [editMode, provider, editName, updateMutation, onClose]);

  
  const baseUrlField = useMemo(() => {
    if (!provider?.fields) return null;
    return provider.fields.find((f) => f.key === "base_url" && f.visibility === "visible");
  }, [provider?.fields]);

  
  const lastTested = (provider as any)?.last_tested ?? null;
  const statusPillText = lastTested
    ? `Tested ${formatRelative(lastTested * 1000, "bare")} ago`
    : "Not tested";

  
  
  const canTest = !!provider && isLlmCategory(provider.category_id, categories);

  
  const primaryField = useMemo(
    () => provider?.fields.find((f) => f.key === "api_key") ?? provider?.fields[0] ?? null,
    [provider?.fields]
  );

  
  const quotaData = (provider as any)?.quota ?? null;

  
  const toggleReveal = useCallback((fieldId: number) => {
    setRevealedFields((prev) => {
      const next = new Set(prev);
      if (next.has(fieldId)) next.delete(fieldId);
      else next.add(fieldId);
      return next;
    });
  }, []);

  
  const handleCopy = useCallback((value: string) => {
    navigator.clipboard.writeText(value).then(() => {
      showToast("已复制", "success");
    });
  }, [showToast]);

  
  const handleCopyPrimary = useCallback(() => {
    if (!primaryField) return;
    handleCopy(primaryField.value);
  }, [primaryField, handleCopy]);

  
  const handleTestClick = useCallback(async () => {
    if (providerId === null || testPending) return;
    setTestPending(true);
    try {
      await onTest(providerId);
    } finally {
      setTestPending(false);
    }
  }, [providerId, testPending, onTest]);

  
  
  
  
  
  const handleAddKv = useCallback(
    async (key: string, value: string, visibility: Visibility) => {
      if (!provider) return;
      const nextFields = [
        ...provider.fields.map((f) => ({
          key: f.key,
          value: f.value,
          visibility: f.visibility,
          sort_index: f.sort_index,
        })),
        { key, value, visibility, sort_index: provider.fields.length },
      ];
      try {
        await updateMutation.mutateAsync({
          id: provider.id,
          fields: nextFields,
        });
        setRevealedFields(new Set());
        showToast("字段已添加", "success");
        setAddKvOpen(false);
      } catch {
        showToast("添加失败", "error");
      }
    },
    [provider, showToast, updateMutation]
  );

  
  
  
  
  if (!providerId) return null;

  if (isLoading) {
    return (
      <Modal open={true} onClose={onClose}>
        <div className="flex items-center justify-center py-12">
          <Icon name="loader" className="w-6 h-6" />
        </div>
      </Modal>
    );
  }

  if (!provider) {
    return (
      <Modal open={true} onClose={onClose}>
        <div className="text-center py-12 text-muted-foreground">加载失败</div>
      </Modal>
    );
  }

  return (
    <>
      <Modal
        open={true}
        onClose={handleClose}
        footer={
          <Button variant="ghost" onClick={onClose}>
            取消
          </Button>
        }
      >
        <div className="space-y-6">
          {}
          <div className="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-3 sm:gap-4">
            <div className="flex items-center gap-3 min-w-0">
              <ProviderIcon
                preset={provider.preset}
                name={provider.name}
                icon={provider.icon}
                className="w-8 h-8 rounded"
              />
              <div>
                {editMode === "edit" ? (
                  <Input
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    className="text-lg font-semibold h-8 font-serif"
                  />
                ) : (
                  <h2 className="text-lg font-semibold font-serif">{provider.name}</h2>
                )}
                <div className="flex items-center gap-2 mt-0.5">
                  {baseUrlField && (
                    <a
                      href={baseUrlField.value}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-xs text-primary hover:underline flex items-center gap-0.5"
                    >
                      {baseUrlField.value.replace(/^https?:\/\//, "").slice(0, 30)}
                      <ExternalLink className="w-3 h-3" />
                    </a>
                  )}
                </div>
              </div>
            </div>

            <div className="flex items-center gap-1 flex-wrap shrink-0">
              {}
              <span
                className={`text-xs px-2 py-0.5 rounded-full border whitespace-nowrap ${
                  lastTested
                    ? "bg-[color-mix(in_srgb,var(--color-primary)_10%,transparent)] text-primary border-[color-mix(in_srgb,var(--color-primary)_20%,transparent)]"
                    : "bg-secondary text-muted-foreground border-border"
                }`}
              >
                {statusPillText}
              </span>

              {}
              {editMode === "view" && (
                <>
                  {}
                  <button
                    type="button"
                    onClick={handleCopyPrimary}
                    disabled={!primaryField}
                    className="p-1.5 rounded hover:bg-accent text-muted-foreground hover:text-foreground transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                    title="复制"
                  >
                    <Copy className="w-4 h-4" />
                  </button>

                  {}
                  <button
                    type="button"
                    onClick={handleStartEdit}
                    className="p-1.5 rounded hover:bg-accent text-muted-foreground hover:text-foreground transition-colors"
                    title="编辑"
                  >
                    <Pencil className="w-4 h-4" />
                  </button>

                  {}
                  {canTest && (
                    <button
                      type="button"
                      onClick={handleTestClick}
                      disabled={testPending}
                      className="p-1.5 rounded hover:bg-accent text-muted-foreground hover:text-foreground transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                      title="测试连接"
                    >
                      {testPending ? (
                        <Loader2 className="w-4 h-4 animate-spin" />
                      ) : (
                        <Terminal className="w-4 h-4" />
                      )}
                    </button>
                  )}

                  {}
                  <button
                    type="button"
                    onClick={() => setDeleteConfirmOpen(true)}
                    className="p-1.5 rounded hover:bg-[color-mix(in_srgb,var(--color-destructive)_10%,transparent)] text-muted-foreground hover:text-destructive transition-colors"
                    title="删除"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </>
              )}

              {}
              {editMode === "edit" && (
                <button
                  type="button"
                  onClick={handleCancelEdit}
                  className="p-1.5 rounded hover:bg-accent text-muted-foreground hover:text-foreground transition-colors"
                  title="取消"
                >
                  <X className="w-4 h-4" />
                </button>
              )}
            </div>
          </div>

          {}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-semibold font-serif">Quota</h3>
              <button
                type="button"
                onClick={() => {
                  if (providerId !== null) onFetchQuota(providerId);
                }}
                className="p-1 rounded hover:bg-accent text-muted-foreground hover:text-foreground transition-colors"
                title="刷新配额"
              >
                <RefreshCw className="w-3.5 h-3.5" />
              </button>
            </div>
            {quotaData ? (
              <>
                <div className="flex items-center justify-between text-sm">
                  <span className="font-medium">
                    {quotaData.used} of {quotaData.total} used
                  </span>
                  {quotaData.reset_at && (
                    <span className="text-xs text-muted-foreground">
                      Resets {new Date(quotaData.reset_at * 1000).toLocaleDateString()}
                    </span>
                  )}
                </div>
                {}
                <div className="w-full h-0.5 bg-card rounded-full overflow-hidden">
                  <div
                    className="h-full bg-primary rounded-full"
                    style={{
                      width: `${quotaData.total > 0 ? (quotaData.used / quotaData.total) * 100 : 0}%`,
                    }}
                  />
                </div>
                <p className="text-xs text-muted-foreground">
                  Subscription · Tier {quotaData.level ?? "--"}
                </p>
              </>
            ) : (
              <p className="text-sm text-muted-foreground">No quota data</p>
            )}
          </div>

          {}
          <div className="space-y-2">
            <h3 className="text-lg font-semibold font-serif">Coding Plan</h3>
            <CodingPlanQuotas providerId={provider.id} />
          </div>

          {}
          <div className="space-y-2">
            <h3 className="text-lg font-semibold font-serif">Credentials</h3>
            <div
              className="max-h-[calc(100vh-220px)] overflow-y-auto"
              style={{ maxHeight: "calc(100vh - 220px)" }}
            >
              {provider.fields.length === 0 ? (
                <p className="text-sm text-muted-foreground py-4 text-center">暂无字段</p>
              ) : (
                provider.fields.map((field, index) => {
                  const isRevealed = revealedFields.has(field.id);
                  const displayValue =
                    field.visibility === "masked" && !isRevealed
                      ? maskValue(field.value)
                      : field.value;

                  return (
                    <div
                      key={field.id}
                      className={`flex items-center gap-3 py-2 ${
                        index < provider.fields.length - 1 ? "border-b border-border" : ""
                      }`}
                    >
                      {}
                      <span className="text-sm font-medium text-primary w-[30%] flex-shrink-0 truncate">
                        {field.key}
                      </span>

                      {}
                      <span className="flex-1 font-mono text-xs text-foreground truncate">
                        {displayValue}
                      </span>

                      {}
                      {field.visibility === "masked" && (
                        <button
                          type="button"
                          onClick={() => toggleReveal(field.id)}
                          className="p-1 rounded hover:bg-accent transition-colors flex-shrink-0"
                          title={isRevealed ? "Hide" : "Show"}
                        >
                          {isRevealed ? (
                            <EyeOff className="w-3.5 h-3.5" />
                          ) : (
                            <Eye className="w-3.5 h-3.5" />
                          )}
                        </button>
                      )}

                      {}
                      <button
                        type="button"
                        onClick={() => handleCopy(field.value)}
                        className="p-1 rounded hover:bg-accent transition-colors flex-shrink-0"
                        title="Copy"
                      >
                        <Copy className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  );
                })
              )}
              {}
              <button
                type="button"
                data-testid="add-field-btn"
                onClick={() => setAddKvOpen(true)}
                className="mt-2 text-sm text-primary hover:underline"
              >
                + 添加字段
              </button>
            </div>
          </div>
        </div>
      </Modal>

      {}
      <ConfirmDialog
        open={deleteConfirmOpen}
        onClose={() => setDeleteConfirmOpen(false)}
        onConfirm={() => deleteMutation.mutate()}
        title="删除凭证"
        message={`确定要删除 "${provider.name}" 吗？此操作无法撤销。`}
        confirmText="删除"
        variant="destructive"
      />

      {}
      <AddKvModal
        open={addKvOpen}
        onClose={() => setAddKvOpen(false)}
        onAdd={handleAddKv}
      />
    </>
  );
});