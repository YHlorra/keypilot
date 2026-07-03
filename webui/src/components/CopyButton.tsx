import * as React from "react";
import { useState, useCallback } from "react";
import { cn } from "@/lib/utils";
import { Icon, useToast } from "./Icon";
import type { Visibility } from "@/types/api";

interface CopyButtonProps {
  value: string;
  visibility: Visibility;
  revealed?: boolean;
  className?: string;
}

export const CopyButton = React.memo(function CopyButton({
  value,
  visibility,
  revealed = false,
  className,
}: CopyButtonProps) {
  const [copied, setCopied] = useState(false);
  const { showToast } = useToast();

  const canCopy = visibility === "visible" || revealed;

  const handleCopy = useCallback(async () => {
    if (!canCopy || !value) return;
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      showToast("已复制", "success");
      setTimeout(() => setCopied(false), 2000);
    } catch {
      showToast("复制失败", "error");
    }
  }, [canCopy, value, showToast]);

  return (
    <button
      type="button"
      onClick={handleCopy}
      disabled={!canCopy}
      className={cn(
        "inline-flex items-center justify-center rounded p-1 transition-colors",
        canCopy ? "hover:bg-accent cursor-pointer" : "opacity-40 cursor-not-allowed",
        className
      )}
      title={canCopy ? "复制" : "点击显示后复制"}
    >
      <Icon name={copied ? "check" : "copy"} className="w-3.5 h-3.5" />
    </button>
  );
});
