import { useQuery } from "@tanstack/react-query";
import { listProviders } from "@/lib/api";

export function useProviders() {
  return useQuery({
    queryKey: ["providers"],
    queryFn: () => listProviders(),
  });
}
