// webui/src/hooks/useActions.ts
// React Query hooks for the Action Registry (Stage 10).
//
// - useActionList: fetches all registered actions (cached for 1h — they don't change at runtime).
// - useExecuteAction: mutation for executing an action by id with dynamic params.

import { useMutation, useQuery } from "@tanstack/react-query";
import type { UseMutationResult, UseQueryResult } from "@tanstack/react-query";
import { executeAction, listActions } from "@/lib/action-registry";
import type { ActionDef } from "@/lib/action-registry";

/** Fetch all registered actions (1h staleTime — registry is static at runtime). */
export function useActionList(): UseQueryResult<ActionDef[]> {
  return useQuery({
    queryKey: ["actions", "list"],
    queryFn: () => listActions(),
    staleTime: 60 * 60 * 1000, // 1 hour
  });
}

/**
 * Execute an action by id with dynamic params.
 * Returns `unknown` — caller is responsible for validating the result shape.
 */
export function useExecuteAction(): UseMutationResult<
  unknown,
  Error,
  { actionId: string; params?: unknown }
> {
  return useMutation({
    mutationFn: ({ actionId, params }) => executeAction(actionId, params),
  });
}

/**
 * Look up a single action by id from the cached list.
 * Returns `undefined` if the list is still loading or the id is not found.
 */
export function useAction(actionId: string | null | undefined): ActionDef | undefined {
  const { data } = useActionList();
  if (!actionId || !data) return undefined;
  return data.find((a) => a.id === actionId);
}
