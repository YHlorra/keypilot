use crate::error::AppError;
use crate::provider::{adapter_for, coding_plan_adapter_for, QuotaError};
use crate::store::AppState;
use crate::timeutil;
use crate::types::{LimitSource, LimitStatus, QuotaSnapshot, SubscriptionQuota};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const QUOTA_CACHE_TTL_SECS: i64 = 900; 




#[derive(Debug, Serialize, Deserialize)]
pub struct SetManualQuotaRequest {
    pub id: i64,
    pub snapshot: QuotaSnapshot,
}





#[tauri::command]
pub async fn fetch_quota(
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<QuotaSnapshot, AppError> {
    fetch_quota_by_state(&state, id).await
}

pub async fn fetch_quota_by_state(
    state: &AppState,
    id: i64,
) -> Result<QuotaSnapshot, AppError> {
    
    let (preset, base_url, api_key, cached) = {
        let db = state.db.lock().unwrap();

        
        let preset: Option<String> = db
            .conn
            .prepare("SELECT preset FROM providers WHERE id = ?1")
            .and_then(|mut stmt| {
                stmt.query_row([id], |row| row.get::<_, Option<String>>(0))
            })
            .ok()
            .flatten();

        let preset = preset.ok_or_else(|| AppError::ProviderNotFound(id))?;

        
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
        let api_key = field_map
            .get("api_key")
            .cloned()
            .unwrap_or_default();

        
        let now = timeutil::now_secs();

        let cached: Option<QuotaSnapshot> = db
            .conn
            .query_row(
                "SELECT snapshot_json, fetched_at, source FROM quota_cache WHERE provider_id = ?1",
                [id],
                |row| Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                )),
            )
            .ok()
            .filter(|(_, fetched_at, source)| {
                source == "manual" || now - fetched_at < QUOTA_CACHE_TTL_SECS
            })
            .and_then(|(json, _, _)| serde_json::from_str(&json).ok());

        (preset, base_url, api_key, cached)
    }; 

    if let Some(snapshot) = cached {
        return Ok(snapshot);
    }

    
    let adapter = adapter_for(&preset).ok_or_else(|| AppError::ProviderQuotaUnsupported(preset.clone()))?;

    if !adapter.can_fetch_quota() {
        return Err(AppError::ProviderQuotaUnsupported(preset));
    }

    
    let snapshot = match adapter.fetch_quota(&base_url, &api_key).await {
        Ok(s) => s,
        Err(QuotaError::Network(_msg)) => QuotaSnapshot {
            total: None,
            used: 0.0,
            remaining: None,
            unit: "USD".to_string(),
            level: None,
            reset_at: None,
            windows: Vec::new(),
            status: LimitStatus::Unavailable,
            source: LimitSource::Api,
            source_detail: "app".to_string(),
            account_label: None,
            account_email: None,
            region: None,
            balance: None,
            used_amount: None,
            balance_usd: None,
            used_usd: None,
        },
        Err(QuotaError::Parse(msg)) => QuotaSnapshot {
            total: None,
            used: 0.0,
            remaining: None,
            unit: "USD".to_string(),
            level: None,
            reset_at: None,
            windows: Vec::new(),
            status: LimitStatus::Error,
            source: LimitSource::Api,
            source_detail: format!("parse error: {}", msg),
            account_label: None,
            account_email: None,
            region: None,
            balance: None,
            used_amount: None,
            balance_usd: None,
            used_usd: None,
        },
        Err(QuotaError::Unsupported) => QuotaSnapshot {
            total: None,
            used: 0.0,
            remaining: None,
            unit: "USD".to_string(),
            level: None,
            reset_at: None,
            windows: Vec::new(),
            status: LimitStatus::NotConfigured,
            source: LimitSource::Api,
            source_detail: "unknown".to_string(),
            account_label: None,
            account_email: None,
            region: None,
            balance: None,
            used_amount: None,
            balance_usd: None,
            used_usd: None,
        },
    };

    
    
    {
        let db = state.db.lock().unwrap();
        let now = timeutil::now_secs();
        let json = serde_json::to_string(&snapshot)?;
        db.conn.execute(
            "INSERT INTO quota_cache (provider_id, snapshot_json, fetched_at, source)
             VALUES (?1, ?2, ?3, 'auto')
             ON CONFLICT(provider_id) DO UPDATE SET
                snapshot_json = excluded.snapshot_json,
                fetched_at = excluded.fetched_at,
                source = 'auto'",
            rusqlite::params![id, json, now],
        )?;
    }

    Ok(snapshot)
}




