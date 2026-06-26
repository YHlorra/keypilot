// src-tauri/src/commands/action.rs
// Action Registry IPC handlers (Stage 10)
//
// Two generic IPC handlers that expose the actions/ registry to external Agent/MCP:
// - list_actions: returns Vec<ActionDef> describing all available actions
// - execute_action: dispatches an action by id with dynamic params

use crate::actions::{self, ActionDef};
use crate::error::AppError;
use crate::store::AppState;
use serde::Deserialize;
use serde_json::Value;
use tauri::State;

/// Return all registered actions with their input/output schemas.
#[tauri::command]
pub fn list_actions() -> Vec<ActionDef> {
    actions::all_actions()
}

/// Execute an action by id with dynamic params.
/// Returns the result as `serde_json::Value` (or `null` for void actions).
#[tauri::command]
pub async fn execute_action(
    state: State<'_, AppState>,
    req: ExecuteActionRequest,
) -> Result<Value, AppError> {
    actions::dispatch(&state, &req.action_id, req.params.unwrap_or(Value::Null)).await
}

#[derive(Debug, Deserialize)]
pub struct ExecuteActionRequest {
    pub action_id: String,
    /// Dynamic params — shape depends on action_id (see ActionDef::input_schema)
    pub params: Option<Value>,
}
