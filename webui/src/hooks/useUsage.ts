import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { UseMutationResult, UseQueryResult } from "@tanstack/react-query";
import {
  getPricing,
  getUsageSummary,
  importUsage,
  listUsageRecords,
} from "@/lib/api";
import type {
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