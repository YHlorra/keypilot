import * as React from "react";
import { useState, useCallback, useMemo } from "react";
import { Modal } from "./Modal";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { PresetCombobox } from "./PresetCombobox";
import { useToast } from "./Icon";
import { useProviders } from "@/hooks/useProviders";
import { useCategories } from "@/hooks/useCategories";
import { useQuery } from "@tanstack/react-query";
import { addProvider, listCatalogPresets } from "@/lib/api";
import type { AddProviderRequest, CatalogPresetMeta, Visibility } from "@/types/api";

interface AddCredentialModalProps {
  open: boolean;
  onClose: () => void;
  defaultCategoryId?: number;
}

type TemplateId = "custom" | "llm" | "dev-tools";
const CUSTOM_PRESET_ID = "__custom__";

const TEMPLATES: Array<{ id: TemplateId; label: string }> = [
  { id: "custom", label: "自定义" },
  { id: "llm", label: "大模型 (LLM)" },
  { id: "dev-tools", label: "开发工具" },
];

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

  // Dynamic catalog presets — single source of truth from backend
  const { data: catalogPresets = [] } = useQuery({
    queryKey: ["catalogPresets"],
    queryFn: listCatalogPresets,
    staleTime: Infinity, // catalog is compiled in, never changes mid-session
  });

  // Derive templates from catalog: protocol === "github" → dev-tools, else → llm
  const presetsByTemplate = useMemo(() => {
    const llm: CatalogPresetMeta[] = [];
    const devTools: CatalogPresetMeta[] = [];
    for (const p of catalogPresets) {
      if (p.protocol === "github") {
        devTools.push(p);
      } else {
        llm.push(p);
      }
    }
    return { llm, devTools };
  }, [catalogPresets]);

  const presetOptionsByTemplate: Record<TemplateId, CatalogPresetMeta[]> = {
    custom: [],
    llm: presetsByTemplate.llm,
    "dev-tools": presetsByTemplate.devTools,
  };

  const usedPresetsInCategory = useMemo(() => {
    return new Set(
      providers
        .filter((p) => p.category_id === categoryId && p.preset != null)
        .map((p) => p.preset as string)
    );
  }, [providers, categoryId]);

  const presetMeta = useMemo(
    () => catalogPresets.find((p) => p.id === preset) ?? null,
    [catalogPresets, preset]
  );

  const handleTemplateChange = useCallback((newTemplate: TemplateId) => {
    setTemplate(newTemplate);
    setPreset(null);
    setCustomPresetName("");
    setFields([{ key: "", value: "", visibility: "visible" }]);
  }, []);

  const handlePresetChange = useCallback((newPreset: string) => {
    setPreset(newPreset || null);
    if (newPreset === CUSTOM_PRESET_ID) {
      setCustomPresetName("");
      setFields([{ key: "", value: "", visibility: "visible" }]);
    } else if (newPreset && presetMeta) {
      setCustomPresetName("");
      const keyField = presetMeta.key_field || "api_key";
      const baseUrl = presetMeta.default_base_url;
      // V0.2.1 multi-endpoint catalog: presets with extras share one api_key
      // across primary + secondary protocols (e.g. DeepSeek OpenAI + Anthropic).
      // Auto-fill both URLs so user sees the dual-protocol shape immediately.
      const extras = presetMeta.extras ?? [];
      const extraFields = extras.map((e) => ({
        key: `${e.protocol}_base_url`,
        value: e.base_url,
        visibility: "visible" as Visibility,
      }));
      setFields([
        { key: keyField, value: "", visibility: "masked" as Visibility },
        ...(baseUrl ? [{ key: "base_url", value: baseUrl, visibility: "visible" as Visibility }] : []),
        ...extraFields,
      ]);
    } else {
      setCustomPresetName("");
      setFields([{ key: "", value: "", visibility: "visible" }]);
    }
  }, [presetMeta]);

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

  const handleClose = useCallback(() => {
    setName("");
    setTemplate(null);
    setPreset(null);
    setCustomPresetName("");
    setFields([]);
    onClose();
  }, [onClose]);

  const handleSave = useCallback(async () => {
    if (!name.trim()) {
      showToast("请输入名称", "error");
      return;
    }
    if (!template) {
      showToast("请选择类型", "error");
      return;
    }
    if (template !== "custom" && !preset) {
      showToast("请选择具体服务", "error");
      return;
    }

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
  }, [name, template, preset, customPresetName, categoryId, fields, refetchProviders, showToast, handleClose]);

  const allPresetOptions = template ? presetOptionsByTemplate[template] : [];
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
        <div>
          <label className="text-sm font-medium mb-1.5 block">名称</label>
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="我的 API Key"
          />
        </div>

        <div>
          <label className="text-sm font-medium mb-1.5 block">分类</label>
          <Select value={String(categoryId)} onValueChange={(v) => setCategoryId(Number(v))}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {categories?.map((cat) => (
                <SelectItem key={cat.id} value={String(cat.id)}>
                  {cat.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div>
          <label className="text-sm font-medium mb-1.5 block">类型</label>
          <Select value={template ?? ""} onValueChange={(v) => handleTemplateChange(v as TemplateId)}>
            <SelectTrigger>
              <SelectValue placeholder="选择类型..." />
            </SelectTrigger>
            <SelectContent>
              {TEMPLATES.map((t) => (
                <SelectItem key={t.id} value={t.id}>
                  {t.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {showPresetPicker && (
          <div>
            <label className="text-sm font-medium mb-1.5 block">服务</label>
            <PresetCombobox
              options={presetOptions}
              value={preset}
              onValueChange={handlePresetChange}
              placeholder="选择服务..."
              allBuiltinsUsed={allBuiltinsUsed}
            />

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

        {fields.length > 0 && (
          <div>
            <label className="text-sm font-medium mb-1.5 block">字段</label>
            <div className="space-y-2">
              {fields.map((field, index) => (
                <div key={index} className="space-y-1.5">
                  <div className="flex items-center gap-2">
                    <Input
                      value={field.key}
                      onChange={(e) => handleFieldChange(index, e.target.value, field.value, field.visibility)}
                      placeholder="key"
                      className="min-w-0 flex-1 font-mono text-xs"
                    />
                    <Select
                      value={field.visibility}
                      onValueChange={(v) => handleFieldChange(index, field.key, field.value, v as Visibility)}
                    >
                      <SelectTrigger className="h-9 w-auto min-w-[5rem] px-2 text-xs">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="visible">可见</SelectItem>
                        <SelectItem value="masked">隐藏</SelectItem>
                      </SelectContent>
                    </Select>
                    <button
                      type="button"
                      onClick={() => handleRemoveField(index)}
                      className="p-1 text-destructive hover:bg-[color-mix(in_srgb,var(--color-destructive)_20%,transparent)] rounded"
                    >
                      <span className="text-xs">×</span>
                    </button>
                  </div>
                  <Input
                    value={field.value}
                    onChange={(e) => handleFieldChange(index, field.key, e.target.value, field.visibility)}
                    placeholder="value"
                    title={field.value}
                    className="w-full h-9 font-mono text-xs"
                  />
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
