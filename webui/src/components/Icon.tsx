import * as React from "react";
import { createContext, useContext, useState, useCallback } from "react";
import { Eye, EyeOff, Copy, Trash2, Plus, RefreshCw, Check, X, Loader2, Search, Pencil, KeyRound } from "lucide-react";
import { cn } from "@/lib/utils";




export const PRESET_COLORS: Record<string, string> = {
  
  openai: "#10a37f",      
  anthropic: "#d97706",   
  deepseek: "#1d4ed8",    
  github: "#24292e",      
  
  kimi: "#1e40af",        
  zhipu: "#0ea5e9",       
  qwen: "#ff6a00",        
  openrouter: "#7c3aed",  
  groq: "#f97316",        
  mistral: "#dc2626",     
  siliconflow: "#0891b2", 
  together: "#059669",    
  volcengine: "#2563eb",  
  stepfun: "#7c2d12",     
  cohere: "#be185d",      
  perplexity: "#22c55e",  
  
  "kimi-anthropic": "#1e40af",
  "zhipu-anthropic": "#0ea5e9",
  "deepseek-anthropic": "#1d4ed8",
  "volcengine-anthropic": "#2563eb",
  
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
  
  
  
  icon?: string | null;
  className?: string;
}

export const ProviderIcon = React.memo(function ProviderIcon({ preset, name, icon, className }: ProviderIconProps) {
  const color = preset ? PRESET_COLORS[preset] : "#8e8e8e";
  
  
  
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
