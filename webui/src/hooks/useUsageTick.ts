import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import type { TokenUsageTickPayload } from "@/types/api";











export function useUsageTick() {
  const queryClient = useQueryClient();

  useEffect(() => {
    let unlistenFn: (() => void) | undefined;
    let cancelled = false;

    listen<TokenUsageTickPayload>("token_usage_tick", () => {
      
      
      
      
      queryClient.invalidateQueries({ queryKey: ["usage", "periods"] });
      queryClient.invalidateQueries({ queryKey: ["usage", "summary"] });
      queryClient.invalidateQueries({ queryKey: ["usage", "records"] });
    })
      .then((fn) => {
        if (cancelled) {
          fn();
        } else {
          unlistenFn = fn;
        }
      })
      .catch((e) => {
        
        
        console.warn("[useUsageTick] listen failed:", e);
      });

    return () => {
      cancelled = true;
      if (unlistenFn) unlistenFn();
    };
  }, [queryClient]);
}