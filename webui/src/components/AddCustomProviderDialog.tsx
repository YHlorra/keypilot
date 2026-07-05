import * as React from "react";
import { useState, useCallback } from "react";
import { Eye, EyeOff, Loader2, CheckCircle2, XCircle } from "lucide-react";
import { Modal } from "./Modal";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";


export type ProtocolId = "openai" | "anthropic" | "github" | "balance" | "deepseek";

export interface AddCustomProviderFormData {
  name: string;
  protocol: ProtocolId;
  base_url: string;
  auth_header: string;
  api_key: string;
  notes?: string;
}

export interface PreflightResult {
  ok: boolean;
  message?: string;
}

export interface AddCustomProviderDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSubmit: (data: AddCustomProviderFormData) => void | Promise<void>;
  onPreflight?: (data: AddCustomProviderFormData) => Promise<PreflightResult>;
}

const PROTOCOLS: ReadonlyArray<{ id: ProtocolId; label: string; urlHint: string }> = [
  { id: "openai", label: "OpenAI 兼容", urlHint: "https://api.openai.com/v1" },
  { id: "anthropic", label: "Anthropic 兼容", urlHint: "https://api.anthropic.com" },
  { id: "deepseek", label: "DeepSeek", urlHint: "https://api.deepseek.com" },
  { id: "github", label: "GitHub", urlHint: "https://api.github.com" },
  { id: "balance", label: "仅余额 (通用)", urlHint: "https://example.com" },
];

const DEFAULT_AUTH_HEADER = "Authorization: Bearer {api_key}";

