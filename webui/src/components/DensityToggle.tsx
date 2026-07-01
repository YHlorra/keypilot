import { LayoutGrid, Square } from "lucide-react";
import { cn } from "@/lib/utils";

export interface DensityToggleProps {
  value: "1" | "2";
  onChange: (v: "1" | "2") => void;
}

export const DensityToggle = ({ value, onChange }: DensityToggleProps) => {
  return (
    <div className="inline-flex items-center rounded-pill border border-border p-0.5 gap-0.5">
      <button
        type="button"
        onClick={() => onChange("1")}
        aria-label="Single column"
        aria-pressed={value === "1"}
        className={cn(
          "inline-flex items-center justify-center w-8 h-8 rounded-pill transition-colors",
          // ponytail: desaturated selected state -- was bg-[var(--color-primary)] which
          // matched the TopRightActions primary "+" button exactly, creating a visual
          // twin that made users perceive the add credential CTA as obscured.
          value === "1"
            ? "bg-[var(--color-surface-sunken)] text-[var(--color-neutral)] shadow-[inset_0_0_0_1px_var(--color-primary)]"
            : "text-[var(--color-muted)] hover:text-[var(--color-neutral)]"
        )}
      >
        <LayoutGrid className="h-4 w-4" />
      </button>
      <button
        type="button"
        onClick={() => onChange("2")}
        aria-label="Two columns"
        aria-pressed={value === "2"}
        className={cn(
          "inline-flex items-center justify-center w-8 h-8 rounded-pill transition-colors",
          value === "2"
            ? "bg-[var(--color-primary)] text-[var(--color-secondary)]"
            : "text-[var(--color-muted)] hover:text-[var(--color-neutral)]"
        )}
      >
        <Square className="h-4 w-4" />
      </button>
    </div>
  );
};