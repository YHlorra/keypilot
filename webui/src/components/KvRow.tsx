import * as React from "react";
import { useState, useEffect, useCallback, useRef } from "react";
import { cn } from "@/lib/utils";
import { Icon, useToast } from "./Icon";
import { CopyButton } from "./CopyButton";
import type { Visibility, ProviderField } from "@/types/api";

interface KvRowProps {
  field: ProviderField;
  onUpdate: (key: string, value: string, visibility: Visibility) => void;
  onDelete: () => void;
}

export const KvRow = React.memo(function KvRow({ field, onUpdate, onDelete }: KvRowProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [editKey, setEditKey] = useState(field.key);
  const [editValue, setEditValue] = useState(field.value);
  const [editVisibility, setEditVisibility] = useState<Visibility>(field.visibility);
  const [revealed, setRevealed] = useState(false);
  const { showToast } = useToast();
  const revealTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  
  const startRevealTimer = useCallback(() => {
    if (revealTimerRef.current) clearTimeout(revealTimerRef.current);
    revealTimerRef.current = setTimeout(() => {
      setRevealed(false);
    }, 3000);
  }, []);

  const handleRevealToggle = useCallback(() => {
    if (field.visibility === "masked") {
      if (revealed) {
        setRevealed(false);
        if (revealTimerRef.current) clearTimeout(revealTimerRef.current);
      } else {
        setRevealed(true);
        startRevealTimer();
      }
    }
  }, [field.visibility, revealed, startRevealTimer]);

  useEffect(() => {
    return () => {
      if (revealTimerRef.current) clearTimeout(revealTimerRef.current);
    };
  }, []);

  const handleEditStart = useCallback(() => {
    setIsEditing(true);
    setEditKey(field.key);
    setEditValue(field.value);
    setEditVisibility(field.visibility);
  }, [field.key, field.value, field.visibility]);

  const handleEditSave = useCallback(() => {
    if (editKey.trim() && editValue.trim()) {
      onUpdate(editKey.trim(), editValue.trim(), editVisibility);
      setIsEditing(false);
    } else {
      showToast("键和值不能为空", "error");
    }
  }, [editKey, editValue, editVisibility, onUpdate, showToast]);

  const handleEditCancel = useCallback(() => {
    setIsEditing(false);
    setEditKey(field.key);
    setEditValue(field.value);
    setEditVisibility(field.visibility);
  }, [field.key, field.value, field.visibility]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") handleEditSave();
      if (e.key === "Escape") handleEditCancel();
    },
    [handleEditSave, handleEditCancel]
  );

  useEffect(() => {
    if (isEditing && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isEditing]);

  const maskedValue = "••••••••";

  return (
    <div className="group flex items-center gap-2 py-2 px-3 rounded-md hover:bg-[color-mix(in_srgb,var(--color-accent)_50%,transparent)] transition-colors">
      {}
      <div className="w-[30%] flex-shrink-0">
        {isEditing ? (
          <input
            ref={inputRef}
            type="text"
            value={editKey}
            onChange={(e) => setEditKey(e.target.value)}
            onKeyDown={handleKeyDown}
            className="w-full h-8 px-2 rounded border border-input bg-background text-sm font-mono focus:outline-none focus:ring-2 focus:ring-ring"
          />
        ) : (
          <span className="text-sm font-mono" style={{ color: "var(--color-primary)" }}>{field.key}</span>
        )}
      </div>

      {}
      <div className="flex-1 min-w-0 flex items-center gap-1">
        {isEditing ? (
          <input
            type="text"
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            onKeyDown={handleKeyDown}
            className="flex-1 h-8 px-2 rounded border border-input bg-background text-sm font-mono focus:outline-none focus:ring-2 focus:ring-ring"
          />
        ) : (
          <>
            <span
              className={cn(
                "flex-1 text-sm font-mono truncate",
                field.visibility === "masked" && !revealed ? "text-muted-foreground" : "text-foreground"
              )}
            >
              {field.visibility === "masked" && !revealed ? maskedValue : field.value}
            </span>
            {field.visibility === "masked" && (
              <button
                type="button"
                onClick={handleRevealToggle}
                className="flex-shrink-0 p-1 rounded hover:bg-accent transition-colors"
                title={revealed ? "隐藏" : "显示"}
              >
                <Icon name={revealed ? "eyeOff" : "eye"} className="w-3.5 h-3.5" />
              </button>
            )}
          </>
        )}
      </div>

      {}
      <div className="flex-shrink-0 flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
        {isEditing ? (
          <>
            <button type="button" onClick={handleEditSave} className="p-1 rounded hover:bg-[color-mix(in_srgb,var(--color-success)_20%,transparent)] text-success" title="保存">
              <Icon name="check" className="w-3.5 h-3.5" />
            </button>
            <button type="button" onClick={handleEditCancel} className="p-1 rounded hover:bg-[color-mix(in_srgb,var(--color-destructive)_20%,transparent)] text-destructive" title="取消">
              <Icon name="x" className="w-3.5 h-3.5" />
            </button>
          </>
        ) : (
          <>
            <CopyButton value={field.value} visibility={field.visibility} revealed={revealed} />
            <button
              type="button"
              onClick={handleEditStart}
              className="p-1 rounded hover:bg-accent"
              title="编辑"
            >
              <Icon name="pencil" className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              onClick={onDelete}
className="p-1 rounded hover:bg-[color-mix(in_srgb,var(--color-destructive)_20%,transparent)] text-destructive"
              title="删除"
            >
              <Icon name="trash" className="w-3.5 h-3.5" />
            </button>
          </>
        )}
      </div>
    </div>
  );
});
