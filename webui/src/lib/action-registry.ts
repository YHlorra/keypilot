


import { invoke } from "@tauri-apps/api/core";





export async function executeAction(
  actionId: string,
  params?: unknown
): Promise<unknown> {
  return invoke<unknown>("execute_action", {
    req: { action_id: actionId, params: params ?? null },
  });
}