#[tauri::command]
pub async fn set_manual_quota(
    state: tauri::State<'_, AppState>,
    req: SetManualQuotaRequest,
) -> Result<(), AppError> {
    let db = state.db.clone();
    let now = timeutil::now_secs();
    let json = serde_json::to_string(&req.snapshot)?;

    tauri::async_runtime::spawn_blocking(move || {
        let guard = db.lock().unwrap();

        
        
        let exists: bool = guard
            .conn
            .query_row(
                "SELECT 1 FROM providers WHERE id = ?1",
                [req.id],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !exists {
            return Err(AppError::ProviderNotFound(req.id));
        }

        guard.conn.execute(
            "INSERT INTO quota_cache (provider_id, snapshot_json, fetched_at, source)
             VALUES (?1, ?2, ?3, 'manual')
             ON CONFLICT(provider_id) DO UPDATE SET
                snapshot_json = excluded.snapshot_json,
                fetched_at = excluded.fetched_at,
                source = 'manual'",
            rusqlite::params![req.id, json, now],
        )?;
        Ok::<_, AppError>(())
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;

    Ok(())
}


















#[tauri::command]
pub async fn fetch_coding_plan_quota(
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<SubscriptionQuota, AppError> {
    fetch_coding_plan_quota_by_state(&state, id).await
}

pub async fn fetch_coding_plan_quota_by_state(
    state: &AppState,
    id: i64,
) -> Result<SubscriptionQuota, AppError> {
    
    
    
    
    let (_preset, base_url, api_key, cached) = {
        let db = state.db.lock().unwrap();

        let preset: Option<String> = db
            .conn
            .prepare("SELECT preset FROM providers WHERE id = ?1")
            .and_then(|mut stmt| {
                stmt.query_row([id], |row| row.get::<_, Option<String>>(0))
            })
            .ok()
            .flatten();

        let preset = preset.ok_or_else(|| AppError::ProviderNotFound(id))?;

        
        
        
        if coding_plan_adapter_for(&preset).is_none() {
            return Err(AppError::ProviderQuotaUnsupported(preset));
        }

        
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

        let base_url = field_map.get("base_url").cloned().unwrap_or_default();
        let api_key = field_map.get("api_key").cloned().unwrap_or_default();

        
        let now = timeutil::now_secs();
        let cached: Option<SubscriptionQuota> = db
            .conn
            .query_row(
                "SELECT snapshot_json, fetched_at, source FROM coding_plan_quota_cache WHERE provider_id = ?1",
                [id],
                |row| Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                )),
            )
            .ok()
            .filter(|(_, fetched_at, source)| {
                source == "manual" || now - fetched_at < QUOTA_CACHE_TTL_SECS
            })
            .and_then(|(json, _, _)| serde_json::from_str(&json).ok());

        (preset, base_url, api_key, cached)
    }; 

    if let Some(snapshot) = cached {
        return Ok(snapshot);
    }

    
    
    
    let snapshot = crate::provider::coding_plan::fetch_coding_plan_quota(&base_url, &api_key).await;

    
    
    {
        let db = state.db.lock().unwrap();
        let now = timeutil::now_secs();
        let json = serde_json::to_string(&snapshot)?;
        db.conn.execute(
            "INSERT INTO coding_plan_quota_cache (provider_id, snapshot_json, fetched_at, source)
             VALUES (?1, ?2, ?3, 'auto')
             ON CONFLICT(provider_id) DO UPDATE SET
                snapshot_json = excluded.snapshot_json,
                fetched_at = excluded.fetched_at,
                source = 'auto'",
            rusqlite::params![id, json, now],
        )?;
    }

    Ok(snapshot)
}
