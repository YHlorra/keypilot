import { useQuery } from "@tanstack/react-query";
import { listProviders } from "@/lib/api";
import type { Provider } from "@/types/api";

export function useProviders() {
  return useQuery({
    queryKey: ["providers"],
    queryFn: () => listProviders(),
  });
}

export function filterProviders(providers: Provider[], query: string): Provider[] {
  const q = query.trim().toLowerCase();
  if (!q) return providers;
  return providers.filter((p) => {
    if (p.name.toLowerCase().includes(q)) return true;
    return p.fields.some((f) => f.key.toLowerCase().includes(q));
  });
}
