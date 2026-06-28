import * as React from "react";
import { Key, BarChart3, Settings } from "lucide-react";
import { cn } from "@/lib/utils";

export type Page = "credentials" | "usage";

interface LeftRailProps {
  currentPage: Page;
  onPageChange: (page: Page) => void;
  onSettingsClick: () => void;
}

const NAV_ITEMS: { page: Page; label: string; icon: React.ReactNode }[] = [
  { page: "credentials", label: "Credentials", icon: <Key className="h-5 w-5" strokeWidth={1.75} /> },
  { page: "usage", label: "Usage", icon: <BarChart3 className="h-5 w-5" strokeWidth={1.75} /> },
];

export const LeftRail = React.memo(function LeftRail({
  currentPage,
  onPageChange,
  onSettingsClick,
}: LeftRailProps) {
  return (
    <>
      {/* Desktop: vertical rail - fixed so it stays put while content scrolls */}
      <nav
        className="hidden md:flex fixed left-0 top-0 bottom-0 flex-col items-center gap-1 py-3 border-r border-border bg-card z-30"
        style={{ width: 64 }}
        aria-label="Main navigation"
      >
        {NAV_ITEMS.map(({ page, label, icon }) => (
          <button
            key={page}
            type="button"
            onClick={() => onPageChange(page)}
            aria-label={label}
            title={label}
            className={cn(
              "relative flex flex-col items-center justify-center gap-1 rounded-sm transition-colors",
              "w-11 h-11 my-0.5",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1",
              currentPage === page
                ? "text-[var(--color-primary)] bg-[var(--color-surface-sunken)]"
                : "text-[var(--color-muted)] hover:text-[var(--color-neutral)] hover:bg-[var(--color-surface-sunken)]"
            )}
          >
            {icon}
            <span className="text-[10px] font-medium leading-none">{label}</span>
          </button>
        ))}

        {/* Spacer */}
        <div className="flex-1" />

        {/* Settings at bottom */}
        <button
          type="button"
          onClick={onSettingsClick}
          aria-label="Settings"
          title="Settings"
          className={cn(
            "flex flex-col items-center justify-center gap-1 rounded-sm transition-colors",
            "w-11 h-11 my-0.5",
            "text-[var(--color-muted)] hover:text-[var(--color-neutral)] hover:bg-[var(--color-surface-sunken)]",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1"
          )}
        >
          <Settings className="h-5 w-5" strokeWidth={1.75} />
          <span className="text-[10px] font-medium leading-none">Settings</span>
        </button>
      </nav>

      {/* Mobile: bottom tab bar */}
      <nav
        className="md:hidden fixed bottom-0 left-0 right-0 z-40 flex items-center justify-around border-t border-border bg-card"
        style={{ height: 56 }}
        aria-label="Main navigation"
      >
        {NAV_ITEMS.map(({ page, label, icon }) => (
          <button
            key={page}
            type="button"
            onClick={() => onPageChange(page)}
            aria-label={label}
            className={cn(
              "flex flex-col items-center justify-center gap-0.5 flex-1 h-full",
              "transition-colors focus-visible:outline-none",
              currentPage === page
                ? "text-[var(--color-primary)]"
                : "text-[var(--color-muted)]"
            )}
          >
            {icon}
            <span className="text-[10px] font-medium leading-none">{label}</span>
          </button>
        ))}
        <button
          type="button"
          onClick={onSettingsClick}
          aria-label="Settings"
          className={cn(
            "flex flex-col items-center justify-center gap-0.5 flex-1 h-full",
            "text-[var(--color-muted)] transition-colors focus-visible:outline-none"
          )}
        >
          <Settings className="h-5 w-5" strokeWidth={1.75} />
          <span className="text-[10px] font-medium leading-none">Settings</span>
        </button>
      </nav>
    </>
  );
});
