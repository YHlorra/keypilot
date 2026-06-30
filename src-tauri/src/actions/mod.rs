// src-tauri/src/actions/mod.rs
// Action Registry — discoverable/callable action layer for external Agent/MCP.
//
// Each action has a stable id (e.g. "provider.list"), human-readable name/description,
// JSON Schema for input/output, and a dispatch function that takes dynamic params
// and returns a dynamic result.
//
// The existing typed IPC handlers (commands/*.rs) remain unchanged — this layer
// is an additional generic interface on top of the same service layer.
//
// === Coverage note (Stage 10, V0.1) ===
// The 3 typed IPCs below are intentionally NOT in the action registry:
//   - commands::tray::pin_provider       (only triggered by tray UI; no Agent use case)
//   - commands::tray::unpin_provider     (only triggered by tray UI; no Agent use case)
//   - commands::quota::set_manual_quota  (Anthropic-only manual override; not an Agent action)
//
// All 21 other typed IPCs ARE exposed via this registry.

use crate::error::AppError;
use crate::store::AppState;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub mod category;
pub mod provider;
pub mod quota;
pub mod system;
pub mod token_usage;

/// Action definition — mirrors what an Agent/MCP needs to discover + call an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDef {
    /// Stable id, e.g. "provider.list", "quota.fetch"
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// One-line description for Agent consumption
    pub description: String,
    /// Category: "provider" | "category" | "quota" | "system" | "token_usage"
    pub category: String,
    /// JSON Schema describing required + optional params (object schema, properties keyed by field name)
    pub input_schema: Value,
    /// JSON Schema describing the result shape
    pub output_schema: Value,
}

/// Return all registered actions across all domains.
pub fn all_actions() -> Vec<ActionDef> {
    let mut actions = Vec::new();
    actions.extend(provider::actions());
    actions.extend(category::actions());
    actions.extend(quota::actions());
    actions.extend(system::actions());
    actions.extend(token_usage::actions());
    actions
}

