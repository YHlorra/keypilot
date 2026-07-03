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
  
  
  const selectValue = categoryFilter === "all" ? ALL_VALUE : String(categoryFilter);
  const handleCategorySelect = (v: string) => {
    onCategoryChange(v === ALL_VALUE ? "all" : Number(v));
  };

  return (
    
    
    
    <div className="shrink-0 flex flex-col sm:flex-row items-start sm:items-center gap-3 px-4 py-3 border-b border-border bg-card">
      {}
      <div className="w-full sm:w-[360px] shrink-0">
        <Input
          type="search"
          placeholder="Search credentials…"
          value={search}
          onChange={(e) => onSearchChange(e.target.value)}
          className="rounded-pill border-[var(--color-border)] bg-transparent h-9 text-sm"
        />
      </div>

      {}
      <div className="flex flex-wrap items-center gap-2 ml-auto">
        {
}
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
