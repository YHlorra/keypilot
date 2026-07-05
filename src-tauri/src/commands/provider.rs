use crate::catalog::{self, CustomSpec, resolve_custom};
use crate::error::AppError;
use crate::provider::registry::{adapter_for, ValidateError};
use crate::services::category::{add_category as svc_add_category, delete_category as svc_delete_category, list_categories as svc_list_categories, AddCategoryRequest, DeleteCategoryRequest};
use crate::services::provider::{add_provider as svc_add_provider, delete_provider as svc_delete_provider, get_provider as svc_get_provider, list_providers as svc_list_providers, update_provider as svc_update_provider, AddProviderRequest, UpdateProviderRequest};
use crate::store::AppState;
use crate::types::{Category, Provider, Theme};
use std::collections::HashMap;

#[derive(Debug, serde::Deserialize)]
pub struct PreflightRequest {
    pub custom_spec: serde_json::Value,
    pub api_key: String,
}

#[derive(Debug, serde::Serialize)]
pub struct PreflightResult {
    pub ok: bool,
    pub message: String,
}

// Phase 5: detailed test result with extras reporting
#[derive(Debug, serde::Serialize)]
pub struct TestConnectionResult {
    pub primary_ok: bool,
    pub primary_message: String,
    pub extras: Vec<ExtraTestResult>,
}

#[derive(Debug, serde::Serialize)]
pub struct ExtraTestResult {
    pub protocol: String,
    pub ok: bool,
    pub message: String,
}

