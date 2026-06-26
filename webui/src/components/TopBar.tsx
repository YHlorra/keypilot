import { Input } from "./ui/input";
import { ChipGroup } from "./ChipGroup";
import { DensityToggle } from "./DensityToggle";
import { Button } from "./ui/button";

export interface TopBarProps {
  search: string;
  onSearchChange: (v: string) => void;
  categoryFilter: number | "all";
  onCategoryChange: (v: number | "all") => void;
  density: "1" | "2";
  onDensityChange: (v: "1" | "2") => void;
  currentPage: "credentials" | "usage";
  onPageChange: (v: "credentials" | "usage") => void;
  categories: Array<{ id: number; name: string }>;
  onAddClick: () => void;
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
  categories,
  onAddClick,
}: TopBarProps) => {
  const CATEGORY_OPTIONS = [
    { value: "all", label: "All" },
    ...categories.map((c) => ({ value: String(c.id), label: c.name })),
  ];

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
        {/* Category filter chips */}
        <ChipGroup
          value={categoryFilter === "all" ? "all" : String(categoryFilter)}
          onChange={(v) => onCategoryChange(v === "all" ? "all" : Number(v))}
          options={CATEGORY_OPTIONS}
        />
        <DensityToggle value={density} onChange={onDensityChange} />
      </div>

      {/* Add credential button */}
      <div className="ml-auto">
        <Button size="sm" onClick={onAddClick} data-testid="add-credential-btn">
          + 添加凭证
        </Button>
      </div>
    </div>
  );
};