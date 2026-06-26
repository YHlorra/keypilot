import { cn } from "@/lib/utils";

export interface ChipGroupProps {
  options?: { value: string; label: string }[];
  value: string;
  onChange: (v: string) => void;
}

const PRESET_OPTIONS: { value: string; label: string }[] = [
  { value: "all", label: "All" },
  { value: "ai", label: "AI" },
  { value: "databases", label: "Databases" },
  { value: "dev", label: "Dev" },
];

export const ChipGroup = ({ value, onChange, options = PRESET_OPTIONS }: ChipGroupProps) => {
  return (
    <div className="inline-flex items-center rounded-pill border border-border p-0.5 gap-0.5">
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          onClick={() => onChange(option.value)}
          className={cn(
            "inline-flex items-center px-3 py-1.5 rounded-pill text-xs font-medium transition-colors",
            value === option.value
              ? "bg-[var(--color-primary)] text-[var(--color-secondary)]"
              : "text-[var(--color-muted)] hover:text-[var(--color-neutral)]"
          )}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
};