import { Input } from "./ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "./ui/select";
import { DensityToggle } from "./DensityToggle";

export interface TopBarProps {
  search: string;
  onSearchChange: (v: string) => void;
  categoryFilter: number | "all";
  onCategoryChange: (v: number | "all") => void;
  density: "1" | "2";
  onDensityChange: (v: "1" | "2") => void;
  categories: Array<{ id: number; name: string }>;
}

const ALL_VALUE = "all";

export const TopBar = ({
  search,
  onSearchChange,
  categoryFilter,
  onCategoryChange,
  density,
  onDensityChange,
  categories,
}: TopBarProps) => {
  // The Select keeps a string-typed value; we round-trip to/from categoryFilter (number | "all")
  // at the boundary.
  const selectValue = categoryFilter === "all" ? ALL_VALUE : String(categoryFilter);
  const handleCategorySelect = (v: string) => {
    onCategoryChange(v === ALL_VALUE ? "all" : Number(v));
  };

  return (
    <div
      className="fixed left-0 md:left-16 right-0 z-40 flex flex-col sm:flex-row items-start sm:items-center gap-3 px-4 py-3 border-b border-border bg-card"
      style={{ top: 48, minHeight: 60 }}
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

      {/* Right cluster: category filter (Select dropdown) + density */}
      <div className="flex flex-wrap items-center gap-2 ml-auto">
        {/* Category filter -- Select dropdown. Replaces the previous ChipGroup that overflowed
            and broke Chinese text mid-character when the user had many categories. */}
        <Select value={selectValue} onValueChange={handleCategorySelect}>
          <SelectTrigger
            aria-label="Filter by category"
            className="h-9 w-[180px] rounded-sm border-[var(--color-border)] bg-transparent text-sm"
          >
            <SelectValue placeholder="All categories" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={ALL_VALUE}>All categories</SelectItem>
            {categories.map((c) => (
              <SelectItem key={c.id} value={String(c.id)}>
                {c.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        <DensityToggle value={density} onChange={onDensityChange} />
      </div>
    </div>
  );
};
