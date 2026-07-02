import { useQuery } from "@tanstack/react-query";
import type { UseQueryResult } from "@tanstack/react-query";
import { fetchCodingPlanQuota } from "@/lib/api";
import type { FetchCodingPlanQuotaResponse } from "@/types/api";

// 5 min — more aggressive than the Rust QUOTA_CACHE_TTL_SECS=900 so a
// manual refetch reflects upstream changes sooner. Auth failures (401/403)
// surface immediately because retry is disabled.
const STALE_TIME_MS = 5 * 60 * 1000;
const GC_TIME_MS = STALE_TIME_MS * 2;

export function useCodingPlanQuota(
  providerId: number | null | undefined
): UseQueryResult<FetchCodingPlanQuotaResponse> {
  return useQuery({
    queryKey: ["coding_plan_quota", providerId],
    queryFn: () => fetchCodingPlanQuota({ id: providerId! }),
    enabled: providerId != null,
    staleTime: STALE_TIME_MS,
    gcTime: GC_TIME_MS,
    retry: false,
  });
}
