import { Settings } from "lucide-react";
import { ThemeToggle } from "./ThemeToggle";
import { cn } from "@/lib/utils";

interface TitlebarProps {
  onSettingsClick?: () => void;
}

export const Titlebar = ({ onSettingsClick }: TitlebarProps) => {
  return (
    <header
      className="fixed top-0 left-0 right-0 z-50 flex items-center justify-between px-4 border-b border-border bg-card"
      style={{ height: 44 }}
    >
      {/* Left: Serif KeyPilot wordmark */}
      <div className="flex items-center">
        <span
          className="text-[17px] font-medium tracking-[-0.2px] text-[var(--color-neutral)]"
          style={{ fontFamily: "var(--font-serif)" }}
        >
          KeyPilot
        </span>
      </div>

      {/* Right: ThemeToggle + Settings */}
      <div className="flex items-center gap-3">
        <ThemeToggle />
        <button
          type="button"
          onClick={onSettingsClick}
          className={cn(
            "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-pill text-xs font-medium",
            "text-[var(--color-muted)] hover:text-[var(--color-neutral)]",
            "border border-border hover:border-[var(--color-primary)]",
            "transition-colors"
          )}
        >
          <Settings className="h-4 w-4" />
          <span>Settings</span>
        </button>
      </div>
    </header>
  );
};
