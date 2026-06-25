use crate::error::AppError;
use crate::provider::{adapter_for, QuotaError};
use crate::store::AppState;
use crate::types::QuotaSnapshot;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const QUOTA_CACHE_TTL_SECS: i64 = 900; // 15 minutes (REQ-QUOTA-DISPLAY-001)

/// Fetch quota for a provider with 15-minute TTL cache.
/// On cache miss, calls the provider adapter and upserts result into quota_cache.
#[tauri::command]
pub async fn fetch_quota(
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<QuotaSnapshot, AppError> {
    // Phase A: read provider fields + check cache (sync SQLite ops, short lock)
    let (preset, base_url, api_key, cached) = {
        let db = state.db.lock().unwrap();

        // Get provider preset
        let preset: Option<String> = db
            .conn
            .prepare("SELECT preset FROM providers WHERE id = ?1")
            .and_then(|mut stmt| {
                stmt.query_row([id], |row| row.get::<_, Option<String>>(0))
            })
            .ok()
            .flatten();

        let preset = preset.ok_or_else(|| AppError::ProviderNotFound(id))?;

        // Get provider fields
        let mut field_stmt = db
            .conn
            .prepare("SELECT key, value FROM provider_fields WHERE provider_id = ?1")?;
        let mut field_rows: Vec<(String, String)> = Vec::new();
        let mut rows = field_stmt.query([id])?;
        while let Some(row) = rows.next()? {
            let k: String = row.get(0)?;
            let v: String = row.get(1)?;
            field_rows.push((k, v));
        }
        let field_map: HashMap<String, String> = field_rows.into_iter().collect();

        let base_url = field_map
            .get("base_url")
            .cloned()
            .unwrap_or_default();
        let api_key = if preset == "postgres" {
            serde_json::to_string(&field_map).unwrap_or_default()
        } else {
            field_map
                .get("api_key")
                .cloned()
                .unwrap_or_default()
        };

        // Check cache TTL
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let cached: Option<QuotaSnapshot> = db
            .conn
            .query_row(
                "SELECT snapshot_json, fetched_at FROM quota_cache WHERE provider_id = ?1",
                [id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .ok()
            .filter(|(_, fetched_at)| now - fetched_at < QUOTA_CACHE_TTL_SECS)
            .and_then(|(json, _)| serde_json::from_str(&json).ok());

        (preset, base_url, api_key, cached)
    }; // Lock released here

    if let Some(snapshot) = cached {
        return Ok(snapshot);
    }

    // Phase B: fetch from adapter (async HTTP, no spawn_blocking wrap)
    let adapter = adapter_for(&preset).ok_or_else(|| AppError::ProviderQuotaUnsupported(preset.clone()))?;

    if !adapter.can_fetch_quota() {
        return Err(AppError::ProviderQuotaUnsupported(preset));
    }

    let snapshot = adapter
        .fetch_quota(&base_url, &api_key)
        .await
        .map_err(|e| match e {
            QuotaError::Network(msg) => AppError::Http(msg),
            QuotaError::Parse(msg) => AppError::Http(msg),
            QuotaError::Unsupported => AppError::ProviderQuotaUnsupported(preset),
        })?;

    // Phase C: write cache (sync SQLite op, short lock)
    {
        let db = state.db.lock().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let json = serde_json::to_string(&snapshot)?;
        db.conn.execute(
            "INSERT INTO quota_cache (provider_id, snapshot_json, fetched_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(provider_id) DO UPDATE SET snapshot_json = excluded.snapshot_json, fetched_at = excluded.fetched_at",
            rusqlite::params![id, json, now],
        )?;
    }

    Ok(snapshot)
}
