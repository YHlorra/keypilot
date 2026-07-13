use crate::catalog::{self, CustomSpec};
use crate::error::AppError;
use crate::provider::registry::{adapter_for, QuotaError};
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
    let (preset, custom_spec_json, api_key, cached, stale_cached) = {
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

        let custom_spec_json: Option<String> = db
            .conn
            .prepare("SELECT custom_spec FROM providers WHERE id = ?1")
            .ok()
            .and_then(|mut stmt| stmt.query_row([id], |row| row.get::<_, Option<String>>(0)).ok().flatten());

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

        let api_key = field_map
            .get("api_key")
            .cloned()
            .unwrap_or_default();

        let now = timeutil::now_secs();

        let cached_row: Option<(String, i64, String)> = db
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
            .ok();

        let cached: Option<QuotaSnapshot> = cached_row
            .as_ref()
            .filter(|(_, fetched_at, source)| {
                source == "manual" || now - fetched_at < QUOTA_CACHE_TTL_SECS
            })
            .and_then(|(json, _, _)| serde_json::from_str::<QuotaSnapshot>(json).ok())
            .filter(|s| !matches!(
                s.status,
                LimitStatus::Unavailable | LimitStatus::Error | LimitStatus::NotConfigured
            ));

        let stale_cached: Option<QuotaSnapshot> = cached_row
            .as_ref()
            .and_then(|(json, _, _)| serde_json::from_str::<QuotaSnapshot>(json).ok())
            .filter(|s| !matches!(
                s.status,
                LimitStatus::Unavailable | LimitStatus::Error | LimitStatus::NotConfigured
            ));

        (preset, custom_spec_json, api_key, cached, stale_cached)
    };

    if let Some(snapshot) = cached {
        return Ok(snapshot);
    }

    // Resolve via catalog (preserves V0.1 base_url field fallback for legacy data)
    let custom_spec: Option<CustomSpec> = custom_spec_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());
    let resolved = catalog::resolve(&preset, custom_spec.as_ref())
        .map_err(|e| AppError::Http(format!("catalog resolve: {e}")))?;

    let adapter = adapter_for(resolved.protocol);
    let snapshot = match adapter.fetch_quota(&resolved, &api_key).await {
        Ok(s) => s,
        Err(QuotaError::Network(_msg)) => {
            if let Some(old) = stale_cached {
                return Ok(old);
            }
            return Ok(QuotaSnapshot {
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
            });
        }
        Err(QuotaError::Parse(_msg)) => {
            if let Some(old) = stale_cached {
                return Ok(old);
            }
            return Ok(QuotaSnapshot {
                total: None,
                used: 0.0,
                remaining: None,
                unit: "USD".to_string(),
                level: None,
                reset_at: None,
                windows: Vec::new(),
                status: LimitStatus::Error,
                source: LimitSource::Api,
                source_detail: "parse error".to_string(),
                account_label: None,
                account_email: None,
                region: None,
                balance: None,
                used_amount: None,
                balance_usd: None,
                used_usd: None,
            });
        }
        Err(QuotaError::Unsupported) => {
            if let Some(old) = stale_cached {
                return Ok(old);
            }
            return Ok(QuotaSnapshot {
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
            });
        }
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
    let (preset, custom_spec_json, api_key, cached, stale_cached) = {
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

        let custom_spec_json: Option<String> = db
            .conn
            .prepare("SELECT custom_spec FROM providers WHERE id = ?1")
            .ok()
            .and_then(|mut stmt| stmt.query_row([id], |row| row.get::<_, Option<String>>(0)).ok().flatten());

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

        let api_key = field_map
            .get("api_key")
            .cloned()
            .unwrap_or_default();

        let now = timeutil::now_secs();
        let cached_row: Option<(String, i64, String)> = db
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
            .ok();
        // Mirror fetch_quota_by_state: only a *successful* snapshot counts as
        // fresh, and keep a last-good copy for fallback on transient failures.
        let cached: Option<SubscriptionQuota> = cached_row
            .as_ref()
            .filter(|(_, fetched_at, source)| {
                *source == "manual" || now - *fetched_at < QUOTA_CACHE_TTL_SECS
            })
            .and_then(|(json, _, _)| serde_json::from_str::<SubscriptionQuota>(json).ok())
            .filter(|s| s.success);
        let stale_cached: Option<SubscriptionQuota> = cached_row
            .as_ref()
            .and_then(|(json, _, _)| serde_json::from_str::<SubscriptionQuota>(json).ok())
            .filter(|s| s.success);

        (preset, custom_spec_json, api_key, cached, stale_cached)
    };

    if let Some(snapshot) = cached {
        return Ok(snapshot);
    }

    // Resolve via catalog
    let custom_spec: Option<CustomSpec> = custom_spec_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());
    let resolved = catalog::resolve(&preset, custom_spec.as_ref())?;

    let vendor = resolved.coding_plan.ok_or_else(|| {
        AppError::ProviderQuotaUnsupported(preset)
    })?;

    let snapshot = crate::provider::coding_plan::registry::fetch(
        vendor,
        &resolved.base_url,
        &api_key,
    )
    .await;

    // Never persist a failed snapshot, and fall back to the last-good cache so a
    // transient network/auth error doesn't get stuck in the tray (and isn't
    // re-served as "cached" on the next refresh within TTL).
    if !snapshot.success {
        if let Some(old) = stale_cached {
            return Ok(old);
        }
        return Ok(snapshot);
    }

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

