import * as React from "react";
import { useState, useEffect, useCallback } from "react";
import { useTheme } from "@/hooks/useTheme";
import type { Theme } from "@/types/api";
import { cn } from "@/lib/utils";

const THEME_OPTIONS: { value: Theme; label: string; icon: string }[] = [
  { value: "dark", label: "深色", icon: "🌙" },
  { value: "light", label: "浅色", icon: "☀️" },
  { value: "auto", label: "自动", icon: "💻" },
];

export const ThemeToggle = React.memo(function ThemeToggle() {
  const { theme, isLoading, setTheme } = useTheme();
  const [activeTheme, setActiveTheme] = useState<Theme>(theme ?? "dark");

  useEffect(() => {
    if (theme) setActiveTheme(theme);
  }, [theme]);

  // Auto mode listener
  useEffect(() => {
    if (activeTheme !== "auto") return;

    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent | MediaQueryList) => {
      document.documentElement.dataset.theme = e.matches ? "dark" : "light";
    };

    handler(mediaQuery);
    mediaQuery.addEventListener("change", handler);
    return () => mediaQuery.removeEventListener("change", handler);
  }, [activeTheme]);

  const handleSetTheme = useCallback(
    (newTheme: Theme) => {
      setActiveTheme(newTheme);
      setTheme(newTheme);
      if (newTheme === "auto") {
        const isDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
        document.documentElement.dataset.theme = isDark ? "dark" : "light";
      } else {
        document.documentElement.dataset.theme = newTheme;
      }
    },
    [setTheme]
  );

  if (isLoading) {
    return (
      <div className="flex items-center justify-center w-20 h-8">
        <span className="text-muted-foreground text-sm">...</span>
      </div>
    );
  }

  return (
    <div className="inline-flex items-center rounded-md border border-border bg-muted p-0.5">
      {THEME_OPTIONS.map((option) => (
        <button
          key={option.value}
          type="button"
          onClick={() => handleSetTheme(option.value)}
          className={cn(
            "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-sm text-xs font-medium transition-all",
            activeTheme === option.value
              ? "bg-background text-foreground shadow-sm"
              : "text-muted-foreground hover:text-foreground"
          )}
        >
          <span>{option.icon}</span>
          <span className="hidden sm:inline">{option.label}</span>
        </button>
      ))}
    </div>
  );
});
