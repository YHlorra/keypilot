import * as React from "react";
import { createContext, useContext, useState, useCallback } from "react";
import { Eye, EyeOff, Copy, Trash2, Plus, RefreshCw, Check, X, Loader2, Search, Pencil, KeyRound } from "lucide-react";
import { cn } from "@/lib/utils";

// Provider preset colors (Radix color scale tones, picked for visual variety).
// 20 presets → 20 colors. When adding a new preset, pick an unused
// tone; the fallback color (#8e8e8e) catches unknown presets.
export const PRESET_COLORS: Record<string, string> = {
  // Original 4
  openai: "#10a37f",      // OpenAI green
  anthropic: "#d97706",   // Anthropic orange
  deepseek: "#1d4ed8",    // DeepSeek blue
  github: "#24292e",      // GitHub near-black
  // OpenAI-compatible (each its own brand tone)
  kimi: "#1e40af",        // Moonshot navy
  zhipu: "#0ea5e9",       // Zhipu sky
  qwen: "#ff6a00",        // Alibaba orange
  openrouter: "#7c3aed",  // OpenRouter violet
  groq: "#f97316",        // Groq orange
  mistral: "#dc2626",     // Mistral red
  siliconflow: "#0891b2", // SiliconFlow teal
  together: "#059669",    // Together emerald
  volcengine: "#2563eb",  // Volcengine blue
  stepfun: "#7c2d12",     // Stepfun brown
  cohere: "#be185d",      // Cohere pink
  perplexity: "#22c55e",  // Perplexity green
  // Anthropic-compat variants — same color as their brand (visual consistency)
  "kimi-anthropic": "#1e40af",
  "zhipu-anthropic": "#0ea5e9",
  "deepseek-anthropic": "#1d4ed8",
  "volcengine-anthropic": "#2563eb",
  // MiniMax — 4 nodes share the brand orange (radix-amber)
  minimax: "#f59e0b",
  "minimax-overseas": "#f59e0b",
  "minimax-anthropic": "#f59e0b",
  "minimax-overseas-anthropic": "#f59e0b",
};

export const PRESET_LABELS: Record<string, string> = {
  openai: "OpenAI",
  anthropic: "Anthropic",
  deepseek: "DeepSeek",
  github: "GitHub",
  kimi: "Moonshot Kimi",
  zhipu: "智谱 GLM",
  qwen: "通义千问",
  openrouter: "OpenRouter",
  groq: "Groq",
  mistral: "Mistral AI",
  siliconflow: "硅基流动",
  together: "Together AI",
  volcengine: "火山引擎",
  stepfun: "阶跃星辰",
  cohere: "Cohere",
  perplexity: "Perplexity",
  "kimi-anthropic": "Kimi (Anthropic)",
  "zhipu-anthropic": "GLM (Anthropic)",
  "deepseek-anthropic": "DeepSeek (Anthropic)",
  "volcengine-anthropic": "Volcengine (Anthropic)",
  // MiniMax — compact tooltip variant (full labels in AddCredentialModal picker)
  minimax: "MiniMax",
  "minimax-overseas": "MiniMax",
  "minimax-anthropic": "MiniMax (Anthropic)",
  "minimax-overseas-anthropic": "MiniMax (Anthropic)",
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
  // Optional icon asset (path under /icons/ or absolute URL). When set, renders
  // an <img> instead of the tinted-circle fallback. Custom providers without
  // an icon fall through to the fallback.
  icon?: string | null;
  className?: string;
}

export const ProviderIcon = React.memo(function ProviderIcon({ preset, name, icon, className }: ProviderIconProps) {
  const color = preset ? PRESET_COLORS[preset] : "#8e8e8e";
  // Custom preset names typed in AddCredentialModal land as truthy strings not in
  // PRESET_LABELS (e.g. "openrouter"). Fall back to `name` so we never call .charAt
  // on undefined — mirrors the guarded pattern in ProviderCard.getFamilyTint.
  const label = (preset && PRESET_LABELS[preset]) || name;

  if (icon) {
    return (
      <span
        title={label}
        className={cn("inline-flex items-center justify-center overflow-hidden", className)}
      >
        <img src={icon} alt={name} className="w-full h-full object-contain" />
      </span>
    );
  }

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
