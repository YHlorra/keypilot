import * as React from "react";
import { useState, useCallback } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Modal } from "./Modal";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { useToast, Icon } from "./Icon";
import { setManualQuota } from "@/lib/api";
import type { QuotaSnapshot } from "@/types/api";

interface ManualQuotaModalProps {
  isOpen: boolean;
  onClose: () => void;
  providerId: number;
  providerName: string;
}

// Unit options per REQ-QUOTA-001~006
const UNIT_OPTIONS = [
  { value: "USD", label: "USD (美元)" },
  { value: "CNY", label: "CNY (人民币)" },
  { value: "token", label: "Token (令牌)" },
  { value: "GB", label: "GB (存储)" },
  { value: "req", label: "Req (请求次数)" },
] as const;

type UnitValue = (typeof UNIT_OPTIONS)[number]["value"];
type LevelValue = "green" | "amber" | "red" | "ruby";

const LEVEL_OPTIONS: Array<{ value: LevelValue; label: string }> = [
  { value: "green", label: "绿色 (充足)" },
  { value: "amber", label: "琥珀色 (预警)" },
  { value: "red", label: "红色 (危险)" },
  { value: "ruby", label: "红宝石 (紧急)" },
];

function formatDateInputValue(timestamp: number | undefined): string {
  if (!timestamp) return "";
  // Convert unix epoch seconds to YYYY-MM-DD
  const d = new Date(timestamp * 1000);
  return d.toISOString().split("T")[0];
}

function parseDateInputValue(dateStr: string): number | undefined {
  if (!dateStr) return undefined;
  const d = new Date(dateStr);
  if (isNaN(d.getTime())) return undefined;
  return Math.floor(d.getTime() / 1000);
}

// Mock local storage key for last saved quota per provider
function getLastQuotaKey(providerId: number): string {
  return `keypilot_manual_quota_${providerId}`;
}

function loadLastQuota(providerId: number): Partial<QuotaSnapshot> | null {
  try {
    const raw = localStorage.getItem(getLastQuotaKey(providerId));
    if (!raw) return null;
    return JSON.parse(raw) as Partial<QuotaSnapshot>;
  } catch {
    return null;
  }
}

function saveLastQuota(providerId: number, quota: Partial<QuotaSnapshot>): void {
  try {
    localStorage.setItem(getLastQuotaKey(providerId), JSON.stringify(quota));
  } catch {
    // Silently ignore storage errors
  }
}

