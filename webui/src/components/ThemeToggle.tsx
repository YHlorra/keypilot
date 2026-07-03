import * as React from "react";
import { useState, useEffect, useCallback } from "react";
import { useTheme } from "@/hooks/useTheme";
import type { Theme } from "@/types/api";
import { cn } from "@/lib/utils";
import { Moon, Sun, Monitor } from "lucide-react";

const CYCLE: Theme[] = ["light", "dark", "auto"];

const ICONS: Record<Theme, React.ReactNode> = {
  light: <Sun className="h-4 w-4" />,
  dark: <Moon className="h-4 w-4" />,
  auto: <Monitor className="h-4 w-4" />,
};

const LABELS: Record<Theme, string> = {
  light: "Light",
  dark: "Dark",
  auto: "Auto",
};

function resolveTheme(theme: Theme): "light" | "dark" {
  if (theme === "auto") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return theme;
}

function nextTheme(theme: Theme): Theme {
  const idx = CYCLE.indexOf(theme);
  return CYCLE[(idx + 1) % CYCLE.length];
}

interface ThemeToggleProps {
  
  bare?: boolean;
}

export const ThemeToggle = React.memo(function ThemeToggle({ bare = false }: ThemeToggleProps = {}) {
  const { theme, isLoading, setTheme } = useTheme();
  const [activeTheme, setActiveTheme] = useState<Theme>(theme ?? "dark");

  
  useEffect(() => {
    const stored = localStorage.getItem("keypilot.theme");
    if (stored === "light" || stored === "dark" || stored === "auto") {
      setActiveTheme(stored);
      setTheme(stored);
      document.documentElement.setAttribute("data-theme", resolveTheme(stored));
    } else {
      document.documentElement.setAttribute("data-theme", "dark");
    }
  }, []);

  
  useEffect(() => {
    if (theme) setActiveTheme(theme);
  }, [theme]);

  
  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent | MediaQueryList) => {
      if (activeTheme === "auto") {
        document.documentElement.setAttribute("data-theme", e.matches ? "dark" : "light");
      }
    };
    handler(mediaQuery);
    mediaQuery.addEventListener("change", handler);
    return () => mediaQuery.removeEventListener("change", handler);
  }, [activeTheme]);

  const handleCycle = useCallback(() => {
    const newTheme = nextTheme(activeTheme);
    setActiveTheme(newTheme);
    setTheme(newTheme);
    localStorage.setItem("keypilot.theme", newTheme);
    document.documentElement.setAttribute("data-theme", resolveTheme(newTheme));
  }, [activeTheme, setTheme]);

  if (isLoading) {
    return (
      <div
        className={cn(
          "inline-flex items-center justify-center",
          bare ? "h-8 w-8" : "h-9 w-9"
        )}
      >
        <span className="text-muted-foreground text-sm">...</span>
      </div>
    );
  }

  return (
    <button
      type="button"
      onClick={handleCycle}
      title={`Theme: ${LABELS[activeTheme]} -- click to change`}
      aria-label={`Theme: ${LABELS[activeTheme]} -- click to change`}
      className={cn(
        "inline-flex items-center justify-center transition-colors duration-150",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-primary)]",
        bare
          ? "h-8 w-8 rounded-pill text-[var(--color-muted)] hover:text-[var(--color-neutral)] hover:bg-[var(--color-surface-sunken)] focus-visible:ring-offset-1"
          : "rounded-md border border-border bg-background hover:bg-muted p-2"
      )}
    >
      {ICONS[activeTheme]}
    </button>
  );
});
