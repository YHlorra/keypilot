import { useQuery } from "@tanstack/react-query";
import type { UseQueryResult } from "@tanstack/react-query";
import { fetchQuota } from "@/lib/api";
import type { FetchQuotaResponse } from "@/types/api";



const STALE_TIME_MS = 5 * 60 * 1000;
const GC_TIME_MS = STALE_TIME_MS * 2;

export function useProviderQuota(
  providerId: number | null | undefined
): UseQueryResult<FetchQuotaResponse> {
  return useQuery({
    queryKey: ["provider_quota", providerId],
    queryFn: () => fetchQuota({ id: providerId! }),
    enabled: providerId != null,
    staleTime: STALE_TIME_MS,
    gcTime: GC_TIME_MS,
    retry: false,
  });
}
