// webui/src/lib/action-registry.ts
// Action Registry — frontend mirror of src-tauri/src/actions/.
//
// Mirrors the backend ActionDef structure and provides typed wrappers around
// list_actions and execute_action IPC handlers. Used by:
//   - useActions.ts (React Query hooks)
//   - any future Agent/MCP integration on the frontend

import { invoke } from "@tauri-apps/api/core";

/**
 * Action definition — mirrors src-tauri/src/actions/mod.rs::ActionDef.
 * Each action has a stable id, human-readable name/description, category,
 * and a JSON Schema (record-of-unknown for hint/validation) for input/output.
 */
export interface ActionDef {
  id: string;
  name: string;
  description: string;
  category: "provider" | "category" | "quota" | "system" | "token_usage";
  input_schema: Record<string, unknown>;
  output_schema: Record<string, unknown>;
}

/** Return all registered actions from the backend. */
export async function listActions(): Promise<ActionDef[]> {
  return invoke<ActionDef[]>("list_actions");
}

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
