use crate::error::AppError;
use crate::store::AppState;
use crate::timeutil;
use crate::types::{Provider, ProviderField, Visibility};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AddProviderRequest {
    pub name: String,
    pub preset: Option<String>,
    pub category_id: i64,
    pub pinned: Option<bool>,
    pub notes: Option<String>,
    pub icon: Option<String>,
    pub icon_color: Option<String>,
    pub fields: Option<Vec<AddProviderFieldRequest>>,
    pub custom_spec: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddProviderFieldRequest {
    pub key: String,
    pub value: String,
    pub visibility: Visibility,
    pub sort_index: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProviderRequest {
    pub id: i64,
    pub name: Option<String>,
    pub category_id: Option<i64>,
    pub pinned: Option<bool>,
    pub notes: Option<String>,
    pub icon: Option<String>,
    pub icon_color: Option<String>,
    pub fields: Option<Vec<UpdateProviderFieldRequest>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProviderFieldRequest {
    pub key: String,
    pub value: String,
    pub visibility: Visibility,
    pub sort_index: i64,
}

fn row_to_provider(row: &rusqlite::Row) -> Result<Provider, rusqlite::Error> {
    Ok(Provider {
        id: row.get(0)?,
        name: row.get(1)?,
        preset: row.get(2)?,
        is_preset: row.get::<_, i64>(3)? != 0,
        category_id: row.get(4)?,
        pinned: row.get::<_, i64>(5)? != 0,
        notes: row.get(6)?,
        icon: row.get(7)?,
        icon_color: row.get(8)?,
        sort_index: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
        fields: Vec::new(),
    })
}

fn row_to_field(row: &rusqlite::Row) -> Result<ProviderField, rusqlite::Error> {
    let visibility_str: String = row.get(4)?;
    Ok(ProviderField {
        id: row.get(0)?,
        provider_id: row.get(1)?,
        key: row.get(2)?,
        value: row.get(3)?,
        visibility: Visibility::parse(&visibility_str).unwrap_or(Visibility::Visible),
        sort_index: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn load_fields(conn: &rusqlite::Connection, provider_id: i64) -> Result<Vec<ProviderField>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, provider_id, key, value, visibility, sort_index, created_at, updated_at
         FROM provider_fields WHERE provider_id = ?1 ORDER BY sort_index"
    )?;
    let fields = stmt
        .query_map([provider_id], row_to_field)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(fields)
}

pub async fn list_providers(state: tauri::State<'_, AppState>) -> Result<Vec<Provider>, AppError> {
    list_providers_by_state(&state).await
}

pub async fn list_providers_by_state(state: &AppState) -> Result<Vec<Provider>, AppError> {
    let db = state.db.clone();
    let providers = tauri::async_runtime::spawn_blocking(move || {
        let guard = db.lock().unwrap();
        let mut stmt = guard.conn.prepare(
            "SELECT id, name, preset, is_preset, category_id, pinned, notes, icon, icon_color,
                    sort_index, created_at, updated_at FROM providers ORDER BY sort_index"
        )?;
        let rows = stmt.query_map([], row_to_provider)?;
        let mut providers: Vec<Provider> = Vec::new();
        for row in rows {
            if let Ok(mut p) = row {
                p.fields = load_fields(&guard.conn, p.id)?;
                providers.push(p);
            }
        }
        Ok::<_, AppError>(providers)
    }).await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;
    Ok(providers)
}

pub async fn get_provider(state: tauri::State<'_, AppState>, id: i64) -> Result<Provider, AppError> {
    get_provider_by_state(&state, id).await
}

pub async fn get_provider_by_state(state: &AppState, id: i64) -> Result<Provider, AppError> {
    let db = state.db.clone();
    let provider = tauri::async_runtime::spawn_blocking(move || {
        let guard = db.lock().unwrap();
        let mut stmt = guard.conn.prepare(
            "SELECT id, name, preset, is_preset, category_id, pinned, notes, icon, icon_color,
                    sort_index, created_at, updated_at FROM providers WHERE id = ?1"
        )?;
        let mut p = stmt.query_row([id], row_to_provider)?;
        p.fields = load_fields(&guard.conn, p.id)?;
        Ok::<_, AppError>(p)
    }).await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;
    Ok(provider)
}

pub async fn add_provider(
    state: tauri::State<'_, AppState>,
    req: AddProviderRequest,
) -> Result<Provider, AppError> {
    add_provider_by_state(&state, req).await
}

pub async fn add_provider_by_state(state: &AppState, req: AddProviderRequest) -> Result<Provider, AppError> {
    let db = state.db.clone();
    let now = timeutil::now_secs();

    // Auto-fill icon from catalog when frontend omitted it (preset id known).
    // Frontend AddCredentialModal only sends preset id; icon column must not stay NULL
    // or ProviderIcon falls back to letter abbreviations.
    let icon = req.icon.clone().or_else(|| {
        req.preset.as_ref().and_then(|p| crate::catalog::preset_icon(p))
    });

    let provider = tauri::async_runtime::spawn_blocking(move || {
        let guard = db.lock().unwrap();
        let pinned = if req.pinned.unwrap_or(false) { 1 } else { 0 };

        guard.conn.execute(
            "INSERT INTO providers (name, preset, is_preset, category_id, pinned, notes, icon,
                                    icon_color, custom_spec, sort_index, created_at, updated_at)
             VALUES (?1, ?2, 0, ?3, ?4, ?5, ?6, ?7,
                     ?8,
                     (SELECT COALESCE(MAX(sort_index), 0) + 1 FROM providers), ?9, ?9)",
            rusqlite::params![
                req.name,
                req.preset,
                req.category_id,
                pinned,
                req.notes,
                icon,
                req.icon_color,
                req.custom_spec,
                now
            ],
        )?;

        let id: i64 = guard.conn.last_insert_rowid();
        let fields = req.fields.unwrap_or_default();
        for field in fields {
            guard.conn.execute(
                "INSERT INTO provider_fields (provider_id, key, value, visibility, sort_index,
                                              created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                rusqlite::params![
                    id,
                    field.key,
                    field.value,
                    field.visibility.as_str(),
                    field.sort_index,
                    now
                ],
            )?;
        }

        let mut stmt = guard.conn.prepare(
            "SELECT id, name, preset, is_preset, category_id, pinned, notes, icon, icon_color,
                    sort_index, created_at, updated_at FROM providers WHERE id = ?1"
        )?;
        let mut p = stmt.query_row([id], row_to_provider)?;
        p.fields = load_fields(&guard.conn, p.id)?;
        Ok::<_, AppError>(p)
    }).await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;

    Ok(provider)
}

pub async fn update_provider(
    state: tauri::State<'_, AppState>,
    req: UpdateProviderRequest,
) -> Result<Provider, AppError> {
    update_provider_by_state(&state, req).await
}

pub async fn update_provider_by_state(state: &AppState, req: UpdateProviderRequest) -> Result<Provider, AppError> {
    let db = state.db.clone();
    let now = timeutil::now_secs();

    let provider = tauri::async_runtime::spawn_blocking(move || {
        let mut guard = db.lock().unwrap();

        
        
        
        let tx = guard.conn.transaction()?;

        
        let exists: bool = tx.query_row(
            "SELECT 1 FROM providers WHERE id = ?1",
            [req.id],
            |_| Ok(true),
        ).unwrap_or(false);
        if !exists {
            return Err(AppError::ProviderNotFound(req.id));
        }

        
        let mut updates = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(name) = &req.name {
            updates.push("name = ?");
            params.push(Box::new(name.clone()));
        }
        if let Some(category_id) = req.category_id {
            updates.push("category_id = ?");
            params.push(Box::new(category_id));
        }
        if let Some(pinned) = req.pinned {
            updates.push("pinned = ?");
            params.push(Box::new(if pinned { 1i64 } else { 0i64 }));
        }
        if let Some(notes) = req.notes {
            updates.push("notes = ?");
            params.push(Box::new(notes));
        }
        if let Some(icon) = req.icon {
            updates.push("icon = ?");
            params.push(Box::new(icon));
        }
        if let Some(icon_color) = req.icon_color {
            updates.push("icon_color = ?");
            params.push(Box::new(icon_color));
        }

        if !updates.is_empty() {
            updates.push("updated_at = ?");
            params.push(Box::new(now));
            params.push(Box::new(req.id));

            let sql = format!(
                "UPDATE providers SET {} WHERE id = ?",
                updates.join(", ")
            );
            tx.execute(&sql, rusqlite::params_from_iter(params.iter()))?;
        }

        
        if let Some(fields) = &req.fields {
            
            tx.execute(
                "DELETE FROM provider_fields WHERE provider_id = ?1",
                [req.id],
            )?;

            
            for field in fields {
                tx.execute(
                    "INSERT INTO provider_fields (provider_id, key, value, visibility, sort_index,
                                                  created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                    rusqlite::params![
                        req.id,
                        field.key,
                        field.value,
                        field.visibility.as_str(),
                        field.sort_index,
                        now
                    ],
                )?;
            }
        }

        tx.commit()?;

        
        let mut stmt = guard.conn.prepare(
            "SELECT id, name, preset, is_preset, category_id, pinned, notes, icon, icon_color,
                    sort_index, created_at, updated_at FROM providers WHERE id = ?1"
        )?;
        let mut p = stmt.query_row([req.id], row_to_provider)?;
        p.fields = load_fields(&guard.conn, p.id)?;
        Ok::<_, AppError>(p)
    }).await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;

    Ok(provider)
}

pub async fn delete_provider(state: tauri::State<'_, AppState>, id: i64) -> Result<(), AppError> {
    delete_provider_by_state(&state, id).await
}

pub async fn delete_provider_by_state(state: &AppState, id: i64) -> Result<(), AppError> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let guard = db.lock().unwrap();
        
        guard.conn.execute("DELETE FROM providers WHERE id = ?1", [id])?;
        Ok::<_, AppError>(())
    }).await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;
    Ok(())
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyCredentialResult {
    pub value: String,
    pub field_key: String,
}

pub async fn copy_credential_by_state(
    state: &AppState,
    id: i64,
    field_key: Option<String>,
) -> Result<CopyCredentialResult, AppError> {
    let provider = get_provider_by_state(state, id).await?;
    
    let resolved = if let Some(k) = field_key {
        provider.fields.iter().find(|f| f.key == k)
    } else {
        provider.fields.iter().find(|f| f.key == "api_key")
    };
    let field = resolved.or_else(|| provider.fields.first())
        .ok_or_else(|| AppError::ProviderNotFound(id))?;
    Ok(CopyCredentialResult {
        value: field.value.clone(),
        field_key: field.key.clone(),
    })
}
