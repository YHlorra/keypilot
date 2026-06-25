import * as React from "react";
import { useState, useCallback } from "react";
import { Modal } from "./Modal";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { useToast } from "./Icon";
import { useProviders } from "@/hooks/useProviders";
import { useCategories } from "@/hooks/useCategories";
import { addProvider } from "@/lib/api";
import type { AddProviderRequest, Visibility } from "@/types/api";

interface AddCredentialModalProps {
  open: boolean;
  onClose: () => void;
  defaultCategoryId?: number;
}

const TEMPLATES = ["blank", "llm", "database"] as const;

const PRESET_DEFAULTS: Record<string, Array<{ key: string; value: string; visibility: Visibility }>> = {
  openai: [
    { key: "api_key", value: "", visibility: "masked" },
    { key: "base_url", value: "https://api.openai.com/v1", visibility: "visible" },
  ],
  deepseek: [
    { key: "api_key", value: "", visibility: "masked" },
    { key: "base_url", value: "https://api.deepseek.com/v1", visibility: "visible" },
  ],
  anthropic: [
    { key: "api_key", value: "", visibility: "masked" },
    { key: "base_url", value: "https://api.anthropic.com", visibility: "visible" },
  ],
  github: [
    { key: "access_token", value: "", visibility: "masked" },
  ],
  postgresql: [
    { key: "connection_string", value: "", visibility: "masked" },
    { key: "host", value: "localhost", visibility: "visible" },
    { key: "port", value: "5432", visibility: "visible" },
  ],
};

export const AddCredentialModal = React.memo(function AddCredentialModal({
  open,
  onClose,
  defaultCategoryId,
}: AddCredentialModalProps) {
  const [name, setName] = useState("");
  const [preset, setPreset] = useState<string | null>(null);
  const [categoryId, setCategoryId] = useState<number>(defaultCategoryId ?? 1);
  const [fields, setFields] = useState<Array<{ key: string; value: string; visibility: Visibility }>>([]);
  const [isSaving, setIsSaving] = useState(false);

  const { refetch: refetchProviders } = useProviders();
  const { data: categories } = useCategories();
  const { showToast } = useToast();

  const handlePresetChange = useCallback((newPreset: string) => {
    setPreset(newPreset);
    if (newPreset && PRESET_DEFAULTS[newPreset]) {
      setFields(PRESET_DEFAULTS[newPreset].map(f => ({ ...f })));
    } else {
      setFields([{ key: "", value: "", visibility: "visible" }]);
    }
  }, []);

  const handleFieldChange = useCallback((index: number, key: string, value: string, visibility: Visibility) => {
    setFields((prev) => {
      const next = [...prev];
      next[index] = { key, value, visibility };
      return next;
    });
  }, []);

  const handleAddField = useCallback(() => {
    setFields((prev) => [...prev, { key: "", value: "", visibility: "visible" }]);
  }, []);

  const handleRemoveField = useCallback((index: number) => {
    setFields((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const handleSave = useCallback(async () => {
    if (!name.trim()) {
      showToast("请输入名称", "error");
      return;
    }
    if (!preset) {
      showToast("请选择类型", "error");
      return;
    }

    setIsSaving(true);
    try {
      const req: AddProviderRequest = {
        name: name.trim(),
        preset,
        category_id: categoryId,
        fields: fields
          .filter((f) => f.key.trim() && f.value.trim())
          .map((f) => ({
            key: f.key.trim(),
            value: f.value.trim(),
            visibility: f.visibility,
            sort_index: 0,
          })),
      };
      await addProvider(req);
      await refetchProviders();
      showToast("添加成功", "success");
      onClose();
      // Reset
      setName("");
      setPreset(null);
      setFields([]);
    } catch (e) {
      showToast("添加失败", "error");
    } finally {
      setIsSaving(false);
    }
  }, [name, preset, categoryId, fields, refetchProviders, showToast, onClose]);

  const handleClose = useCallback(() => {
    setName("");
    setPreset(null);
    setFields([]);
    onClose();
  }, [onClose]);

  return (
    <Modal
      open={open}
      onClose={handleClose}
      title="添加凭证"
      footer={
        <>
          <Button variant="ghost" onClick={handleClose} disabled={isSaving}>
            取消
          </Button>
          <Button onClick={handleSave} disabled={isSaving}>
            {isSaving ? "保存中..." : "保存"}
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        {/* Name */}
        <div>
          <label className="text-sm font-medium mb-1.5 block">名称</label>
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="我的 API Key"
          />
        </div>

        {/* Category */}
        <div>
          <label className="text-sm font-medium mb-1.5 block">分类</label>
          <select
            value={categoryId}
            onChange={(e) => setCategoryId(Number(e.target.value))}
            className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm"
          >
            {categories?.map((cat) => (
              <option key={cat.id} value={cat.id}>
                {cat.name}
              </option>
            ))}
          </select>
        </div>

        {/* Preset */}
        <div>
          <label className="text-sm font-medium mb-1.5 block">类型</label>
          <select
            value={preset ?? ""}
            onChange={(e) => handlePresetChange(e.target.value)}
            className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm"
          >
            <option value="">选择类型...</option>
            {TEMPLATES.map((t) => (
              <option key={t} value={t}>
                {t === "blank" ? "自定义" : t === "llm" ? "大模型 (LLM)" : "数据库"}
              </option>
            ))}
          </select>
        </div>

        {/* Fields */}
        {fields.length > 0 && (
          <div>
            <label className="text-sm font-medium mb-1.5 block">字段</label>
            <div className="space-y-2">
              {fields.map((field, index) => (
                <div key={index} className="flex items-center gap-2">
                  <Input
                    value={field.key}
                    onChange={(e) => handleFieldChange(index, e.target.value, field.value, field.visibility)}
                    placeholder="key"
                    className="flex-1 font-mono text-xs"
                  />
                  <Input
                    value={field.value}
                    onChange={(e) => handleFieldChange(index, field.key, e.target.value, field.visibility)}
                    placeholder="value"
                    className="flex-1 font-mono text-xs"
                  />
                  <select
                    value={field.visibility}
                    onChange={(e) => handleFieldChange(index, field.key, field.value, e.target.value as Visibility)}
                    className="h-9 rounded-md border border-input bg-transparent px-2 text-xs"
                  >
                    <option value="visible">可见</option>
                    <option value="masked">隐藏</option>
                  </select>
                  <button
                    type="button"
                    onClick={() => handleRemoveField(index)}
                    className="p-1 text-danger hover:bg-danger/20 rounded"
                  >
                    <span className="text-xs">×</span>
                  </button>
                </div>
              ))}
            </div>
            <button
              type="button"
              onClick={handleAddField}
              className="mt-2 text-sm text-primary hover:underline"
            >
              + 添加字段
            </button>
          </div>
        )}
      </div>
    </Modal>
  );
});
