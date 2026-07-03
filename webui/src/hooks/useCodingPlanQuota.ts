import { useQuery } from "@tanstack/react-query";
import type { UseQueryResult } from "@tanstack/react-query";
import { fetchCodingPlanQuota } from "@/lib/api";
import type { FetchCodingPlanQuotaResponse } from "@/types/api";




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
