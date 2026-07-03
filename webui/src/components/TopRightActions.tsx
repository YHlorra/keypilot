import * as React from "react";
import { Wrench, Plus } from "lucide-react";
import { ThemeToggle } from "./ThemeToggle";
import { cn } from "@/lib/utils";

interface TopRightActionsProps {
  onSettingsClick: () => void;
  onAddClick: () => void;
}








export const TopRightActions = React.memo(function TopRightActions({
  onSettingsClick,
  onAddClick,
}: TopRightActionsProps) {
  return (
    <div className="flex items-center gap-2.5" data-testid="top-right-actions">
      {}
      <div
        className="inline-flex items-center gap-0.5 p-1 rounded-pill"
        style={{ backgroundColor: "var(--color-surface-elevated)" }}
        role="toolbar"
        aria-label="Application actions"
      >
        <PillIconButton
          icon={<Wrench className="h-4 w-4" strokeWidth={1.75} />}
          label="Settings"
          onClick={onSettingsClick}
        />
        <ThemeToggle bare />
      </div>

      {}
      <button
        type="button"
        onClick={onAddClick}
        aria-label="Add credential"
        title="Add credential"
        data-testid="add-credential-btn"
        className={cn(
          "inline-flex items-center justify-center h-10 w-10 rounded-full",
          "bg-[var(--color-primary)] text-[var(--color-secondary)]",
          "transition-all duration-150",
          "hover:scale-[1.06] hover:shadow-[0_4px_14px_rgba(27,54,93,0.25)]",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-primary)] focus-visible:ring-offset-2",
          "active:scale-95"
        )}
      >
        <Plus className="h-5 w-5" strokeWidth={2.5} />
      </button>
    </div>
  );
});

interface PillIconButtonProps {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
  active?: boolean;
}

const PillIconButton = React.memo(function PillIconButton({
  icon,
  label,
  onClick,
  active = false,
}: PillIconButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={label}
      title={label}
      className={cn(
        "inline-flex items-center justify-center h-8 w-8 rounded-pill",
        "transition-colors duration-150",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-primary)] focus-visible:ring-offset-1",
        active
          ? "text-[var(--color-primary)] bg-[var(--color-surface-sunken)]"
          : "text-[var(--color-muted)] hover:text-[var(--color-neutral)] hover:bg-[var(--color-surface-sunken)]"
      )}
    >
      {icon}
    </button>
  );
});
