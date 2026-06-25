import * as React from "react";
import { useState, useEffect, useCallback } from "react";
import { useTheme } from "@/hooks/useTheme";
import type { Theme } from "@/types/api";
import { cn } from "@/lib/utils";
import { Moon, Sun, Monitor } from "lucide-react";

const THEME_OPTIONS: { value: Theme; label: string; icon: React.ReactNode }[] = [
  { value: "light", label: "Light", icon: <Sun className="h-4 w-4" /> },
  { value: "dark", label: "Dark", icon: <Moon className="h-4 w-4" /> },
  { value: "auto", label: "Auto", icon: <Monitor className="h-4 w-4" /> },
];

function resolveTheme(theme: Theme): "light" | "dark" {
  if (theme === "auto") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
  return theme;
}

export const ThemeToggle = React.memo(function ThemeToggle() {
  const { theme, isLoading, setTheme } = useTheme();
  const [activeTheme, setActiveTheme] = useState<Theme>(theme ?? "dark");

  // Initialize from localStorage on mount
  useEffect(() => {
    const stored = localStorage.getItem("keypilot.theme");
    if (stored === "light" || stored === "dark" || stored === "auto") {
      setActiveTheme(stored);
      setTheme(stored);
      document.documentElement.setAttribute("data-theme", resolveTheme(stored));
    } else {
      // Default to dark
      document.documentElement.setAttribute("data-theme", "dark");
    }
  }, []);

  useEffect(() => {
    if (theme) setActiveTheme(theme);
  }, [theme]);

  // Auto mode listener — REQ-UI-004.3
  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent | MediaQueryList) => {
      if (activeTheme === "auto") {
        document.documentElement.setAttribute(
          "data-theme",
          e.matches ? "dark" : "light"
        );
      }
    };

    handler(mediaQuery); // Apply current state
    mediaQuery.addEventListener("change", handler);
    return () => mediaQuery.removeEventListener("change", handler);
  }, [activeTheme]);

  const handleSetTheme = useCallback(
    (newTheme: Theme) => {
      setActiveTheme(newTheme);
      setTheme(newTheme);
      localStorage.setItem("keypilot.theme", newTheme);
      document.documentElement.setAttribute("data-theme", resolveTheme(newTheme));
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
    <div className="inline-flex items-center rounded-pill border border-border bg-muted p-0.5">
      {THEME_OPTIONS.map((option) => (
        <button
          key={option.value}
          type="button"
          onClick={() => handleSetTheme(option.value)}
          className={cn(
            "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-pill text-xs font-medium transition-all",
            activeTheme === option.value
              ? "bg-background text-foreground"
              : "text-muted-foreground hover:text-foreground"
          )}
        >
          {option.icon}
          <span className="hidden sm:inline">{option.label}</span>
        </button>
      ))}
    </div>
  );
});