export const ManualQuotaModal = React.memo(function ManualQuotaModal({
  isOpen,
  onClose,
  providerId,
  providerName,
}: ManualQuotaModalProps) {
  const queryClient = useQueryClient();
  const { showToast } = useToast();

  // Load last saved quota for pre-fill
  const lastQuota = React.useMemo(() => loadLastQuota(providerId), [providerId]);

  const [unit, setUnit] = useState<UnitValue>(() => (lastQuota?.unit as UnitValue) ?? "token");
  const [used, setUsed] = useState<string>(() => lastQuota?.used?.toString() ?? "");
  const [total, setTotal] = useState<string>(
    () => (lastQuota?.total != null ? lastQuota.total.toString() : "")
  );
  const [remaining, setRemaining] = useState<string>(
    () => (lastQuota?.remaining?.toString() ?? "")
  );
  const [level, setLevel] = useState<LevelValue>(() =>
    (lastQuota?.level as LevelValue) ?? "amber"
  );
  const [resetAt, setResetAt] = useState<string>(() => formatDateInputValue(lastQuota?.reset_at));

  const [isSaving, setIsSaving] = useState(false);

  // Invalidate quota cache after save
  const handleSaveSuccess = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: ["quota", providerId] });
    showToast("额度已保存", "success");
    onClose();
  }, [queryClient, providerId, showToast, onClose]);

  const saveMutation = useMutation({
    mutationFn: async (snapshot: QuotaSnapshot) => {
      // Store in localStorage for persistence
      saveLastQuota(providerId, snapshot);
      // Call setManualQuota IPC (V0.1: localStorage-only, V0.1.1+: real IPC)
      await setManualQuota({ id: providerId, snapshot });
    },
    onSuccess: handleSaveSuccess,
    onError: () => {
      showToast("保存失败", "error");
    },
  });

  const handleSave = useCallback(async () => {
    const usedNum = parseFloat(used);
    if (isNaN(usedNum) || usedNum < 0) {
      showToast("请输入有效的已用量", "error");
      return;
    }

    const totalNum = total ? parseFloat(total) : null;
    if (total && (isNaN(totalNum!) || totalNum! < 0)) {
      showToast("请输入有效的总量", "error");
      return;
    }

    const remainingNum = remaining ? parseFloat(remaining) : null;
    if (remaining && (isNaN(remainingNum!) || remainingNum! < 0)) {
      showToast("请输入有效的剩余量", "error");
      return;
    }

    // Compute remaining from total - used if not provided
    const computedRemaining =
      remainingNum ?? (totalNum !== null ? Math.max(0, totalNum - usedNum) : null);

    const snapshot: QuotaSnapshot = {
      total: totalNum,
      used: usedNum,
      remaining: computedRemaining ?? undefined,
      unit,
      level,
      reset_at: parseDateInputValue(resetAt),
    };

    setIsSaving(true);
    try {
      await saveMutation.mutateAsync(snapshot);
    } finally {
      setIsSaving(false);
    }
  }, [used, total, remaining, unit, level, resetAt, saveMutation, showToast]);

  const handleClose = useCallback(() => {
    // Reset form to last saved state
    const last = loadLastQuota(providerId);
    setUnit((last?.unit as UnitValue) ?? "token");
    setUsed(last?.used?.toString() ?? "");
    setTotal(last?.total != null ? last.total.toString() : "");
    setRemaining(last?.remaining?.toString() ?? "");
    setLevel((last?.level as LevelValue) ?? "amber");
    setResetAt(formatDateInputValue(last?.reset_at));
    onClose();
  }, [providerId, onClose]);

  return (
    <Modal
      open={isOpen}
      onClose={handleClose}
      title={`手动输入额度 — ${providerName}`}
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
        {/* Help text */}
        <div className="flex items-start gap-2 p-3 rounded-md bg-muted text-sm text-muted-foreground">
          <Icon name="x" className="w-4 h-4 mt-0.5 flex-shrink-0 opacity-50" />
          <span>Anthropic 不提供额度查询 API。请手动输入当前用量。</span>
        </div>

        {/* Unit */}
        <div>
          <label className="text-sm font-medium mb-1.5 block">单位</label>
          <Select value={unit} onValueChange={(v) => setUnit(v as UnitValue)}>
            <SelectTrigger>
              <SelectValue placeholder="选择单位" />
            </SelectTrigger>
            <SelectContent>
              {UNIT_OPTIONS.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* Used */}
        <div>
          <label className="text-sm font-medium mb-1.5 block">
            已用量 <span className="text-danger">*</span>
          </label>
          <Input
            type="number"
            min="0"
            step="any"
            value={used}
            onChange={(e) => setUsed(e.target.value)}
            placeholder="例如: 125.50"
          />
        </div>

        {/* Total (optional) */}
        <div>
          <label className="text-sm font-medium mb-1.5 block">
            总量 <span className="text-xs text-muted-foreground">(可选)</span>
          </label>
          <Input
            type="number"
            min="0"
            step="any"
            value={total}
            onChange={(e) => setTotal(e.target.value)}
            placeholder="例如: 500.00 (PostgreSQL 等可不填)"
          />
        </div>

        {/* Remaining (optional) */}
        <div>
          <label className="text-sm font-medium mb-1.5 block">
            剩余 <span className="text-xs text-muted-foreground">(可选，留空则自动计算)</span>
          </label>
          <Input
            type="number"
            min="0"
            step="any"
            value={remaining}
            onChange={(e) => setRemaining(e.target.value)}
            placeholder="自动计算: 总量 - 已用量"
          />
        </div>

        {/* Level */}
        <div>
          <label className="text-sm font-medium mb-1.5 block">状态等级</label>
          <Select
            value={level}
            onValueChange={(v) => setLevel(v as LevelValue)}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {LEVEL_OPTIONS.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* Reset at (optional) */}
        <div>
          <label className="text-sm font-medium mb-1.5 block">
            重置日期 <span className="text-xs text-muted-foreground">(可选)</span>
          </label>
          <Input
            type="date"
            value={resetAt}
            onChange={(e) => setResetAt(e.target.value)}
            placeholder="YYYY-MM-DD"
          />
        </div>
      </div>
    </Modal>
  );
});