#[tauri::command]
pub async fn list_providers(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Provider>, AppError> {
    svc_list_providers(state).await
}

#[tauri::command]
pub async fn get_provider(
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<Provider, AppError> {
    svc_get_provider(state, id).await
}

#[tauri::command]
pub async fn add_provider(
    state: tauri::State<'_, AppState>,
    req: AddProviderRequest,
) -> Result<Provider, AppError> {
    svc_add_provider(state, req).await
}

#[tauri::command]
pub async fn update_provider(
    state: tauri::State<'_, AppState>,
    req: UpdateProviderRequest,
) -> Result<Provider, AppError> {
    svc_update_provider(state, req).await
}

#[tauri::command]
pub async fn delete_provider(
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<(), AppError> {
    svc_delete_provider(state, id).await
}

#[tauri::command]
pub async fn list_categories(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Category>, AppError> {
    svc_list_categories(state).await
}

#[tauri::command]
pub async fn add_category(
    state: tauri::State<'_, AppState>,
    req: AddCategoryRequest,
) -> Result<Category, AppError> {
    svc_add_category(state, req).await
}

#[tauri::command]
pub async fn delete_category(
    state: tauri::State<'_, AppState>,
    req: DeleteCategoryRequest,
) -> Result<(), AppError> {
    svc_delete_category(state, req).await
}

pub async fn test_connection_by_state(
    state: &AppState,
    id: i64,
) -> Result<(), AppError> {
    let db = state.db.clone();

    let join_result = tauri::async_runtime::spawn_blocking(move || {
        let guard = db.lock().unwrap();

        let preset: Option<String> = {
            let mut stmt = guard.conn.prepare("SELECT preset FROM providers WHERE id = ?1")?;
            stmt.query_row([id], |row| row.get::<_, Option<String>>(0)).ok().flatten()
        };

        let custom_spec_json: Option<String> = guard.conn
            .prepare("SELECT custom_spec FROM providers WHERE id = ?1")
            .ok()
            .and_then(|mut stmt| stmt.query_row([id], |row| row.get::<_, Option<String>>(0)).ok().flatten());

        let mut field_stmt = guard.conn.prepare("SELECT key, value FROM provider_fields WHERE provider_id = ?1")?;
        let mut field_rows: Vec<(String, String)> = Vec::new();
        let mut rows = field_stmt.query([id])?;
        while let Some(row) = rows.next()? {
            let k: String = row.get(0)?;
            let v: String = row.get(1)?;
            field_rows.push((k, v));
        }

        Ok::<_, AppError>((preset, custom_spec_json, field_rows))
    }).await;

    let (preset, custom_spec_json, field_rows) = match join_result {
        Ok(inner) => inner?,
        Err(e) => return Err(AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))),
    };

    let preset = preset.ok_or_else(|| AppError::ProviderNotFound(id))?;

    let field_map: HashMap<String, String> = field_rows.into_iter().collect();
    let api_key = field_map.get("api_key").cloned().unwrap_or_default();

    // Resolve via catalog (preserves V0.1 base_url field fallback for legacy data)
    let custom_spec: Option<CustomSpec> = custom_spec_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());
    let resolved = match &custom_spec {
        Some(cs) => catalog::resolve(&preset, Some(cs))?,
        None => catalog::resolve(&preset, None)?,
    };

    let adapter = adapter_for(resolved.protocol);
    adapter.validate(&resolved, &api_key).await.map_err(|e| match e {
        ValidateError::InvalidKey => AppError::Http("Invalid API key".to_string()),
        ValidateError::Ambiguous => AppError::Http("Ambiguous response".to_string()),
        ValidateError::Network(msg) => AppError::Http(msg),
    })
}

/// Phase 5: test primary + all extras, return detailed result per REQ-CAT-023
#[tauri::command]
pub async fn test_connection_detailed(
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<TestConnectionResult, AppError> {
    let db = state.db.clone();

    let join_result = tauri::async_runtime::spawn_blocking(move || {
        let guard = db.lock().unwrap();

        let preset: Option<String> = {
            let mut stmt = guard.conn.prepare("SELECT preset FROM providers WHERE id = ?1")?;
            stmt.query_row([id], |row| row.get::<_, Option<String>>(0)).ok().flatten()
        };

        let custom_spec_json: Option<String> = guard.conn
            .prepare("SELECT custom_spec FROM providers WHERE id = ?1")
            .ok()
            .and_then(|mut stmt| stmt.query_row([id], |row| row.get::<_, Option<String>>(0)).ok().flatten());

        let mut field_stmt = guard.conn.prepare("SELECT key, value FROM provider_fields WHERE provider_id = ?1")?;
        let mut field_rows: Vec<(String, String)> = Vec::new();
        let mut rows = field_stmt.query([id])?;
        while let Some(row) = rows.next()? {
            let k: String = row.get(0)?;
            let v: String = row.get(1)?;
            field_rows.push((k, v));
        }

        Ok::<_, AppError>((preset, custom_spec_json, field_rows))
    }).await;

    let (preset, custom_spec_json, field_rows) = match join_result {
        Ok(inner) => inner?,
        Err(e) => return Err(AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))),
    };

    let preset = preset.ok_or_else(|| AppError::ProviderNotFound(id))?;

    let field_map: HashMap<String, String> = field_rows.into_iter().collect();
    let api_key = field_map.get("api_key").cloned().unwrap_or_default();

    let custom_spec: Option<CustomSpec> = custom_spec_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());
    let resolved = match &custom_spec {
        Some(cs) => catalog::resolve(&preset, Some(cs))?,
        None => catalog::resolve(&preset, None)?,
    };

    // Primary validate
    let adapter = adapter_for(resolved.protocol);
    let (primary_ok, primary_message) = match adapter.validate(&resolved, &api_key).await {
        Ok(()) => (true, "连接成功".to_string()),
        Err(ValidateError::InvalidKey) => (false, "Invalid API key".to_string()),
        Err(ValidateError::Ambiguous) => (false, "Ambiguous response".to_string()),
        Err(ValidateError::Network(msg)) => (false, msg),
    };

    // Extras validate
    let mut extras_results = Vec::new();
    for extra in &resolved.extras {
        let extra_resolved = crate::catalog::ResolvedProvider {
            id: format!("{}-extra-{}", resolved.id, extra.protocol.as_str()),
            name: format!("{} ({})", resolved.name, extra.protocol.as_str()),
            icon: resolved.icon.clone(),
            protocol: extra.protocol,
            base_url: extra.base_url.clone(),
            auth_header: extra.auth_header.clone(),
            validate_probe: extra.validate_probe.clone(),
            quota_probe: resolved.quota_probe.clone(),
            coding_plan: resolved.coding_plan,
            extras: vec![],
        };
        let extra_adapter = adapter_for(extra.protocol);
        let (ok, message) = match extra_adapter.validate(&extra_resolved, &api_key).await {
            Ok(()) => (true, "连接成功".to_string()),
            Err(ValidateError::InvalidKey) => (false, "Invalid API key".to_string()),
            Err(ValidateError::Ambiguous) => (false, "Ambiguous response".to_string()),
            Err(ValidateError::Network(msg)) => (false, msg),
        };
        extras_results.push(ExtraTestResult {
            protocol: extra.protocol.as_str().to_string(),
            ok,
            message,
        });
    }

    Ok(TestConnectionResult {
        primary_ok,
        primary_message,
        extras: extras_results,
    })
}

