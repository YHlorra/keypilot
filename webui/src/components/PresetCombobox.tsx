import * as React from "react";
import { useState, useMemo, useCallback } from "react";
import {
  Command,
  CommandInput,
  CommandList,
  CommandEmpty,
  CommandItem,
  CommandSeparator,
} from "./ui/command";
import type { CatalogPresetMeta } from "@/types/api";

const CUSTOM_PRESET_ID = "__custom__";

interface PresetComboboxProps {
  options: CatalogPresetMeta[];
  value: string | null;
  onValueChange: (id: string) => void;
  placeholder?: string;
  allBuiltinsUsed?: boolean;
}

export const PresetCombobox = React.memo(function PresetCombobox({
  options,
  value,
  onValueChange,
  allBuiltinsUsed = false,
}: PresetComboboxProps) {
  const [search, setSearch] = useState("");

  const selectedLabel = useMemo(() => {
    if (!value) return null;
    if (value === CUSTOM_PRESET_ID) return "自定义";
    const found = options.find((p) => p.id === value);
    return found?.name ?? null;
  }, [value, options]);

  const filter = useCallback(
    (itemValue: string, q: string, keywords?: string[]) => {
      const query = q.toLowerCase().trim();
      if (itemValue === CUSTOM_PRESET_ID) return 1;
      const haystack = (
        itemValue +
        " " +
        (keywords?.join(" ") ?? "")
      ).toLowerCase();
      return haystack.includes(query) ? 1 : 0;
    },
    []
  );

  const handleSelect = useCallback(
    (itemValue: string) => {
      onValueChange(itemValue);
      setSearch("");
    },
    [onValueChange]
  );

  return (
    <div className="rounded-sm border border-input">
      <Command filter={filter}>
        <CommandInput
          value={search}
          onValueChange={setSearch}
          placeholder="输入或搜索服务名..."
          className="border-b border-border"
        />
        <CommandList className="max-h-[240px]">
          {options.length === 0 && !allBuiltinsUsed ? (
            <CommandEmpty>无可用服务</CommandEmpty>
          ) : (
            <>
              {options.map((p) => (
                <CommandItem
                  key={p.id}
                  value={p.id}
                  keywords={[p.name]}
                  onSelect={() => handleSelect(p.id)}
                >
                  <span className="flex-1">{p.name}</span>
                  {value === p.id && (
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      width="12"
                      height="12"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      className="opacity-70"
                    >
                      <path d="m20 6 9 17l-5-5" />
                    </svg>
                  )}
                </CommandItem>
              ))}
              <CommandSeparator />
              <CommandItem
                value={CUSTOM_PRESET_ID}
                onSelect={() => handleSelect(CUSTOM_PRESET_ID)}
                className="text-muted-foreground"
              >
                <span className="flex-1">+ 自定义...</span>
              </CommandItem>
            </>
          )}
        </CommandList>
      </Command>

      {selectedLabel && (
        <div className="px-3 py-1.5 border-t border-border text-xs text-muted-foreground">
          当前: {selectedLabel}
        </div>
      )}

      {allBuiltinsUsed && (
        <div className="px-3 py-1.5 border-t border-border text-xs text-muted-foreground">
          该分类下已添加所有预设服务。可使用「+ 自定义...」添加其他服务。
        </div>
      )}
    </div>
  );
});
