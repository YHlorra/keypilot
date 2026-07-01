// webui/src/lib/action-registry.ts
// Action Registry — frontend mirror of src-tauri/src/actions/.

import { invoke } from "@tauri-apps/api/core";

/**
 * Execute an action by id with dynamic params.
 * Returns the result as `unknown` (caller should validate shape).
 */
export async function executeAction(
  actionId: string,
  params?: unknown
): Promise<unknown> {
  return invoke<unknown>("execute_action", {
    req: { action_id: actionId, params: params ?? null },
  });
}