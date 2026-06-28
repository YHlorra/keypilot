import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { UseMutationResult, UseQueryResult } from "@tanstack/react-query";
import {
  getLastAutoImport,
  getPricing,
  getUsageSummary,
  importUsage,
  listUsageRecords,
} from "@/lib/api";
import type {
  AutoImportSummary,
  ImportFormat,
  ImportResult,
  PaginatedResponse,
  PricingEntry,
  UsageFilter,
  UsageRecord,
  UsageSummary,
} from "@/types/api";

// useUsageSummary -- REQ-TOKEN-003.3
export function useUsageSummary(
  filter: UsageFilter
): UseQueryResult<UsageSummary> {
  return useQuery({
    queryKey: ["usage", "summary", filter],
    queryFn: () => getUsageSummary(filter),
    staleTime: 5 * 60 * 1000, // 5 min
  });
}

// useUsageRecords -- REQ-TOKEN-003.3
export function useUsageRecords(
  filter: UsageFilter,
  page: number,
  perPage: number
): UseQueryResult<PaginatedResponse<UsageRecord>> {
  return useQuery({
    queryKey: ["usage", "records", filter, page, perPage],
    queryFn: () => listUsageRecords(filter, page, perPage),
    staleTime: 5 * 60 * 1000, // 5 min
  });
}

// useImportUsage -- REQ-TOKEN-003.3
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

// usePricing -- REQ-TOKEN-003.3
export function usePricing(): UseQueryResult<PricingEntry[]> {
  return useQuery({
    queryKey: ["usage", "pricing"],
    queryFn: () => getPricing(),
    staleTime: 60 * 60 * 1000, // 1 hour
  });
}

// useLastAutoImport -- reads `meta.last_auto_import` JSON on App mount.
// Returns null if no run has been recorded yet, or the parsed summary otherwise.
export function useLastAutoImport(): UseQueryResult<AutoImportSummary | null> {
  return useQuery({
    queryKey: ["usage", "last-auto-import"],
    queryFn: async () => {
      const raw = await getLastAutoImport();
      if (!raw) return null;
      try {
        return JSON.parse(raw) as AutoImportSummary;
      } catch {
        return null;
      }
    },
    staleTime: Infinity, // one-shot — we only fire on cold start
    gcTime: 60 * 1000,
    retry: false,
  });
}