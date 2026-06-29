//! Token Usage IPC handlers (Stage C)
//!
//! Bridges the frontend API contract (webui/src/types/api.ts) to the pure
//! Rust business layer (`services::TokenUsageService` + `services::PricingService`).
//! All handlers are `#[tauri::command]` async and run on Tauri's async runtime.
//! SQLite ops are wrapped in `spawn_blocking` to avoid blocking the event loop.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::AppError;
use crate::services::token_usage::TokenUsageService;
use crate::store::AppState;
use crate::types::{
    UsageFilter as RustUsageFilter,
    UsageRecordInput as RustUsageRecordInput,
    UsageSummary as RustUsageSummary,
    ImportResult as RustImportResult,
    PricingEntry as RustPricingEntry,
    TokenUsageRecord,
    PeriodsSummary,
};

// ---------- IPC DTOs (mirror webui/src/types/api.ts) ----------

#[derive(Debug, Deserialize)]
pub struct RecordUsageRequest {
    pub req: UsageRecordInputIpc,
}

#[derive(Debug, Deserialize)]
pub struct UsageRecordInputIpc {
    pub occurred_at: String,
    pub finished_at: Option<String>,
    pub latency_ms: Option<i64>,
    pub provider: String,
    pub model: String,
    pub agent_type: Option<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub observation_type: Option<String>,
    pub status: Option<String>,
    pub error_code: Option<String>,
    pub cache_hit: Option<i64>,
    pub usage_details: TokenBreakdownIpc,
    pub cost_details: Option<CostBreakdownIpc>,
    pub pricing_version: Option<String>,
    pub messages: Option<String>,
    pub response: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenBreakdownIpc {
    pub input: Option<i64>,
    pub output: Option<i64>,
    pub cache_read: Option<i64>,
    pub cache_creation: Option<i64>,
    pub reasoning: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CostBreakdownIpc {
    pub input: Option<f64>,
    pub output: Option<f64>,
    pub cache_read: Option<f64>,
    pub total: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct ListUsageRecordsRequest {
    pub filter: UsageFilterIpc,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Deserialize, Default)]
pub struct UsageFilterIpc {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub agent_type: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponseIpc<T> {
    pub items: Vec<T>,
    pub total: u32,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageRecordResponse {
    pub id: String,
    pub occurred_at: String,
    pub finished_at: Option<String>,
    pub latency_ms: Option<i64>,
    pub provider: String,
    pub model: String,
    pub agent_type: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub observation_type: Option<String>,
    pub status: Option<String>,
    pub error_code: Option<String>,
    pub cache_hit: Option<i64>,
    pub usage_details: TokenBreakdownIpc,
    pub cost_details: Option<CostBreakdownIpc>,
    pub pricing_version: Option<String>,
    pub messages: Option<String>,
    pub response: Option<String>,
    pub tags: Option<Vec<String>>,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub currency: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageSummaryResponse {
    pub total_tokens: i64,
    pub total_cost_usd: f64,
    pub total_requests: i64,
    pub agent_pairs: Vec<AgentPairResponse>,
    pub daily_series: Vec<DailySeriesResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentPairResponse {
    pub agent_type: String,
    pub model: String,
    pub provider: String,
    pub total_tokens: i64,
    pub total_cost_usd: f64,
    pub request_count: i64,
    pub token_breakdown: TokenBreakdownIpc,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailySeriesResponse {
    pub date: String,
    pub total_tokens: i64,
    pub total_cost_usd: f64,
    pub request_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResultResponse {
    pub imported: u32,
    pub skipped: u32,
    pub errors: Vec<ImportErrorResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportErrorResponse {
    pub line: u32,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PricingEntryResponse {
    pub model: String,
    pub input_cost_per_token: f64,
    pub output_cost_per_token: f64,
    pub cache_read_cost: Option<f64>,
    pub cache_creation_cost: Option<f64>,
    pub supports_reasoning: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecomputeCostsRequest {
    pub from_date: String, // ISO date "YYYY-MM-DD"
    pub to_date: String,   // ISO date "YYYY-MM-DD" (inclusive; +1 day when converting to epoch)
}

#[derive(Debug, Clone, Serialize)]
pub struct RecomputeCostsResponse {
    pub recomputed: u32,
    pub dates_refreshed: u32,
}

// ---------- Conversion helpers ----------

fn iso_to_epoch(iso: &str) -> Result<i64, AppError> {
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|dt| dt.timestamp_millis())
        .map_err(|e| AppError::TokenUsageInvalidFormat(format!("invalid ISO8601 '{iso}': {e}")))
}

/// Parse an ISO calendar date "YYYY-MM-DD" into epoch seconds at 00:00:00 UTC.
///
/// When `exclusive` is true, advance one day so the caller can build a
/// half-open interval `[from_epoch, to_epoch_exclusive)` where `to_date`
/// covers the full calendar day (e.g. "2026-06-28" → 2026-06-29 00:00 UTC).
fn iso_date_to_epoch(date_str: &str, exclusive: bool) -> Result<i64, AppError> {
    let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|e| AppError::TokenUsageInvalidFormat(format!("invalid date '{date_str}': {e}")))?;
    let actual = if exclusive {
        date.succ_opt()
            .ok_or_else(|| AppError::TokenUsageInvalidFormat("date overflow".into()))?
    } else {
        date
    };
    Ok(actual
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp_millis())
}

fn epoch_to_iso(epoch: i64) -> String {
    chrono::DateTime::from_timestamp_millis(epoch)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".into())
}

fn parse_usage_details(json: &Option<String>) -> TokenBreakdownIpc {
    json.as_ref()
        .and_then(|s| serde_json::from_str::<TokenBreakdownIpc>(s).ok())
        .unwrap_or(TokenBreakdownIpc {
            input: None,
            output: None,
            cache_read: None,
            cache_creation: None,
            reasoning: None,
        })
}

fn parse_cost_details(json: &Option<String>) -> Option<CostBreakdownIpc> {
    json.as_ref()
        .and_then(|s| serde_json::from_str::<CostBreakdownIpc>(s).ok())
}

fn ipc_to_rust_filter(ipc: UsageFilterIpc) -> Result<RustUsageFilter, AppError> {
    let date_from = match ipc.start_date {
        Some(ref s) => Some(iso_to_epoch(s)?),
        None => None,
    };
    let date_to = match ipc.end_date {
        Some(ref s) => Some(iso_to_epoch(s)?),
        None => None,
    };
    Ok(RustUsageFilter {
        date_from,
        date_to,
        agent_type: ipc.agent_type,
        model: ipc.model,
        provider_name: ipc.provider,
    })
}

fn rust_record_to_ipc(record: &TokenUsageRecord) -> UsageRecordResponse {
    UsageRecordResponse {
        id: record.id.clone(),
        occurred_at: epoch_to_iso(record.occurred_at),
        finished_at: None,
        latency_ms: None,
        provider: record.provider_name.clone(),
        model: record.model.clone(),
        agent_type: record.agent_type.clone(),
        user_id: None,
        session_id: record.session_id.clone(),
        observation_type: None,
        status: None,
        error_code: None,
        cache_hit: None,
        usage_details: parse_usage_details(&record.usage_details),
        cost_details: parse_cost_details(&record.cost_details),
        pricing_version: record.pricing_version.clone(),
        messages: None,
        response: None,
        tags: None,
        total_tokens: record.total_tokens,
        total_cost: record.total_cost,
        currency: record.currency.clone(),
    }
}

fn rust_summary_to_ipc(summary: RustUsageSummary) -> UsageSummaryResponse {
    UsageSummaryResponse {
        total_tokens: summary.total_tokens,
        total_cost_usd: summary.total_cost,
        total_requests: summary.total_requests,
        agent_pairs: summary.agent_pairs.into_iter().map(|pair| {
            AgentPairResponse {
                agent_type: pair.agent_type,
                model: pair.model,
                provider: pair.provider,
                total_tokens: pair.total_tokens,
                total_cost_usd: pair.total_cost,
                request_count: pair.request_count,
                token_breakdown: TokenBreakdownIpc {
                    input: Some(pair.input_tokens),
                    output: Some(pair.output_tokens),
                    cache_read: None,
                    cache_creation: None,
                    reasoning: None,
                },
            }
        }).collect(),
        daily_series: summary.daily_series.into_iter().map(|ds| {
            DailySeriesResponse {
                date: ds.date,
                total_tokens: ds.total_tokens,
                total_cost_usd: ds.total_cost,
                request_count: ds.request_count,
            }
        }).collect(),
    }
}

fn rust_import_result_to_ipc(result: RustImportResult) -> ImportResultResponse {
    ImportResultResponse {
        imported: result.imported,
        skipped: result.skipped,
        errors: result.errors.into_iter().map(|e| ImportErrorResponse {
            line: e.line,
            message: e.message,
        }).collect(),
    }
}

fn rust_pricing_to_ipc(entries: Vec<&RustPricingEntry>) -> Vec<PricingEntryResponse> {
    entries.into_iter().map(|e| {
        let per_token = |price_per_1m: Option<f64>| price_per_1m.map(|p| p / 1_000_000.0);
        PricingEntryResponse {
            model: e.model.clone(),
            input_cost_per_token: per_token(e.input_price_per_1m).unwrap_or(0.0),
            output_cost_per_token: per_token(e.output_price_per_1m).unwrap_or(0.0),
            cache_read_cost: per_token(e.cache_read_price_per_1m),
            cache_creation_cost: per_token(e.cache_creation_price_per_1m),
            supports_reasoning: e.reasoning_price_per_1m.is_some(),
        }
    }).collect()
}

// ---------- Handlers ----------

/// Record a single usage row (manual entry or proxy forward).
#[tauri::command]
pub async fn record_usage(
    state: State<'_, AppState>,
    req: RecordUsageRequest,
) -> Result<UsageRecordResponse, AppError> {
    record_usage_by_state(&state, req.req).await
}

pub async fn record_usage_by_state(
    state: &AppState,
    input: UsageRecordInputIpc,
) -> Result<UsageRecordResponse, AppError> {
    let occurred_at = iso_to_epoch(&input.occurred_at)?;

    let usage_details_json = serde_json::to_string(&input.usage_details)
        .unwrap_or_else(|_| "{}".into());

    let rust_input = RustUsageRecordInput {
        agent_type: input.agent_type.unwrap_or_else(|| "unknown".into()),
        model: input.model,
        provider_name: input.provider,
        occurred_at,
        session_id: input.session_id,
        request_id: None,
        input_tokens: input.usage_details.input.unwrap_or(0),
        output_tokens: input.usage_details.output.unwrap_or(0),
        cache_read_input_tokens: input.usage_details.cache_read.unwrap_or(0),
        cache_creation_input_tokens: input.usage_details.cache_creation.unwrap_or(0),
        reasoning_tokens: input.usage_details.reasoning.unwrap_or(0),
        usage_details: Some(usage_details_json),
    };

    let id = format!("req-{}", uuid::Uuid::new_v4());
    let svc = TokenUsageService::new(state.db.clone(), state.pricing.clone());
    let record = tauri::async_runtime::spawn_blocking(move || svc.record_usage(&id, rust_input))
        .await
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;

    Ok(rust_record_to_ipc(&record))
}

/// List usage records with pagination and filters.
#[tauri::command]
pub async fn list_usage_records(
    state: State<'_, AppState>,
    req: ListUsageRecordsRequest,
) -> Result<PaginatedResponseIpc<UsageRecordResponse>, AppError> {
    list_usage_records_by_state(&state, req).await
}

pub async fn list_usage_records_by_state(
    state: &AppState,
    req: ListUsageRecordsRequest,
) -> Result<PaginatedResponseIpc<UsageRecordResponse>, AppError> {
    let filter = ipc_to_rust_filter(req.filter)?;
    let page = req.page.max(1);
    let per_page = req.per_page.max(1).min(200);

    let (records, total) = tauri::async_runtime::spawn_blocking({
        let db = state.db.clone();
        move || -> Result<(Vec<TokenUsageRecord>, i64), AppError> {
            let guard = db.lock().map_err(|e| {
                AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;
            let total = guard.count_token_usage_records_filtered(
                filter.agent_type.as_deref(),
                filter.model.as_deref(),
                filter.provider_name.as_deref(),
                filter.date_from,
                filter.date_to,
            )?;
            let records = guard.list_token_usage_records_filtered(
                filter.agent_type.as_deref(),
                filter.model.as_deref(),
                filter.provider_name.as_deref(),
                filter.date_from,
                filter.date_to,
                (page.saturating_sub(1) * per_page) as i64,
                per_page as i64,
            )?;
            Ok((records, total))
        }
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;

    let items = records.into_iter().map(|r| rust_record_to_ipc(&r)).collect();
    Ok(PaginatedResponseIpc {
        items,
        total: total as u32,
        page,
        per_page,
    })
}

/// Get usage summary (agent pairs + daily series).
#[tauri::command]
pub async fn get_usage_summary(
    state: State<'_, AppState>,
    filter: UsageFilterIpc,
) -> Result<UsageSummaryResponse, AppError> {
    get_usage_summary_by_state(&state, filter).await
}

pub async fn get_usage_summary_by_state(
    state: &AppState,
    filter: UsageFilterIpc,
) -> Result<UsageSummaryResponse, AppError> {
    let rust_filter = ipc_to_rust_filter(filter)?;
    let svc = TokenUsageService::new(state.db.clone(), state.pricing.clone());
    let summary = tauri::async_runtime::spawn_blocking(move || svc.get_summary(rust_filter))
        .await
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;
    Ok(rust_summary_to_ipc(summary))
}

/// Batch import usage from JSONL or CSV content.
#[tauri::command]
pub async fn import_usage(
    state: State<'_, AppState>,
    content: String,
    format: String,
    source_hint: Option<String>,
) -> Result<ImportResultResponse, AppError> {
    import_usage_by_state(&state, content, format, source_hint).await
}

pub async fn import_usage_by_state(
    state: &AppState,
    content: String,
    format: String,
    source_hint: Option<String>,
) -> Result<ImportResultResponse, AppError> {
    let svc = TokenUsageService::new(state.db.clone(), state.pricing.clone());
    let result = tauri::async_runtime::spawn_blocking(move || {
        match format.as_str() {
            "jsonl" => svc.import_jsonl(&content, source_hint.as_deref()),
            "csv" => svc.import_csv(&content),
            _ => Err(AppError::TokenUsageInvalidFormat(format!("unknown format '{format}'"))),
        }
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;
    Ok(rust_import_result_to_ipc(result))
}

/// Return the full pricing table (Top 50 models) in frontend format.
#[tauri::command]
pub async fn get_pricing(
    state: State<'_, AppState>,
) -> Result<Vec<PricingEntryResponse>, AppError> {
    get_pricing_by_state(&state).await
}

pub async fn get_pricing_by_state(state: &AppState) -> Result<Vec<PricingEntryResponse>, AppError> {
    let entries = state.pricing.all_entries();
    Ok(rust_pricing_to_ipc(entries))
}

// ---------- opencode.db import ----------

#[derive(Debug, Deserialize)]
pub struct ImportOpencodeDbRequest {
    pub db_path: String,
}

#[tauri::command]
pub async fn import_opencode_db(
    state: State<'_, AppState>,
    req: ImportOpencodeDbRequest,
) -> Result<ImportResultResponse, AppError> {
    import_opencode_db_by_state(&state, req).await
}

pub async fn import_opencode_db_by_state(
    state: &AppState,
    req: ImportOpencodeDbRequest,
) -> Result<ImportResultResponse, AppError> {
    let svc = TokenUsageService::new(state.db.clone(), state.pricing.clone());
    let result = tauri::async_runtime::spawn_blocking(move || {
        svc.import_opencode_db(std::path::Path::new(&req.db_path))
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;
    Ok(rust_import_result_to_ipc(result))
}

/// Return the last auto-import summary JSON (stored in `meta.last_auto_import`).
///
/// Frontend queries this on `App.tsx` mount to decide whether to surface a
/// toast.  This replaces the prior `auto_import_completed` Tauri event which
/// had an emit-before-window race (event fired in `.setup()` before the
/// webview existed; listener dead on arrival).
#[tauri::command]
pub async fn get_last_auto_import(
    state: State<'_, AppState>,
) -> Result<Option<String>, AppError> {
    get_last_auto_import_by_state(&state).await
}

pub async fn get_last_auto_import_by_state(
    state: &AppState,
) -> Result<Option<String>, AppError> {
    let result = tauri::async_runtime::spawn_blocking({
        let db = state.db.clone();
        move || -> Result<Option<String>, AppError> {
            let guard = db.lock().map_err(|e| {
                AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;
            match guard.get_meta("last_auto_import") {
                Ok(v) => Ok(Some(v)),
                Err(AppError::Database(_)) => Ok(None), // key not found
                Err(e) => Err(e),
            }
        }
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;
    Ok(result)
}

/// Recompute cost snapshots for usage records whose `occurred_at` falls within
/// `[from_date 00:00, to_date+1day 00:00)` (UTC).  Both endpoints are
/// inclusive calendar days; `to_date` is advanced by one day internally to
/// form the half-open upper bound required by `TokenUsageService::recompute_costs`.
#[tauri::command]
pub async fn recompute_costs(
    state: State<'_, AppState>,
    req: RecomputeCostsRequest,
) -> Result<RecomputeCostsResponse, AppError> {
    recompute_costs_by_state(&state, req).await
}

pub async fn recompute_costs_by_state(
    state: &AppState,
    req: RecomputeCostsRequest,
) -> Result<RecomputeCostsResponse, AppError> {
    let from_epoch = iso_date_to_epoch(&req.from_date, false)?;
    let to_epoch_exclusive = iso_date_to_epoch(&req.to_date, true)?;

    let svc = TokenUsageService::new(state.db.clone(), state.pricing.clone());
    let result = tauri::async_runtime::spawn_blocking(move || {
        svc.recompute_costs(from_epoch, to_epoch_exclusive)
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;

    Ok(RecomputeCostsResponse {
        recomputed: result.recomputed,
        dates_refreshed: result.dates_refreshed,
    })
}

/// `get_usage_periods_summary` IPC — 三周期 PeriodsSummary 一次性返回。
///
/// 对齐 token-monitor usage.js 主数据契约,前端发 1 次请求拿全
/// today/month/allTime + client_models + limits。
///
/// 接收 `UsageFilterIpc`(与 `get_usage_summary` 一致),内部转 `RustUsageFilter`。
#[tauri::command]
pub async fn get_usage_periods_summary(
    state: State<'_, AppState>,
    filter: UsageFilterIpc,
) -> Result<PeriodsSummary, AppError> {
    get_usage_periods_summary_by_state(&state, filter).await
}

pub async fn get_usage_periods_summary_by_state(
    state: &AppState,
    filter: UsageFilterIpc,
) -> Result<PeriodsSummary, AppError> {
    let rust_filter = ipc_to_rust_filter(filter)?;
    let svc = TokenUsageService::new(state.db.clone(), state.pricing.clone());
    tauri::async_runtime::spawn_blocking(move || svc.get_periods_summary(&rust_filter))
        .await
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?
}
