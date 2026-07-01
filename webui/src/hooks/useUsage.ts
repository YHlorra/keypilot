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

// useUsagePeriodsSummary -- token-monitor-alignment Part A #1
// 一次返回 today/month/allTime 三周期 + client_models + limits,不再双 useQuery
export function useUsagePeriodsSummary(
  filter: UsageFilter
): UseQueryResult<PeriodsSummary> {
  return useQuery({
    queryKey: ["usage", "periods", filter],
    queryFn: () => getUsagePeriodsSummary(filter),
    staleTime: 60 * 1000, // 1 min(对齐 token-monitor usage.js 主数据契约)
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

// useLastAutoImport -- reads `meta.last_auto_import` JSON on App mount.
// Returns null if no run has been recorded yet, or the parsed summary otherwise.
//
// Auto-import now runs in `spawn_blocking` (non-blocking to webview creation,
// see lib.rs setup).  On cold start the meta row may not be written yet when
// the query first fires — we poll briefly inside queryFn so a slow scan still
// surfaces its summary toast without relying on React Query's retry (which
// only triggers on queryFn rejection, not on a null return).
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
    staleTime: Infinity, // one-shot — we only fire on cold start
    gcTime: 60 * 1000,
    retry: false,
  });
}