#[tauri::command]
pub async fn get_theme(
    state: tauri::State<'_, AppState>,
) -> Result<Theme, AppError> {
    get_theme_by_state(&state).await
}

pub async fn get_theme_by_state(state: &AppState) -> Result<Theme, AppError> {
    let db = state.db.clone();

    let join_result = tauri::async_runtime::spawn_blocking(move || {
        let guard = db.lock().unwrap();
        let theme: String = guard.conn.query_row(
            "SELECT value FROM meta WHERE key = 'theme'",
            [],
            |row| row.get(0),
        ).unwrap_or_else(|_| "auto".to_string());
        Ok::<_, AppError>(theme)
    }).await;

    let theme_str = match join_result {
        Ok(inner) => inner?,
        Err(e) => return Err(AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))),
    };

    Theme::parse(&theme_str)
}

#[tauri::command]
pub async fn set_theme(
    state: tauri::State<'_, AppState>,
    theme: Theme,
) -> Result<(), AppError> {
    set_theme_by_state(&state, theme).await
}

pub async fn set_theme_by_state(state: &AppState, theme: Theme) -> Result<(), AppError> {
    let db = state.db.clone();
    let theme_str = theme.as_str().to_string();

    let join_result = tauri::async_runtime::spawn_blocking(move || {
        let guard = db.lock().unwrap();
        guard.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('theme', ?1)",
            [&theme_str],
        )?;
        Ok::<_, AppError>(())
    }).await;

    match join_result {
        Ok(inner) => inner?,
        Err(e) => return Err(AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))),
    }
    Ok(())
}

#[tauri::command]
pub async fn preflight(req: PreflightRequest) -> Result<PreflightResult, AppError> {
    let cs: CustomSpec = serde_json::from_value(req.custom_spec)
        .map_err(|e| AppError::Catalog(format!("Invalid custom_spec: {e}")))?;

    let resolved = resolve_custom(&cs)?;

    let adapter = adapter_for(resolved.protocol);
    adapter.validate(&resolved, &req.api_key).await.map_err(|e| match e {
        ValidateError::InvalidKey => AppError::Http("Invalid API key".into()),
        ValidateError::Ambiguous => AppError::Http("Ambiguous response".into()),
        ValidateError::Network(msg) => AppError::Http(msg),
    })?;

    Ok(PreflightResult {
        ok: true,
        message: "连接成功 (HTTP 200)".into(),
    })
}

#[cfg(test)]
mod preflight_tests {
    use super::*;
    use crate::catalog::{CustomSpec, ProtocolId};

    /// Verifies that the dialog payload (only protocol + base_url + auth_header + notes)
    /// resolves successfully through resolve_custom without requiring validate/quota probes.
    /// This guards the Phase 3 happy path against oracle finding #1.
    #[test]
    fn resolve_custom_accepts_dialog_payload_shape() {
        let cs = CustomSpec {
            protocol: ProtocolId::Openai,
            base_url: Some("https://proxy.example.com/v1".into()),
            auth_header: Some("Authorization: Bearer {api_key}".into()),
            validate: None,
            quota: None,
            notes: Some("test".into()),
        };
        let resolved = resolve_custom(&cs).expect("resolve_custom accepts dialog payload");
        assert_eq!(resolved.protocol, ProtocolId::Openai);
        assert_eq!(resolved.base_url, "https://proxy.example.com/v1");
        // Defaults filled in by resolve_custom
        assert_eq!(resolved.validate_probe.path, "/models");
    }

    /// Verifies the JsonValue → CustomSpec conversion accepts the dialog's exact JSON shape.
    /// This is the most likely failure point in the happy path (serde rename_all on ProtocolId).
    #[test]
    fn dialog_json_payload_deserializes_into_custom_spec() {
        let dialog_json = serde_json::json!({
            "protocol": "openai",
            "base_url": "https://proxy.example.com/v1",
            "auth_header": "Authorization: Bearer {api_key}",
            "notes": "company proxy"
        });
        let cs: CustomSpec = serde_json::from_value(dialog_json)
            .expect("dialog payload deserializes");
        assert_eq!(cs.protocol, ProtocolId::Openai);
        assert_eq!(cs.base_url.as_deref(), Some("https://proxy.example.com/v1"));
    }
}

#[tauri::command]
pub fn list_catalog_presets() -> Vec<crate::catalog::CatalogPresetMeta> {
    crate::catalog::list_catalog_presets()
}
