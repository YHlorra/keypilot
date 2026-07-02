import * as React from "react";
import { createContext, useContext, useState, useCallback } from "react";
import { Eye, EyeOff, Copy, Trash2, Plus, RefreshCw, Check, X, Loader2, Search, Pencil, KeyRound } from "lucide-react";
import { cn } from "@/lib/utils";

// Provider preset colors (Option A: teal/indigo/orange/gray/cyan)
export const PRESET_COLORS: Record<string, string> = {
  openai: "#00a2a2",     // teal
  deepseek: "#3e63dd",   // indigo
  anthropic: "#f76808", // orange
  github: "#8e8e8e",     // gray
};

export const PRESET_LABELS: Record<string, string> = {
  openai: "OpenAI",
  deepseek: "DeepSeek",
  anthropic: "Anthropic",
  github: "GitHub",
};

interface IconProps {
  name: string;
  className?: string;
  color?: string;
}

export const Icon = React.memo(function Icon({ name, className, color }: IconProps) {
  const iconMap: Record<string, React.ReactNode> = {
    eye: <Eye className={cn("w-4 h-4", className)} style={{ color }} />,
    eyeOff: <EyeOff className={cn("w-4 h-4", className)} style={{ color }} />,
    copy: <Copy className={cn("w-4 h-4", className)} style={{ color }} />,
    trash: <Trash2 className={cn("w-4 h-4", className)} style={{ color }} />,
    plus: <Plus className={cn("w-4 h-4", className)} style={{ color }} />,
    refresh: <RefreshCw className={cn("w-4 h-4", className)} style={{ color }} />,
    check: <Check className={cn("w-4 h-4", className)} style={{ color }} />,
    x: <X className={cn("w-4 h-4", className)} style={{ color }} />,
    loader: <Loader2 className={cn("w-4 h-4 animate-spin", className)} style={{ color }} />,
    search: <Search className={cn("w-4 h-4", className)} style={{ color }} />,
    pencil: <Pencil className={cn("w-4 h-4", className)} style={{ color }} />,
    keyRound: <KeyRound className={cn("w-4 h-4", className)} style={{ color }} />,
  };

  return <>{iconMap[name] || null}</>;
});

interface ProviderIconProps {
  preset: string | null;
  name: string;
  className?: string;
}

export const ProviderIcon = React.memo(function ProviderIcon({ preset, name, className }: ProviderIconProps) {
  const color = preset ? PRESET_COLORS[preset] : "#8e8e8e";
  // Custom preset names typed in AddCredentialModal land as truthy strings not in
  // PRESET_LABELS (e.g. "openrouter"). Fall back to `name` so we never call .charAt
  // on undefined — mirrors the guarded pattern in ProviderCard.getFamilyTint.
  const label = (preset && PRESET_LABELS[preset]) || name;

  return (
    <span
      className={cn("inline-flex items-center justify-center w-6 h-6 rounded text-xs font-bold text-[var(--color-secondary)]", className)}
      style={{ backgroundColor: color }}
      title={label}
    >
      {label.charAt(0).toUpperCase()}
    </span>
  );
});

// Toast context
interface ToastItem {
  id: string;
  message: string;
  variant?: "default" | "success" | "error";
}

interface ToastContextValue {
  showToast: (message: string, variant?: "default" | "success" | "error") => void;
}

const ToastContext = createContext<ToastContextValue>({ showToast: () => {} });

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  const showToast = useCallback((message: string, variant: "default" | "success" | "error" = "default") => {
    const id = Math.random().toString(36).slice(2);
    setToasts((prev) => [...prev, { id, message, variant }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 3000);
  }, []);

  return (
    <ToastContext.Provider value={{ showToast }}>
      {children}
      <div className="fixed bottom-4 right-4 z-[100] flex flex-col gap-2">
        {toasts.map((toast) => (
          <div
            key={toast.id}
            className={cn(
              "animate-in slide-in-from-bottom-2 fade-in duration-200 rounded-md border border-border px-4 py-3 text-sm shadow-lg",
              toast.variant === "success" && "bg-success text-[var(--color-secondary)] border-success",
              toast.variant === "error" && "bg-destructive text-destructive-foreground border-destructive",
              toast.variant !== "success" && toast.variant !== "error" && "bg-popover text-popover-foreground"
            )}
          >
            {toast.message}
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast() {
  return useContext(ToastContext);
}