export const AddCustomProviderDialog = React.memo(function AddCustomProviderDialog({
  open,
  onOpenChange,
  onSubmit,
  onPreflight,
}: AddCustomProviderDialogProps) {
  const [name, setName] = useState("");
  const [protocol, setProtocol] = useState<ProtocolId>("openai");
  const [baseUrl, setBaseUrl] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [notes, setNotes] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [preflight, setPreflight] = useState<PreflightResult | null>(null);
  const [errors, setErrors] = useState<Partial<Record<keyof AddCustomProviderFormData, string>>>({});

  const selectedMeta = PROTOCOLS.find((p) => p.id === protocol) ?? PROTOCOLS[0];

  const reset = useCallback(() => {
    setName("");
    setProtocol("openai");
    setBaseUrl("");
    setApiKey("");
    setNotes("");
    setShowKey(false);
    setIsTesting(false);
    setIsSubmitting(false);
    setPreflight(null);
    setErrors({});
  }, []);

  const handleClose = useCallback(() => {
    reset();
    onOpenChange(false);
  }, [reset, onOpenChange]);

  const validate = useCallback((): boolean => {
    const e: typeof errors = {};
    if (!name.trim()) e.name = "请输入名称";
    if (!baseUrl.trim()) e.base_url = "请输入 Base URL";
    else if (!/^https?:\/\//i.test(baseUrl.trim())) e.base_url = "需以 http(s):// 开头";
    if (!apiKey.trim()) e.api_key = "请输入 API Key";
    setErrors(e);
    return Object.keys(e).length === 0;
  }, [name, baseUrl, apiKey]);

  const buildData = useCallback(
    (): AddCustomProviderFormData => ({
      name: name.trim(),
      protocol,
      base_url: baseUrl.trim(),
      auth_header: DEFAULT_AUTH_HEADER,
      api_key: apiKey.trim(),
      ...(notes.trim() ? { notes: notes.trim() } : {}),
    }),
    [name, protocol, baseUrl, apiKey, notes],
  );

  const handleTest = useCallback(async () => {
    if (!validate() || !onPreflight) return;
    setIsTesting(true);
    setPreflight(null);
    try {
      const res = await onPreflight(buildData());
      const ok = !!res?.ok;
      setPreflight({ ok, message: res?.message ?? (ok ? "HTTP 200" : "失败") });
    } catch (e) {
      setPreflight({ ok: false, message: e instanceof Error ? e.message : "测试失败" });
    } finally {
      setIsTesting(false);
    }
  }, [validate, onPreflight, buildData]);

  const handleSubmit = useCallback(async () => {
    if (!validate()) return;
    setIsSubmitting(true);
    try {
      await onSubmit(buildData());
      handleClose();
    } catch {
      // Parent surfaces error; keep dialog open so user can retry.
    } finally {
      setIsSubmitting(false);
    }
  }, [validate, onSubmit, buildData, handleClose]);

  const busy = isTesting || isSubmitting;
  const canTest =
    !!onPreflight && !busy && !!name.trim() && !!baseUrl.trim() && !!apiKey.trim();

  return (
    <Modal
      open={open}
      onClose={handleClose}
      title="添加自定义供应商"
      footer={
        <>
          <Button variant="ghost" onClick={handleClose} disabled={busy}>
            取消
          </Button>
          {onPreflight && (
            <Button
              variant="outline"
              onClick={handleTest}
              disabled={!canTest}
              data-testid="preflight-btn"
            >
              {isTesting ? (
                <>
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  测试中
                </>
              ) : (
                "测试连接"
              )}
            </Button>
          )}
          <Button onClick={handleSubmit} disabled={busy} data-testid="submit-btn">
            {isSubmitting ? (
              <>
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                添加中
              </>
            ) : (
              "添加"
            )}
          </Button>
        </>
      }
    >
      <div className="max-w-md mx-auto space-y-4">
        <p className="text-xs text-muted-foreground">添加一个未在内置列表中的供应商</p>


        <div>
          <label className="text-sm font-medium mb-1.5 block">名称</label>
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="我的代理"
            disabled={busy}
          />
          {errors.name && (
            <p className="mt-1 text-xs text-[var(--color-destructive)]">{errors.name}</p>
          )}
        </div>


        <div>
          <label className="text-sm font-medium mb-1.5 block">协议</label>
          <Select
            value={protocol}
            onValueChange={(v) => setProtocol(v as ProtocolId)}
            disabled={busy}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {PROTOCOLS.map((p) => (
                <SelectItem key={p.id} value={p.id}>
                  {p.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>


        <div>
          <label className="text-sm font-medium mb-1.5 block">API Key</label>
          <div className="relative">
            <Input
              type={showKey ? "text" : "password"}
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="sk-xxxxx"
              autoComplete="off"
              disabled={busy}
              className="pr-10 font-mono"
            />
            <button
              type="button"
              onClick={() => setShowKey((s) => !s)}
              tabIndex={-1}
              aria-label={showKey ? "隐藏 API Key" : "显示 API Key"}
              className="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground hover:bg-accent rounded"
            >
              {showKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
            </button>
          </div>
          {errors.api_key && (
            <p className="mt-1 text-xs text-[var(--color-destructive)]">{errors.api_key}</p>
          )}
        </div>


        <div>
          <label className="text-sm font-medium mb-1.5 block">Base URL</label>
          <Input
            value={baseUrl}
            onChange={(e) => setBaseUrl(e.target.value)}
            placeholder={selectedMeta.urlHint}
            disabled={busy}
            className="font-mono text-xs"
          />
          {errors.base_url && (
            <p className="mt-1 text-xs text-[var(--color-destructive)]">{errors.base_url}</p>
          )}
        </div>


        <div>
          <label className="text-sm font-medium mb-1.5 block">备注</label>
          <textarea
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            placeholder="可选"
            rows={3}
            disabled={busy}
            className="flex w-full rounded-sm border border-border px-3 py-1.5 text-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 disabled:cursor-not-allowed disabled:opacity-50"
            style={{ backgroundColor: "var(--color-surface)" }}
          />
        </div>


        {(isTesting || preflight) && (
          <div
            className={`text-xs flex items-center gap-1.5 ${
              isTesting
                ? "text-muted-foreground"
                : preflight?.ok
                  ? "text-[var(--color-success)]"
                  : "text-[var(--color-destructive)]"
            }`}
            data-testid="preflight-status"
            data-tone={isTesting ? "testing" : preflight?.ok ? "ok" : "fail"}
          >
            {isTesting ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : preflight?.ok ? (
              <CheckCircle2 className="h-3.5 w-3.5" />
            ) : (
              <XCircle className="h-3.5 w-3.5" />
            )}
            <span>
              {isTesting
                ? "测试中…"
                : preflight?.ok
                  ? `✓ 连接成功 (${preflight.message ?? "HTTP 200"})`
                  : `✗ 连接失败: ${preflight?.message ?? "未知错误"}`}
            </span>
          </div>
        )}
      </div>
    </Modal>
  );
});
