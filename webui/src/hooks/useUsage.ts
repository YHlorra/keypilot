import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { UseMutationResult, UseQueryResult } from "@tanstack/react-query";
import {
  getLastAutoImport,
  getUsagePeriodsSummary,
  importUsage,
} from "@/lib/api";
import type {
  AutoImportSummary,
  ImportFormat,
  ImportResult,
  PeriodsSummary,
  UsageFilter,
} from "@/types/api";



export function useUsagePeriodsSummary(
  filter: UsageFilter
): UseQueryResult<PeriodsSummary> {
  return useQuery({
    queryKey: ["usage", "periods", filter],
    queryFn: () => getUsagePeriodsSummary(filter),
    staleTime: 60 * 1000, 
  });
}


export function useImportUsage(): UseMutationResult<
  ImportResult,
  Error,
  { content: string; format: ImportFormat; sourceHint?: string }
> {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ content, format, sourceHint }) =>
      importUsage(content, format, sourceHint),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["usage", "summary"] });
      queryClient.invalidateQueries({ queryKey: ["usage", "records"] });
    },
  });
}









export function useLastAutoImport(): UseQueryResult<AutoImportSummary | null> {
  return useQuery({
    queryKey: ["usage", "last-auto-import"],
    queryFn: async () => {
      for (let attempt = 0; attempt < 4; attempt++) {
        const raw = await getLastAutoImport();
        if (raw) {
          try {
            return JSON.parse(raw) as AutoImportSummary;
          } catch {
            return null;
          }
        }
        await new Promise((r) => setTimeout(r, 300));
      }
      return null;
    },
    staleTime: Infinity, 
    gcTime: 60 * 1000,
    retry: false,
  });
}