/// Dispatch an action by id with dynamic params.
/// Returns the result as `serde_json::Value` so the same IPC handler can serve all actions.
pub async fn dispatch(
    state: &AppState,
    action_id: &str,
    params: Value,
) -> Result<Value, AppError> {
    // Pre-check: params must be an object. If Agent sends a non-object (e.g. a string),
    // fail fast with a clear validation error instead of a confusing "Missing field" message.
    if !params.is_object() && !params.is_null() {
        return Err(AppError::ActionValidation(
            "params must be a JSON object".into(),
        ));
    }

    match action_id {
        // --- provider ---
        "provider.list" => {
            let r = crate::services::provider::list_providers_by_state(state).await?;
            Ok(serde_json::to_value(r).map_err(AppError::Serde)?)
        }
        "provider.get" => {
            let id = require_i64(&params, "id")?;
            let r = crate::services::provider::get_provider_by_state(state, id).await?;
            Ok(serde_json::to_value(r).map_err(AppError::Serde)?)
        }
        "provider.open_for_edit" => {
            let id = require_i64(&params, "id")?;
            let r = crate::services::provider::get_provider_by_state(state, id).await?;
            Ok(serde_json::to_value(r).map_err(AppError::Serde)?)
        }
        "provider.copy_credential" => {
            let id = require_i64(&params, "id")?;
            let field_key = params_get(&params, "field_key")
                .and_then(|v| v.as_str())
                .map(String::from);
            let r = crate::services::provider::copy_credential_by_state(state, id, field_key).await?;
            Ok(serde_json::to_value(r).map_err(AppError::Serde)?)
        }
        "provider.test_and_refresh" => {
            let id = require_i64(&params, "id")?;
            // Multi-end: test + quota in sequence
            let test_result = match crate::commands::provider::test_connection_by_state(state, id).await {
                Ok(_) => Ok("ok".to_string()),
                Err(e) => Err(e),
            };
            let quota_result = crate::commands::quota::fetch_quota_by_state(state, id).await;
            // Compose: test result is required, quota is optional (may fail for non-supported presets like Anthropic)
            let test_status = match &test_result {
                Ok(s) => s.clone(),
                Err(e) => format!("error: {}", e),
            };
            let quota = quota_result.ok();
            Ok(json!({ "test": test_status, "quota": quota }))
        }
        "provider.add" => {
            let req: crate::services::provider::AddProviderRequest = parse(params)?;
            let r = crate::services::provider::add_provider_by_state(state, req).await?;
            Ok(serde_json::to_value(r).map_err(AppError::Serde)?)
        }
        "provider.update" => {
            let req: crate::services::provider::UpdateProviderRequest = parse(params)?;
            let r = crate::services::provider::update_provider_by_state(state, req).await?;
            Ok(serde_json::to_value(r).map_err(AppError::Serde)?)
        }
        "provider.delete" => {
            let id = require_i64(&params, "id")?;
            crate::services::provider::delete_provider_by_state(state, id).await?;
            Ok(Value::Null)
        }
        "provider.test_connection" => {
            let id = require_i64(&params, "id")?;
            crate::commands::provider::test_connection_by_state(state, id).await?;
            Ok(Value::Null)
        }

        // --- category ---
        "category.list" => {
            let r = crate::services::category::list_categories_by_state(state).await?;
            Ok(serde_json::to_value(r).map_err(AppError::Serde)?)
        }
        "category.add" => {
            let req: crate::services::category::AddCategoryRequest = parse(params)?;
            let r = crate::services::category::add_category_by_state(state, req).await?;
            Ok(serde_json::to_value(r).map_err(AppError::Serde)?)
        }
        "category.delete" => {
            let req: crate::services::category::DeleteCategoryRequest = parse(params)?;
            crate::services::category::delete_category_by_state(state, req).await?;
            Ok(Value::Null)
        }

        // --- quota ---
        "quota.fetch" => {
            let id = require_i64(&params, "id")?;
            let r = crate::commands::quota::fetch_quota_by_state(state, id).await?;
            Ok(serde_json::to_value(r).map_err(AppError::Serde)?)
        }

        // --- system ---
        "system.get_theme" => {
            let r = crate::commands::provider::get_theme_by_state(state).await?;
            Ok(serde_json::to_value(r).map_err(AppError::Serde)?)
        }
        "system.set_theme" => {
            let theme_str = require_string(&params, "theme")?;
            let theme = crate::types::Theme::parse(&theme_str)?;
            crate::commands::provider::set_theme_by_state(state, theme).await?;
            Ok(Value::Null)
        }
        "system.quit" => {
            // quit_app uses std::process::exit(0), no AppState needed.
            std::process::exit(0);
        }

        // --- token_usage ---
        "token_usage.record" => {
            // Flat shape: params = UsageRecordInputIpc directly.
            let input: crate::commands::token_usage::UsageRecordInputIpc = parse(params)?;
            let r = crate::commands::token_usage::record_usage_by_state(state, input).await?;
            Ok(serde_json::to_value(r).map_err(AppError::Serde)?)
        }
        "token_usage.list" => {
            let req: crate::commands::token_usage::ListUsageRecordsRequest = parse(params)?;
            let r = crate::commands::token_usage::list_usage_records_by_state(state, req).await?;
            Ok(serde_json::to_value(r).map_err(AppError::Serde)?)
        }
        "token_usage.summary" => {
            // Empty `{}` accepted via Default; serde handles null too.
            let filter: crate::commands::token_usage::UsageFilterIpc = parse(params)?;
            let r = crate::commands::token_usage::get_usage_summary_by_state(state, filter).await?;
            Ok(serde_json::to_value(r).map_err(AppError::Serde)?)
        }
        "token_usage.import" => {
            let content = require_string(&params, "content")?;
            let format = require_string(&params, "format")?;
            let source_hint = params_get(&params, "source_hint")
                .and_then(|v| v.as_str())
                .map(String::from);
            let r = crate::commands::token_usage::import_usage_by_state(state, content, format, source_hint).await?;
            Ok(serde_json::to_value(r).map_err(AppError::Serde)?)
        }
        "token_usage.pricing" => {
            let r = crate::commands::token_usage::get_pricing_by_state(state).await?;
            Ok(serde_json::to_value(r).map_err(AppError::Serde)?)
        }
        "token_usage.import_opencode_db" => {
            let db_path = require_string(&params, "db_path")?;
            let req = crate::commands::token_usage::ImportOpencodeDbRequest { db_path };
            let r = crate::commands::token_usage::import_opencode_db_by_state(state, req).await?;
            Ok(serde_json::to_value(r).map_err(AppError::Serde)?)
        }
        "token_usage.force_rescan_all" => {
            // Bug #3 escape hatch (2026-06-29) — wipe cursors so the next
            // scan re-ingests every known JSONL file from byte 0.  FNV-1a
            // dedup keeps the DB clean.
            let r = crate::commands::token_usage::force_rescan_all_by_state(state).await?;
            Ok(serde_json::to_value(r).map_err(AppError::Serde)?)
        }

        _ => Err(AppError::ActionNotFound(action_id.to_string())),
    }
}

// --- helpers ---

/// Convert `serde_json::Value` into a typed request struct.
fn parse<T: serde::de::DeserializeOwned>(v: Value) -> Result<T, AppError> {
    serde_json::from_value(v).map_err(AppError::Serde)
}

/// Get a required field from params as i64.
fn require_i64(params: &Value, field: &str) -> Result<i64, AppError> {
    params_get(params, field)
        .and_then(|v| v.as_i64())
        .ok_or_else(|| AppError::ActionValidation(format!("Missing or invalid required field: {}", field)))
}

/// Get a required string field from params.
fn require_string(params: &Value, field: &str) -> Result<String, AppError> {
    params_get(params, field)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| AppError::ActionValidation(format!("Missing or invalid required field: {}", field)))
}

/// Get an optional field from params.
fn params_get<'a>(params: &'a Value, field: &str) -> Option<&'a Value> {
    params.as_object().and_then(|o| o.get(field))
}
