import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { getTheme, setTheme } from "@/lib/api";
import type { Theme, SetThemeRequest } from "@/types/api";

export function useTheme() {
  const queryClient = useQueryClient();
  const query = useQuery({
    queryKey: ["theme"],
    queryFn: () => getTheme(),
  });

  const mutation = useMutation({
    mutationFn: (theme: Theme) => setTheme({ theme } as SetThemeRequest),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["theme"] });
    },
  });

  return { theme: query.data, isLoading: query.isLoading, setTheme: mutation.mutate };
}
