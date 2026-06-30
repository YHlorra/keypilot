import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import type { TokenUsageTickPayload } from "@/types/api";

/**
 * Listen for `token_usage_tick` events emitted by the Rust
 * `IncrementalImporter` (services/incremental_import.rs).  When a tick
 * arrives, invalidate the periods query so the heatmap / trend / KPI
 * cards re-fetch the latest aggregate.
 *
 * Bug #3 fix 2026-06-29: replaced silent cold-start scan with real-time
 * file watcher that pushes deltas as the user's coding agent appends
 * JSONL lines.  The frontend doesn't need to poll — it just listens.
 */
export function useUsageTick() {
  const queryClient = useQueryClient();

  useEffect(() => {
    let unlistenFn: (() => void) | undefined;
    let cancelled = false;

    listen<TokenUsageTickPayload>("token_usage_tick", () => {
      // Re-fetch the periods summary so KPI cards / heatmap reflect the
      // new totals.  V0.1 keeps the refresh path simple: invalidate and
      // let TanStack Query hit the IPC again.  Task 2 (popover) will
      // patch in a setQueryData fast-path for the popover window.
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
        // Listener failure shouldn't break the page; log and move on.
        // eslint-disable-next-line no-console
        console.warn("[useUsageTick] listen failed:", e);
      });

    return () => {
      cancelled = true;
      if (unlistenFn) unlistenFn();
    };
  }, [queryClient]);
}