#[tauri::command]
pub async fn refresh_provider_quota(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<(), AppError> {
    // Read preset + custom_spec to pick the correct quota path.
    let (preset, custom_spec_json) = {
        let db = state.db.lock().unwrap();
        let preset: Option<String> = db
            .conn
            .prepare("SELECT preset FROM providers WHERE id = ?1")
            .and_then(|mut s| s.query_row([id], |r| r.get::<_, Option<String>>(0)))
            .ok()
            .flatten();
        let custom_spec_json: Option<String> = db
            .conn
            .prepare("SELECT custom_spec FROM providers WHERE id = ?1")
            .ok()
            .and_then(|mut s| s.query_row([id], |r| r.get::<_, Option<String>>(0)).ok().flatten());
        (preset, custom_spec_json)
    };

    let has_cp = crate::tray::has_coding_plan(&preset, &custom_spec_json);
    if has_cp {
        let _ = fetch_coding_plan_quota_by_state(&state, id).await;
    } else {
        let _ = fetch_quota_by_state(&state, id).await;
    }

    // Notify tray to refresh asynchronously (don't block IPC return).
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        crate::tray::refresh_and_rebuild(&app_handle).await;
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::AppState;
    use crate::types::subscription::{
        CredentialStatus, QuotaTier, QuotaTierKind, SubscriptionQuota, TierStatus,
    };

    fn setup_state() -> AppState {
        let db = crate::database::Database::open_in_memory().unwrap();
        db.setup_schema().unwrap();
        AppState::new(db)
    }

    fn pinned_coding_plan_provider(state: &AppState, preset: &str) -> i64 {
        let db = state.db.lock().unwrap();
        db.conn
            .execute(
                "INSERT INTO providers (name, preset, is_preset, category_id, pinned, sort_index, created_at, updated_at)
                 VALUES (?1, ?2, 0, 1, 1, 1, 1, 1)",
                [format!("{preset}-test"), preset.to_string()],
            )
            .unwrap();
        db.conn.last_insert_rowid()
    }

    fn valid_coding_plan_snapshot() -> SubscriptionQuota {
        SubscriptionQuota {
            provider_id: "kimi".to_string(),
            credential_status: CredentialStatus::Valid,
            credential_message: None,
            success: true,
            tiers: vec![QuotaTier {
                kind: QuotaTierKind::FiveHour,
                label: "Kimi 5h".to_string(),
                used: Some(10.0),
                limit: Some(100.0),
                used_percent: Some(10.0),
                remaining_percent: Some(90.0),
                resets_at_ms: None,
                reset_description: String::new(),
                status: TierStatus::Active,
            }],
            error: None,
            queried_at_ms: 0,
        }
    }

    // Prove-It for the tray persistence bug: a transient fetch failure must NOT
    // overwrite a previously-good coding_plan cache row, otherwise the tray
    // gets stuck showing the error and the "refresh quota" button becomes a no-op
    // until TTL expires.
    #[tokio::test]
    async fn coding_plan_failure_does_not_poison_cache() {
        let state = setup_state();
        let pid = pinned_coding_plan_provider(&state, "kimi");

        // Seed a *valid* (success=true) snapshot that is already past TTL.
        let now = timeutil::now_secs();
        let valid = valid_coding_plan_snapshot();
        let json = serde_json::to_string(&valid).unwrap();
        {
            let db = state.db.lock().unwrap();
            db.conn
                .execute(
                    "INSERT INTO coding_plan_quota_cache (provider_id, snapshot_json, fetched_at, source)
                     VALUES (?1, ?2, ?3, 'auto')",
                    rusqlite::params![pid, json, now - 2 * QUOTA_CACHE_TTL_SECS],
                )
                .unwrap();
        }

        // The real fetch (no valid API key / offline) deterministically fails
        // with success=false.
        let result = fetch_coding_plan_quota_by_state(&state, pid)
            .await
            .unwrap();

        // Fall back to the last-good cache, not the failure.
        assert!(
            result.success,
            "transient failure must fall back to last-good cached snapshot"
        );

        // The cache row must NOT have been overwritten with the failed snapshot.
        let cached_json: String = state
            .db
            .lock()
            .unwrap()
            .conn
            .query_row(
                "SELECT snapshot_json FROM coding_plan_quota_cache WHERE provider_id = ?1",
                [pid],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            cached_json.contains("\"success\":true"),
            "cache must keep the valid snapshot, got: {cached_json}"
        );
    }
}
