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

// Two-level cascade: template → preset list
// @see openspec/changes/v0.1-general-credentials/spec.md REQ-PROV-007
type TemplateId = "custom" | "llm" | "dev-tools" | "database";
const CUSTOM_PRESET_ID = "__custom__";

const TEMPLATES: Array<{ id: TemplateId; label: string }> = [
  { id: "custom", label: "自定义" },
  { id: "llm", label: "大模型 (LLM)" },
  { id: "dev-tools", label: "开发工具" },
  { id: "database", label: "数据库" },
];

const PRESETS_BY_TEMPLATE: Record<TemplateId, Array<{ id: string; label: string }>> = {
  custom: [],
  llm: [
    { id: "openai", label: "OpenAI" },
    { id: "deepseek", label: "DeepSeek" },
    { id: "anthropic", label: "Anthropic" },
    { id: CUSTOM_PRESET_ID, label: "自定义..." },
  ],
  "dev-tools": [
    { id: "github", label: "GitHub" },
    { id: CUSTOM_PRESET_ID, label: "自定义..." },
  ],
  database: [
    { id: "postgres", label: "PostgreSQL" },
    { id: CUSTOM_PRESET_ID, label: "自定义..." },
  ],
};

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
  postgres: [
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
  const [template, setTemplate] = useState<TemplateId | null>(null);
  const [preset, setPreset] = useState<string | null>(null);
  const [customPresetName, setCustomPresetName] = useState("");
  const [categoryId, setCategoryId] = useState<number>(defaultCategoryId ?? 1);
  const [fields, setFields] = useState<Array<{ key: string; value: string; visibility: Visibility }>>([]);
  const [isSaving, setIsSaving] = useState(false);

  const { data: providers = [], refetch: refetchProviders } = useProviders();
  const { data: categories } = useCategories();
  const { showToast } = useToast();

  // Presets already used in the currently selected category — exclude from picker.
  // Multiple custom-named presets can coexist, so CUSTOM_PRESET_ID is always available.
  const usedPresetsInCategory = new Set(
    providers
      .filter((p) => p.category_id === categoryId && p.preset != null)
      .map((p) => p.preset as string)
  );

  // Level-1 → Level-2 cascade: changing template resets preset + fields.
  const handleTemplateChange = useCallback((newTemplate: TemplateId) => {
    setTemplate(newTemplate);
    setPreset(null);
    setCustomPresetName("");
    setFields([{ key: "", value: "", visibility: "visible" }]);
  }, []);

  const handlePresetChange = useCallback((newPreset: string) => {
    setPreset(newPreset || null);
    if (newPreset === CUSTOM_PRESET_ID) {
      // User wants a custom preset — keep blank fields, require name input.
      setCustomPresetName("");
      setFields([{ key: "", value: "", visibility: "visible" }]);
    } else if (newPreset && PRESET_DEFAULTS[newPreset]) {
      setCustomPresetName("");
      setFields(PRESET_DEFAULTS[newPreset].map((f) => ({ ...f })));
    } else {
      setCustomPresetName("");
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
    if (!template) {
      showToast("请选择类型", "error");
      return;
    }
    // For non-custom templates, preset is required.
    if (template !== "custom" && !preset) {
      showToast("请选择具体服务", "error");
      return;
    }
    // When user picks "自定义..." in the service picker, they must type a service name.
    // The typed name is used as the `preset` value so the Rust side will fall back to None adapter.
    let effectivePreset: string | null = preset;
    if (preset === CUSTOM_PRESET_ID) {
      if (!customPresetName.trim()) {
        showToast("请输入自定义服务名", "error");
        return;
      }
      effectivePreset = customPresetName.trim().toLowerCase().replace(/\s+/g, "_");
    }

    setIsSaving(true);
    try {
      const req: AddProviderRequest = {
        name: name.trim(),
        preset: effectivePreset,
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
      handleClose();
    } catch (e) {
      showToast("添加失败", "error");
    } finally {
      setIsSaving(false);
    }
  }, [name, template, preset, customPresetName, categoryId, fields, refetchProviders, showToast]);

  const handleClose = useCallback(() => {
    setName("");
    setTemplate(null);
    setPreset(null);
    setCustomPresetName("");
    setFields([]);
    onClose();
  }, [onClose]);

  // Filter out presets that already exist in the current category.
  // Custom preset is always allowed (user may have multiple custom-named providers).
  const allPresetOptions = template ? PRESETS_BY_TEMPLATE[template] : [];
  const presetOptions = allPresetOptions.filter(
    (p) => p.id === CUSTOM_PRESET_ID || !usedPresetsInCategory.has(p.id)
  );
  const showPresetPicker = template !== null && presetOptions.length > 0;
  const allBuiltinsUsed =
    template !== null &&
    allPresetOptions.filter((p) => p.id !== CUSTOM_PRESET_ID).every((p) => usedPresetsInCategory.has(p.id));

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

        {/* Level 1: Template */}
        <div>
          <label className="text-sm font-medium mb-1.5 block">类型</label>
          <select
            value={template ?? ""}
            onChange={(e) => handleTemplateChange(e.target.value as TemplateId)}
            className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm"
          >
            <option value="" hidden>选择类型...</option>
            {TEMPLATES.map((t) => (
              <option key={t.id} value={t.id}>
                {t.label}
              </option>
            ))}
          </select>
        </div>

        {/* Level 2: Preset (depends on template) */}
        {showPresetPicker && (
          <div>
            <label className="text-sm font-medium mb-1.5 block">服务</label>
            <select
              value={preset ?? ""}
              onChange={(e) => handlePresetChange(e.target.value)}
              className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm"
            >
              <option value="" hidden>选择服务...</option>
              {presetOptions.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.label}
                </option>
              ))}
            </select>

            {/* Hint when every built-in preset is already used in this category */}
            {allBuiltinsUsed && (
              <p className="mt-1 text-xs text-muted-foreground">
                该分类下已添加所有预设服务。可使用"自定义..."添加其他服务。
              </p>
            )}

            {/* Custom preset name input — only shown when user picks "自定义..." */}
            {preset === CUSTOM_PRESET_ID && (
              <Input
                value={customPresetName}
                onChange={(e) => setCustomPresetName(e.target.value)}
                placeholder="自定义服务名 (例如: Mistral / Groq / 自建网关)"
                className="mt-2"
              />
            )}
          </div>
        )}

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
