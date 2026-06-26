import { Input } from "./ui/input";
import { ChipGroup } from "./ChipGroup";
import { DensityToggle } from "./DensityToggle";

export interface TopBarProps {
  search: string;
  onSearchChange: (v: string) => void;
  categoryFilter: string;
  onCategoryChange: (v: string) => void;
  density: "1" | "2";
  onDensityChange: (v: "1" | "2") => void;
  currentPage: "credentials" | "usage";
  onPageChange: (v: "credentials" | "usage") => void;
}

const PAGE_OPTIONS: { value: "credentials" | "usage"; label: string }[] = [
  { value: "credentials", label: "Credentials" },
  { value: "usage", label: "Usage" },
];

export const TopBar = ({
  search,
  onSearchChange,
  categoryFilter,
  onCategoryChange,
  density,
  onDensityChange,
  currentPage,
  onPageChange,
}: TopBarProps) => {
  return (
    <div
      className="fixed top-[44px] left-0 right-0 z-40 flex flex-col sm:flex-row items-start sm:items-center gap-3 px-4 py-3 border-b border-border bg-card"
      style={{ minHeight: 60 }}
    >
      {/* Search input - 360px wide on desktop */}
      <div className="w-full sm:w-[360px] shrink-0">
        <Input
          type="search"
          placeholder="Search credentials…"
          value={search}
          onChange={(e) => onSearchChange(e.target.value)}
          className="rounded-pill border-[var(--color-border)] bg-transparent h-9 text-sm"
        />
      </div>

      {/* ChipGroup + DensityToggle */}
      <div className="flex flex-wrap items-center gap-2">
        {/* Page nav chips -- visually distinct, slightly larger */}
        <div className="inline-flex items-center rounded-pill border border-border bg-muted p-0.5 gap-0.5">
          {PAGE_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              type="button"
              onClick={() => onPageChange(opt.value)}
              className={`inline-flex items-center px-4 py-1.5 rounded-pill text-sm font-medium transition-colors ${
                currentPage === opt.value
                  ? "bg-[var(--color-primary)] text-[var(--color-secondary)]"
                  : "text-[var(--color-muted)] hover:text-[var(--color-neutral)]"
              }`}
            >
              {opt.label}
            </button>
          ))}
        </div>
        {/* Category filter chips (existing) */}
        <ChipGroup value={categoryFilter} onChange={onCategoryChange} />
        <DensityToggle value={density} onChange={onDensityChange} />
      </div>
    </div>
  );
};