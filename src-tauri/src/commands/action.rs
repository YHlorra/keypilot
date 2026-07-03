






use crate::actions::{self, ActionDef};
use crate::error::AppError;
use crate::store::AppState;
use serde::Deserialize;
use serde_json::Value;
use tauri::State;


#[tauri::command]
pub fn list_actions() -> Vec<ActionDef> {
    actions::all_actions()
}



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
    
    pub params: Option<Value>,
}
