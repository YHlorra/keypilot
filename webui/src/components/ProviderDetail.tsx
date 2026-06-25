import { useState, useCallback } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { getProvider, updateProvider, deleteProvider, testConnection } from "@/lib/api";
import type { GetProviderRequest, UpdateProviderRequest, Visibility } from "@/types/api";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { KvRow } from "./KvRow";
import { QuotaBadge } from "./QuotaBadge";
import { AddKvModal } from "./AddKvModal";
import { ConfirmDialog } from "./ConfirmDialog";
import { ManualQuotaModal } from "./ManualQuotaModal";
import { Icon, useToast, ProviderIcon, PRESET_LABELS } from "./Icon";

interface ProviderDetailProps {
  providerId: number | null;
}

export function ProviderDetail({ providerId }: ProviderDetailProps) {
  const queryClient = useQueryClient();
  const { showToast } = useToast();
  const [addKvOpen, setAddKvOpen] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState(false);
  const [manualQuotaOpen, setManualQuotaOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [editName, setEditName] = useState("");
  const [editNotes, setEditNotes] = useState("");

  const { data: provider, isLoading, isError, error } = useQuery({
    queryKey: ["provider", providerId],
    queryFn: () => getProvider({ id: providerId! } as GetProviderRequest),
    enabled: providerId !== null,
  });

  const updateMutation = useMutation({
    mutationFn: (req: UpdateProviderRequest) => updateProvider(req),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["provider", providerId] });
      queryClient.invalidateQueries({ queryKey: ["providers"] });
      showToast("更新成功", "success");
      setIsEditing(false);
    },
    onError: () => {
      showToast("更新失败", "error");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: () => deleteProvider({ id: providerId! }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["providers"] });
      showToast("已删除", "success");
      setDeleteConfirm(false);
    },
    onError: () => {
      showToast("删除失败", "error");
    },
  });

  const testMutation = useMutation({
    mutationFn: () => testConnection({ id: providerId! }),
    onSuccess: () => {
      showToast("连接测试成功", "success");
    },
    onError: () => {
      showToast("连接测试失败", "error");
    },
  });

  const handleStartEdit = useCallback(() => {
    if (provider) {
      setEditName(provider.name);
      setEditNotes(provider.notes ?? "");
      setIsEditing(true);
    }
  }, [provider]);

  const handleSaveEdit = useCallback(() => {
    if (!provider) return;
    updateMutation.mutate({
      id: provider.id,
      name: editName.trim() || provider.name,
      notes: editNotes.trim() || null,
    });
  }, [provider, editName, editNotes, updateMutation]);

  const handleCancelEdit = useCallback(() => {
    setIsEditing(false);
    setEditName("");
    setEditNotes("");
  }, []);

  const handleUpdateField = useCallback(
    (key: string, value: string, visibility: Visibility) => {
      if (!provider) return;
      const newFields = provider.fields.map((f) =>
        f.key === key ? { ...f, value, visibility } : f
      );
      updateMutation.mutate({
        id: provider.id,
        fields: newFields.map(({ key, value, visibility, sort_index }) => ({
          key,
          value,
          visibility,
          sort_index,
        })),
      });
    },
    [provider, updateMutation]
  );

  const handleDeleteField = useCallback(
    (fieldKey: string) => {
      if (!provider) return;
      const newFields = provider.fields.filter((f) => f.key !== fieldKey);
      updateMutation.mutate({
        id: provider.id,
        fields: newFields.map(({ key, value, visibility, sort_index }) => ({
          key,
          value,
          visibility,
          sort_index,
        })),
      });
    },
    [provider, updateMutation]
  );

  const handleAddField = useCallback(
    (key: string, value: string, visibility: Visibility) => {
      if (!provider) return;
      const newFields = [
        ...provider.fields,
        { key, value, visibility, sort_index: provider.fields.length },
      ];
      updateMutation.mutate({
        id: provider.id,
        fields: newFields.map(({ key, value, visibility, sort_index }) => ({
          key,
          value,
          visibility,
          sort_index,
        })),
      });
    },
    [provider, updateMutation]
  );

  if (!providerId) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground">
        <div className="text-center">
          <p className="text-lg mb-2">← 选择凭证</p>
          <p className="text-sm">从左侧选择一个凭证查看详情</p>
        </div>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Icon name="loader" className="w-6 h-6" />
      </div>
    );
  }

  if (isError || !provider) {
    return (
      <div className="flex items-center justify-center h-full text-danger">
        <div className="text-center">
          <p className="text-lg mb-2">加载失败</p>
          <p className="text-sm">{error instanceof Error ? error.message : "未知错误"}</p>
        </div>
      </div>
    );
  }

  const presetLabel = provider.preset ? PRESET_LABELS[provider.preset] : "自定义";

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="px-6 py-4 border-b border-border">
        <div className="flex items-start justify-between gap-4">
          <div className="flex items-center gap-3">
            <ProviderIcon preset={provider.preset} name={provider.name} className="w-8 h-8" />
            <div>
              {isEditing ? (
                <Input
                  value={editName}
                  onChange={(e) => setEditName(e.target.value)}
                  className="text-lg font-semibold h-8"
                />
              ) : (
                <h2 className="text-lg font-semibold">{provider.name}</h2>
              )}
              <p className="text-sm text-muted-foreground">{presetLabel}</p>
            </div>
          </div>

          {/* Actions */}
          <div className="flex items-center gap-2">
            {isEditing ? (
              <>
                <Button size="sm" variant="ghost" onClick={handleCancelEdit}>
                  取消
                </Button>
                <Button size="sm" onClick={handleSaveEdit}>
                  保存
                </Button>
              </>
            ) : (
              <>
                <Button size="sm" variant="ghost" onClick={() => testMutation.mutate()}>
                  测试
                </Button>
                <Button size="sm" variant="ghost" onClick={handleStartEdit}>
                  重命名
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => setDeleteConfirm(true)}
                  className="text-danger hover:text-danger"
                >
                  删除
                </Button>
              </>
            )}
          </div>
        </div>

        {/* Quota badge */}
        <div className="mt-3">
          <QuotaBadge
            providerId={provider.id}
            onOpenManual={() => setManualQuotaOpen(true)}
          />
        </div>
      </div>

      {/* Fields */}
      <div className="flex-1 overflow-y-auto px-6 py-4">
        <div className="mb-4 flex items-center justify-between">
          <h3 className="text-sm font-medium">字段</h3>
          <Button size="sm" variant="ghost" onClick={() => setAddKvOpen(true)}>
            <Icon name="plus" className="w-3.5 h-3.5 mr-1" />
            添加字段
          </Button>
        </div>

        {provider.fields.length === 0 ? (
          <div className="py-8 text-center">
            <p className="text-sm text-muted-foreground mb-3">暂无字段</p>
            <Button size="sm" onClick={() => setAddKvOpen(true)}>
              <Icon name="plus" className="w-3.5 h-3.5 mr-1" />
              添加第一个字段
            </Button>
          </div>
        ) : (
          <div className="border border-border rounded-lg divide-y divide-border">
            {provider.fields.map((field) => (
              <KvRow
                key={field.id}
                field={field}
                onUpdate={handleUpdateField}
                onDelete={() => handleDeleteField(field.key)}
              />
            ))}
          </div>
        )}

        {/* Notes */}
        <div className="mt-6">
          <h3 className="text-sm font-medium mb-2">备注</h3>
          {isEditing ? (
            <textarea
              value={editNotes}
              onChange={(e) => setEditNotes(e.target.value)}
              placeholder="添加备注..."
              className="w-full h-24 px-3 py-2 rounded-md border border-input bg-transparent text-sm resize-none focus:outline-none focus:ring-2 focus:ring-ring"
            />
          ) : (
            <div className="text-sm text-muted-foreground whitespace-pre-wrap">
              {provider.notes || "暂无备注"}
            </div>
          )}
        </div>
      </div>

      {/* Add KV Modal */}
      <AddKvModal open={addKvOpen} onClose={() => setAddKvOpen(false)} onAdd={handleAddField} />

      {/* Delete Confirm */}
      <ConfirmDialog
        open={deleteConfirm}
        onClose={() => setDeleteConfirm(false)}
        onConfirm={() => deleteMutation.mutate()}
        title="删除凭证"
        message={`确定要删除 "${provider.name}" 吗？此操作无法撤销。`}
        confirmText="删除"
        variant="destructive"
      />

      {/* Manual quota entry (Anthropic / quota fetch failure) */}
      <ManualQuotaModal
        isOpen={manualQuotaOpen}
        onClose={() => setManualQuotaOpen(false)}
        providerId={provider.id}
        providerName={provider.name}
      />
    </div>
  );
}
