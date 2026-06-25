import { useQuery } from "@tanstack/react-query";
import { fetchQuota } from "@/lib/api";
import type { FetchQuotaRequest } from "@/types/api";

export function useQuota(providerId: number | null) {
  return useQuery({
    queryKey: ["quota", providerId],
    queryFn: () => fetchQuota({ id: providerId! } as FetchQuotaRequest),
    enabled: providerId !== null,
    staleTime: 5 * 60 * 1000,  // 5 min per REQ-QUOTA-DISPLAY-001
    gcTime: 30 * 60 * 1000,    // 30 min
  });
}
