use crate::error::AppError;
use crate::provider::{adapter_for, ValidateError};
use crate::services::category::{add_category as svc_add_category, delete_category as svc_delete_category, list_categories as svc_list_categories, AddCategoryRequest, DeleteCategoryRequest};
use crate::services::provider::{add_provider as svc_add_provider, delete_provider as svc_delete_provider, get_provider as svc_get_provider, list_providers as svc_list_providers, update_provider as svc_update_provider, AddProviderRequest, UpdateProviderRequest};
use crate::store::AppState;
use crate::types::{Category, Provider, Theme};
use std::collections::HashMap;



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

#[tauri::command]
pub async fn test_connection(
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<(), AppError> {
    test_connection_by_state(&state, id).await
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

        let mut field_stmt = guard.conn.prepare("SELECT key, value FROM provider_fields WHERE provider_id = ?1")?;
        let mut field_rows: Vec<(String, String)> = Vec::new();
        let mut rows = field_stmt.query([id])?;
        while let Some(row) = rows.next()? {
            let k: String = row.get(0)?;
            let v: String = row.get(1)?;
            field_rows.push((k, v));
        }

        Ok::<_, AppError>((preset, field_rows))
    }).await;

    let (preset, fields) = match join_result {
        Ok(inner) => inner?,
        Err(e) => return Err(AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))),
    };

    let preset = preset.ok_or_else(|| AppError::ProviderNotFound(id))?;

    let adapter = adapter_for(&preset).ok_or_else(|| AppError::ProviderCannotTest(preset.clone()))?;

    if !adapter.can_test() {
        return Err(AppError::ProviderCannotTest(preset));
    }

    
    let field_map: HashMap<String, String> = fields.into_iter().collect();
    let base_url = field_map.get("base_url").cloned().unwrap_or_default();
    let api_key = field_map.get("api_key").cloned().unwrap_or_default();

    adapter.validate_key(&base_url, &api_key).await.map_err(|e| match e {
        ValidateError::InvalidKey => AppError::Http("Invalid API key".to_string()),
        ValidateError::Ambiguous => AppError::Http("Ambiguous response".to_string()),
        ValidateError::Network(msg) => AppError::Http(msg),
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
