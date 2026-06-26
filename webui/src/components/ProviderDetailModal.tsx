import * as React from "react";
import { useState, useCallback, useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { formatDistanceToNow } from "date-fns";
import { Trash2, Eye, EyeOff, Copy, ExternalLink } from "lucide-react";
import { Modal } from "./Modal";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { ConfirmDialog } from "./ConfirmDialog";
import { Icon, useToast, ProviderIcon } from "./Icon";
import { getProvider, updateProvider, deleteProvider } from "@/lib/api";
import type { GetProviderRequest, UpdateProviderRequest } from "@/types/api";

interface ProviderDetailModalProps {
  providerId: number | null;
  onClose: () => void;
  onTest: (id: number) => void;
  onFetchQuota: (id: number) => void;
}

type EditMode = "view" | "edit";

// Mask value: first 3 + ••• + last 3; if ≤6 chars → "••••••"
function maskValue(value: string): string {
  if (value.length <= 6) return "••••••";
  return value.slice(0, 3) + "•••••" + value.slice(-3);
}

export const ProviderDetailModal = React.memo(function ProviderDetailModal({
  providerId,
  onClose,
  onTest,
  onFetchQuota,
}: ProviderDetailModalProps) {
  const queryClient = useQueryClient();
  const { showToast } = useToast();
  const [editMode, setEditMode] = useState<EditMode>("view");
  const [editName, setEditName] = useState("");
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
  const [revealedFields, setRevealedFields] = useState<Set<number>>(new Set());

  // Fetch provider data
  const { data: provider, isLoading } = useQuery({
    queryKey: ["provider", providerId],
    queryFn: () => getProvider({ id: providerId! } as GetProviderRequest),
    enabled: providerId !== null,
  });

  // Update mutation (silent save on modal close)
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

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: () => deleteProvider({ id: providerId! }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["providers"] });
      showToast("Credential deleted", "success");
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
    // Silent save on modal close (no toast)
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

  // Get base_url from fields for display
  const baseUrlField = useMemo(() => {
    if (!provider?.fields) return null;
    return provider.fields.find((f) => f.key === "base_url" && f.visibility === "visible");
  }, [provider?.fields]);

  // Status pill
  const lastTested = (provider as any)?.last_tested ?? null;
  const statusPillText = lastTested
    ? `Tested ${formatDistanceToNow(new Date(lastTested * 1000))} ago`
    : "Not tested";

  // Quota data (stub -- real data comes from quota query)
  const quotaData = (provider as any)?.quota ?? null;

  // Toggle field visibility
  const toggleReveal = useCallback((fieldId: number) => {
    setRevealedFields((prev) => {
      const next = new Set(prev);
      if (next.has(fieldId)) next.delete(fieldId);
      else next.add(fieldId);
      return next;
    });
  }, []);

  // Copy to clipboard
  const handleCopy = useCallback((value: string) => {
    navigator.clipboard.writeText(value).then(() => {
      showToast("已复制", "success");
    });
  }, [showToast]);

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
          <>
            <Button variant="ghost" onClick={onClose}>
              Cancel
            </Button>
            <Button
              variant="ghost"
              onClick={() => {
                if (providerId !== null) onFetchQuota(providerId);
              }}
            >
              Fetch quota
            </Button>
            <Button
              onClick={() => {
                if (providerId !== null) onTest(providerId);
              }}
            >
              Test connection
            </Button>
          </>
        }
      >
        <div className="space-y-6">
          {/* Header */}
          <div className="flex items-start justify-between gap-4">
            <div className="flex items-center gap-3">
              <ProviderIcon
                preset={provider.preset}
                name={provider.name}
                className="w-8 h-8 text-sm"
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

            <div className="flex items-center gap-2">
              {/* Status pill */}
              <span
                className={`text-xs px-2 py-0.5 rounded-full border ${
                  lastTested
                    ? "bg-primary/10 text-primary border-primary/20"
                    : "bg-muted text-muted-foreground border-border"
                }`}
              >
                {statusPillText}
              </span>

              {/* Edit / Cancel toggle */}
              {editMode === "view" ? (
                <Button variant="ghost" size="sm" onClick={handleStartEdit}>
                  Edit
                </Button>
              ) : (
                <Button variant="ghost" size="sm" onClick={handleCancelEdit}>
                  Cancel
                </Button>
              )}

              {/* Trash */}
              <button
                type="button"
                onClick={() => setDeleteConfirmOpen(true)}
                className="p-1.5 rounded hover:bg-danger/10 text-muted-foreground hover:text-danger transition-colors"
                title="Delete"
              >
                <Trash2 className="w-4 h-4" />
              </button>
            </div>
          </div>

          {/* Quota section */}
          <div className="space-y-2">
            <h3 className="text-lg font-semibold font-serif">Quota</h3>
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
                {/* Inline 2px progress bar */}
                <div className="w-full h-0.5 bg-surface rounded-full overflow-hidden">
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

          {/* Credentials section */}
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
                      {/* Key */}
                      <span className="text-sm font-medium text-primary w-[30%] flex-shrink-0 truncate">
                        {field.key}
                      </span>

                      {/* Value */}
                      <span className="flex-1 font-mono text-xs text-foreground truncate">
                        {displayValue}
                      </span>

                      {/* Visibility toggle */}
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

                      {/* Copy */}
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
            </div>
          </div>
        </div>
      </Modal>

      {/* Delete confirm dialog */}
      <ConfirmDialog
        open={deleteConfirmOpen}
        onClose={() => setDeleteConfirmOpen(false)}
        onConfirm={() => deleteMutation.mutate()}
        title="删除凭证"
        message={`确定要删除 "${provider.name}" 吗？此操作无法撤销。`}
        confirmText="删除"
        variant="destructive"
      />
    </>
  );
});