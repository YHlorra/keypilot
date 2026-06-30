//! TokenUsageService — Stage B
//!
//! Pure Rust business layer. No Tauri/IPC imports. Consumes `Database` +
//! `PricingService` and exposes 6 methods to record, list, summarize, and
//! batch-import token-usage rows.

use std::sync::{Arc, Mutex};

use serde::Deserialize;

use crate::database::Database;
use crate::error::AppError;
use crate::services::pricing::PricingService;
use crate::types::{
    DailySeries, ImportError, ImportResult, LimitProvider, LimitsSummary, PeriodWindow,
    PeriodWindowsPair, PeriodsSummary, PeriodsTriplet, RecomputeResult, TokenUsageRecord,
    UsageFilter, UsageRecordInput, UsageSummary, UsageSummaryAgentPair,
};

// ---------- FNV-1a 64-bit hash (deterministic, no extra dep) ----------

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Deterministic 64-bit ID for dedup.  Includes `provider_name` so that the
/// same model used through different providers (e.g. `claude-sonnet-4` via
/// Anthropic vs AWS Bedrock) is NOT collapsed into one row.
///
/// Breaking change (2026-06-28): existing V0.1 DBs must be re-imported once
/// after upgrade — old rows keyed without provider_name will be re-inserted
/// with new IDs.  This is the single source of truth; `auto_import.rs` calls
/// this function instead of maintaining its own FNV-1a copy.
pub fn deterministic_id(input: &UsageRecordInput) -> String {
    let key = format!(
        "{}|{}|{}|{}|{}|{}",
        input.agent_type,
        input.model,
        input.occurred_at,
        input.input_tokens,
        input.output_tokens,
        input.provider_name
    );
    format!("{:016x}", fnv1a_64(key.as_bytes()))
}

/// Normalize raw agent string to short name (照抄 token-monitor normalizeClientName).
/// "ClaudeCode" / "claude-code" / "claude code" / "CLAUDE" → "claude"
pub fn normalize_agent_type(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("claude") { "claude".to_string() }
    else if lower.contains("codex") { "codex".to_string() }
    else if lower.contains("cursor") { "cursor".to_string() }
    else if lower.contains("gemini") { "gemini".to_string() }
    else if lower.contains("hermes") { "hermes".to_string() }
    else if lower.contains("opencode") { "opencode".to_string() }
    else if lower.contains("aider") { "aider".to_string() }
    else { "unknown".to_string() }
}

fn iso_date(epoch: i64) -> String {
    chrono::DateTime::from_timestamp_millis(epoch)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string())
}

// ---------- JSONL row shapes ----------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClaudeUsage {
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_creation_input_tokens: Option<i64>,
    cache_read_input_tokens: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ClaudeRow {
    #[serde(default)]
    agent: Option<String>,
    model: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    timestamp: Option<i64>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    usage: Option<ClaudeUsage>,
}

#[derive(Debug, Deserialize)]
struct CodexUsage {
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CodexRow {
    #[serde(default)]
    agent: Option<String>,
    model: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    timestamp: Option<i64>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    usage: Option<CodexUsage>,
}

// ---------- CSV row ----------

#[derive(Debug, Default)]
struct CsvColumns {
    timestamp: Option<usize>,
    agent: Option<usize>,
    model: Option<usize>,
    provider: Option<usize>,
    input: Option<usize>,
    output: Option<usize>,
    cache_read: Option<usize>,
    cache_creation: Option<usize>,
    reasoning: Option<usize>,
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            if in_quotes && chars.peek() == Some(&'"') {
                cur.push('"');
                chars.next();
            } else {
                in_quotes = !in_quotes;
            }
        } else if c == ',' && !in_quotes {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    out.push(cur);
    out
}

fn detect_csv_columns(header: &[String]) -> Result<CsvColumns, AppError> {
    let mut cols = CsvColumns::default();
    for (i, name) in header.iter().enumerate() {
        match name.trim().to_lowercase().as_str() {
            "timestamp" | "occurred_at" | "time" => cols.timestamp = Some(i),
            "agent" | "agent_type" => cols.agent = Some(i),
            "model" => cols.model = Some(i),
            "provider" | "provider_name" => cols.provider = Some(i),
            "input_tokens" | "prompt_tokens" => cols.input = Some(i),
            "output_tokens" | "completion_tokens" => cols.output = Some(i),
            "cache_read_input_tokens" | "cache_read_tokens" => cols.cache_read = Some(i),
            "cache_creation_input_tokens" | "cache_creation_tokens" => cols.cache_creation = Some(i),
            "reasoning_tokens" => cols.reasoning = Some(i),
            _ => {}
        }
    }
    if cols.timestamp.is_none() || cols.agent.is_none() || cols.model.is_none()
        || cols.input.is_none() || cols.output.is_none() {
        return Err(AppError::TokenUsageInvalidFormat(
            "CSV header missing required columns (timestamp/agent/model/input_tokens/output_tokens)".into(),
        ));
    }
    Ok(cols)
}

fn get_cell<'a>(row: &'a [String], idx: Option<usize>) -> Option<&'a str> {
    idx.and_then(|i| row.get(i).map(|s| s.as_str()))
}

// ---------- opencode.db row parser (pure — used by import_opencode_db and AgentParser) ----------

/// Parse opencode.db session rows into canonical `UsageRecordInput` records.
/// No DB writes.  Exposed so `AgentParser` implementations can reuse it.
pub fn parse_opencode_db_records(
    db_path: &std::path::Path,
) -> Result<Vec<UsageRecordInput>, AppError> {
    use rusqlite::OpenFlags;
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| AppError::TokenUsageInvalidFormat(format!("open db: {e}")))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, model, cost, tokens_input, tokens_output, \
                    tokens_reasoning, tokens_cache_read, tokens_cache_write, time_created \
             FROM session \
             WHERE (tokens_input > 0 OR tokens_output > 0) \
             ORDER BY time_created ASC",
        )
        .map_err(|e| AppError::TokenUsageInvalidFormat(format!("prepare: {e}")))?;

    let mut rows = stmt
        .query([])
        .map_err(|e| AppError::TokenUsageInvalidFormat(format!("query: {e}")))?;

    let mut records = Vec::new();

    while let Some(row) = rows
        .next()
        .map_err(|e| AppError::TokenUsageInvalidFormat(format!("next: {e}")))?
    {
        let id: String = row.get(0).unwrap_or_default();
        let model_raw: Option<String> = row.get(1).ok();
        let cost: f64 = row.get(2).unwrap_or(0.0);
        let input_tokens: i64 = row.get(3).unwrap_or(0);
        let output_tokens: i64 = row.get(4).unwrap_or(0);
        let reasoning_tokens: i64 = row.get(5).unwrap_or(0);
        let cache_read: i64 = row.get(6).unwrap_or(0);
        let cache_write: i64 = row.get(7).unwrap_or(0);
        let occurred_at: i64 = row.get(8).unwrap_or(0);

        let (provider_name, model) = match model_raw.as_deref() {
            // opencode Go v1.17+ stores `model` as a JSON object:
            //   {"id":"kimi-k2.7-code","providerID":"opencode-go","variant":"max"}
            // Extract `providerID` + `id` so the rest of the pipeline sees a
            // canonical (provider, model) pair like every other parser emits.
            Some(m) if m.starts_with('{') => match serde_json::from_str::<serde_json::Value>(m) {
                Ok(v) => {
                    let provider = v
                        .get("providerID")
                        .and_then(|x| x.as_str())
                        .unwrap_or("opencode")
                        .to_string();
                    let id = v
                        .get("id")
                        .and_then(|x| x.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    (provider, id)
                }
                Err(_) => ("opencode".to_string(), m.to_string()),
            },
            // Legacy `vendor/model` slash convention (older opencode forks).
            Some(m) if m.contains('/') => {
                let mut parts = m.splitn(2, '/');
                (
                    parts.next().unwrap_or("opencode").to_string(),
                    parts.next().unwrap_or(m).to_string(),
                )
            }
            Some(m) => ("opencode".to_string(), m.to_string()),
            None => ("opencode".to_string(), "unknown".to_string()),
        };

        let usage_details = Some(format!(r#"{{"cost_usd":{cost},"source":"opencode"}}"#));

        records.push(UsageRecordInput {
            agent_type: "opencode".to_string(),
            model,
            provider_name,
            occurred_at,
            session_id: Some(id),
            request_id: None,
            input_tokens,
            output_tokens,
            cache_read_input_tokens: cache_read,
            cache_creation_input_tokens: cache_write,
            reasoning_tokens,
            usage_details,
        });
    }

    Ok(records)
}

// ---------- Service ----------

#[derive(Clone)]
pub struct TokenUsageService {
    db: Arc<Mutex<Database>>,
    pricing: Arc<PricingService>,
}

impl TokenUsageService {
    pub fn new(db: Arc<Mutex<Database>>, pricing: Arc<PricingService>) -> Self {
        Self { db, pricing }
    }

    /// Share the underlying `PricingService` with downstream consumers
    /// (e.g. `default_parsers` for `ClaudeCodeParser` provider lookup).
    pub fn pricing(&self) -> Arc<PricingService> {
        self.pricing.clone()
    }

    pub fn count_records(&self) -> Result<u64, AppError> {
        let db = self.db.lock().map_err(|e| {
            AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        })?;
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM token_usage_records", [], |row| row.get(0))
            .map_err(AppError::Database)?;
        Ok(count as u64)
    }

    pub fn record_usage(
        &self,
        id: &str,
        input: UsageRecordInput,
    ) -> Result<TokenUsageRecord, AppError> {
        if id.is_empty() {
            return Err(AppError::TokenUsageInvalidFormat("empty id".into()));
        }

        let cost = self
            .pricing
            .calculate_token_usage_cost(
                &input.model,
                input.input_tokens,
                input.output_tokens,
                input.cache_read_input_tokens,
                input.cache_creation_input_tokens,
                input.reasoning_tokens,
            )?;

        let total_tokens = input.input_tokens
            + input.output_tokens
            + input.cache_read_input_tokens
            + input.cache_creation_input_tokens;

        let agent_type = normalize_agent_type(&input.agent_type);

        let usage_details_json = input.usage_details.clone().unwrap_or_else(|| "{}".into());

        let mut cost_json = serde_json::json!({
            "currency": cost.currency,
            "input": cost.input_cost,
            "output": cost.output_cost,
            "cache_read": cost.cache_read_cost,
            "cache_creation": cost.cache_creation_cost,
            "reasoning": cost.reasoning_cost,
            "total": cost.total_cost,
        });
        if let Some(missing) = cost.pricing_missing_for.as_ref() {
            cost_json["pricing_missing_for"] = serde_json::Value::String(missing.clone());
        }
        let cost_details = Some(cost_json.to_string());

        let record = TokenUsageRecord {
            id: id.to_string(),
            agent_type: agent_type.clone(),
            model: input.model.clone(),
            provider_name: input.provider_name.clone(),
            occurred_at: input.occurred_at,
            recorded_at: chrono::Utc::now().timestamp_millis(),
            session_id: input.session_id.clone(),
            request_id: input.request_id.clone(),
            input_tokens: input.input_tokens,
            output_tokens: input.output_tokens,
            total_tokens,
            cache_read_input_tokens: input.cache_read_input_tokens,
            cache_creation_input_tokens: input.cache_creation_input_tokens,
            reasoning_tokens: input.reasoning_tokens,
            prompt_cost: cost.input_cost,
            completion_cost: cost.output_cost,
            cache_read_cost: cost.cache_read_cost,
            cache_creation_cost: cost.cache_creation_cost,
            reasoning_cost: cost.reasoning_cost,
            total_cost: cost.total_cost,
            currency: cost.currency.clone(),
            pricing_version: Some(self.pricing.version().to_string()),
            usage_details: Some(usage_details_json),
            cost_details,
        };

        let db = self.db.lock().map_err(|e| {
            AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        })?;
        match db.insert_token_usage(&record) {
            Ok(()) => Ok(record),
            Err(AppError::Database(rusqlite::Error::SqliteFailure(err, _)))
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err(AppError::TokenUsageDuplicate(id.to_string()))
            }
            Err(e) => Err(e),
        }
    }

    pub fn list_records(
        &self,
        filter: UsageFilter,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<TokenUsageRecord>, AppError> {
        let offset = (page.saturating_sub(1) as i64) * (per_page as i64);
        let limit = per_page as i64;
        let db = self.db.lock().map_err(|e| {
            AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        })?;
        db.list_token_usage_records_filtered(
            filter.agent_type.as_deref(),
            filter.model.as_deref(),
            filter.provider_name.as_deref(),
            filter.date_from,
            filter.date_to,
            offset,
            limit,
        )
    }

    pub fn get_summary(&self, filter: UsageFilter) -> Result<UsageSummary, AppError> {
        let db = self.db.lock().map_err(|e| {
            AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        })?;

        // Collect distinct dates in range from daily_agent_model_usage.
        // Use ? placeholders so params bind correctly (called by get_periods_summary
        // with date filters — string interpolation + unused params would error in rusqlite).
        let (date_clause, date_params): (String, Vec<String>) = if filter.date_from.is_some() || filter.date_to.is_some() {
            let mut s = String::from(" WHERE 1=1");
            let mut p = Vec::new();
            if let Some(from) = filter.date_from {
                s.push_str(" AND date >= ?");
                p.push(iso_date(from));
            }
            if let Some(to) = filter.date_to {
                s.push_str(" AND date <= ?");
                p.push(iso_date(to));
            }
            (s, p)
        } else {
            (String::new(), Vec::new())
        };

        let conn = db.conn();
        let mut daily_stmt = conn.prepare(&format!(
            "SELECT date, SUM(request_count), SUM(total_tokens), SUM(total_cost)
             FROM daily_agent_model_usage{date_clause}
             GROUP BY date ORDER BY date ASC"
        )).map_err(AppError::Database)?;
        let daily_rows = daily_stmt
            .query_map(rusqlite::params_from_iter(date_params.iter()), |row| {
                Ok(DailySeries {
                    date: row.get(0)?,
                    request_count: row.get::<_, i64>(1)?,
                    total_tokens: row.get::<_, i64>(2)?,
                    total_cost: row.get::<_, f64>(3)?,
                })
            })
            .map_err(AppError::Database)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database)?;

        let mut pair_stmt = conn.prepare(&format!(
            "SELECT agent_type, model, provider, SUM(request_count), SUM(input_tokens), SUM(output_tokens),
             SUM(total_tokens), SUM(total_cost)
             FROM daily_agent_model_usage{date_clause}
             GROUP BY agent_type, model, provider ORDER BY SUM(total_tokens) DESC LIMIT 10"
        )).map_err(AppError::Database)?;
        let pair_rows = pair_stmt
            .query_map(rusqlite::params_from_iter(date_params.iter()), |row| {
                Ok(UsageSummaryAgentPair {
                    agent_type: row.get(0)?,
                    model: row.get(1)?,
                    provider: row.get(2)?,
                    request_count: row.get::<_, i64>(3)?,
                    input_tokens: row.get::<_, i64>(4)?,
                    output_tokens: row.get::<_, i64>(5)?,
                    total_tokens: row.get::<_, i64>(6)?,
                    total_cost: row.get::<_, f64>(7)?,
                })
            })
            .map_err(AppError::Database)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database)?;

        let total_tokens: i64 = daily_rows.iter().map(|d| d.total_tokens).sum();
        let total_cost: f64 = daily_rows.iter().map(|d| d.total_cost).sum();
        let total_requests: i64 = daily_rows.iter().map(|d| d.request_count).sum();

        Ok(UsageSummary {
            total_tokens,
            total_cost,
            total_requests,
            agent_pairs: pair_rows,
            daily_series: daily_rows,
        })
    }

    /// 三周期 PeriodsSummary 主入口(对齐 token-monitor usage.js 主数据契约)。
    ///
    /// 用 `chrono::Local::now()` 算 today / month 边界(本地时区),3 次调
    /// `get_summary`,构造 `period_windows`,再调 `aggregate_client_models`
    /// + `aggregate_limits_summary` 填充五元结构。
    pub fn get_periods_summary(&self, filter: &UsageFilter) -> Result<PeriodsSummary, AppError> {
        use chrono::{Datelike, Local, TimeZone};

        let now = Local::now();
        let today_start = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map(|dt| Local.from_local_datetime(&dt).unwrap())
            .unwrap_or(now);
        let today_end = today_start + chrono::Duration::days(1);

        // Month start: 本月 1 日 00:00
        let month_start = now
            .date_naive()
            .with_day(1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| Local.from_local_datetime(&dt).unwrap())
            .unwrap_or(now);
        // Month end: 下月 1 日 00:00(用 +32 天再 with_day(1) 跨月)
        let month_end = (month_start + chrono::Duration::days(32))
            .date_naive()
            .with_day(1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| Local.from_local_datetime(&dt).unwrap())
            .unwrap_or(now);

        let today_start_ms = today_start.timestamp_millis();
        let today_end_ms = today_end.timestamp_millis();
        let month_start_ms = month_start.timestamp_millis();
        let month_end_ms = month_end.timestamp_millis();

        // 三周期 filter(以传入 filter 为基础,override date_from/date_to)
        let today_filter = UsageFilter {
            date_from: Some(today_start_ms),
            date_to: Some(today_end_ms),
            agent_type: filter.agent_type.clone(),
            model: filter.model.clone(),
            provider_name: filter.provider_name.clone(),
        };
        let month_filter = UsageFilter {
            date_from: Some(month_start_ms),
            date_to: Some(month_end_ms),
            agent_type: filter.agent_type.clone(),
            model: filter.model.clone(),
            provider_name: filter.provider_name.clone(),
        };
        // all_time:不限制 date(用传入 filter,但去掉 date 范围)
        let all_time_filter = UsageFilter {
            date_from: None,
            date_to: None,
            agent_type: filter.agent_type.clone(),
            model: filter.model.clone(),
            provider_name: filter.provider_name.clone(),
        };

        let today_summary = self.get_summary(today_filter)?;
        let month_summary = self.get_summary(month_filter)?;
        let all_time_summary = self.get_summary(all_time_filter)?;

        let period_windows = PeriodWindowsPair {
            today: PeriodWindow {
                key: today_start.format("%Y-%m-%d").to_string(),
                ends_at: today_end.to_rfc3339(),
            },
            month: PeriodWindow {
                key: month_start.format("%Y-%m").to_string(),
                ends_at: month_end.to_rfc3339(),
            },
        };

        let client_models = self.aggregate_client_models(filter)?;
        let limits = self.aggregate_limits_summary()?;

        Ok(PeriodsSummary {
            periods: PeriodsTriplet {
                today: today_summary,
                month: month_summary,
                all_time: all_time_summary,
            },
            period_windows,
            client_models,
            limits,
        })
    }

    /// client × model 二维聚合:agent_type → model → total_tokens
    /// 用 BTreeMap 保证 key 排序稳定(对齐 token-monitor clientModels)。
    ///
    /// 注意:`daily_agent_model_usage` 表没有 cache_read/creation 列,
    /// 所以用 `SUM(total_tokens)`(total_tokens 在 record_usage 时已包含 cache 维度)。
    pub fn aggregate_client_models(
        &self,
        filter: &UsageFilter,
    ) -> Result<std::collections::BTreeMap<String, std::collections::BTreeMap<String, i64>>, AppError>
    {
        use std::collections::BTreeMap;

        let db = self.db.lock().map_err(|e| {
            AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        })?;
        let conn = db.conn();

        // daily_agent_model_usage 无 cache_read/creation 列,total_tokens 已含 cache 维度。
        let mut sql = String::from(
            "SELECT agent_type, model, SUM(total_tokens) AS total
             FROM daily_agent_model_usage WHERE 1=1",
        );
        let mut params: Vec<String> = Vec::new();
        if let Some(from) = filter.date_from {
            sql.push_str(" AND date >= ?");
            params.push(iso_date(from));
        }
        if let Some(to) = filter.date_to {
            sql.push_str(" AND date <= ?");
            params.push(iso_date(to));
        }
        if let Some(ref agent) = filter.agent_type {
            sql.push_str(" AND agent_type = ?");
            params.push(agent.clone());
        }
        if let Some(ref model) = filter.model {
            sql.push_str(" AND model = ?");
            params.push(model.clone());
        }
        sql.push_str(" GROUP BY agent_type, model ORDER BY agent_type, model");

        let mut stmt = conn.prepare(&sql).map_err(AppError::Database)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(AppError::Database)?;

        let mut result: BTreeMap<String, BTreeMap<String, i64>> = BTreeMap::new();
        for row in rows {
            let (agent, model, total) = row.map_err(AppError::Database)?;
            result
                .entry(agent)
                .or_insert_with(BTreeMap::new)
                .insert(model, total);
        }
        Ok(result)
    }

    /// 聚合 quota_cache 全表为 LimitsSummary(对齐 token-monitor aggregateLimits)。
    /// quota_cache 为空时返回 None。
    ///
    /// schema: quota_cache(provider_id PK, snapshot_json, fetched_at, source)
    /// provider_name 通过 LEFT JOIN providers 拿(可能 NULL → "Unknown")。
    pub fn aggregate_limits_summary(&self) -> Result<Option<LimitsSummary>, AppError> {
        let db = self.db.lock().map_err(|e| {
            AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        })?;
        let conn = db.conn();

        let mut stmt = conn
            .prepare(
                "SELECT qc.snapshot_json, qc.fetched_at, p.name AS provider_name
                 FROM quota_cache qc
                 LEFT JOIN providers p ON p.id = qc.provider_id
                 ORDER BY p.name ASC",
            )
            .map_err(AppError::Database)?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(AppError::Database)?;

        let mut providers = Vec::new();
        let mut latest_updated = 0i64;
        for row in rows {
            let (json, fetched_at, name_opt) = row.map_err(AppError::Database)?;
            let provider_name = name_opt.unwrap_or_else(|| "Unknown".to_string());

            // 反序列化为 QuotaSnapshot;损坏 JSON 跳过。
            let snap: crate::types::QuotaSnapshot = match serde_json::from_str(&json) {
                Ok(s) => s,
                Err(_) => continue,
            };

            providers.push(LimitProvider {
                provider: provider_name,
                windows: snap.windows,
                status: snap.status,
                source: snap.source,
                source_detail: snap.source_detail,
                account_label: snap.account_label,
                account_email: snap.account_email,
                region: snap.region,
                balance: snap.balance,
                used_amount: snap.used_amount,
                balance_usd: snap.balance_usd,
                used_usd: snap.used_usd,
            });

            if fetched_at > latest_updated {
                latest_updated = fetched_at;
            }
        }

        if providers.is_empty() {
            Ok(None)
        } else {
            Ok(Some(LimitsSummary {
                providers,
                updated_at: latest_updated,
            }))
        }
    }

    pub fn import_jsonl(
        &self,
        content: &str,
        source_hint: Option<&str>,
    ) -> Result<ImportResult, AppError> {
        let mut imported: u32 = 0;
        let mut skipped: u32 = 0;
        let mut errors: Vec<ImportError> = Vec::new();

        for (idx, raw_line) in content.lines().enumerate() {
            let line_no = (idx as u32) + 1;
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            // Try Claude shape first
            let parsed: Result<UsageRecordInput, AppError> = (|| {
                if let Ok(claude) = serde_json::from_str::<ClaudeRow>(line) {
                    if let Some(usage) = claude.usage {
                        let agent = claude.agent
                            .or_else(|| source_hint.map(|s| s.to_string()))
                            .ok_or_else(|| AppError::TokenUsageInvalidFormat(
                                format!("line {line_no}: missing agent_type"),
                            ))?;
                        let model = claude.model.ok_or_else(|| AppError::TokenUsageInvalidFormat(
                            format!("line {line_no}: missing model"),
                        ))?;
                        let ts = claude.timestamp.ok_or_else(|| AppError::TokenUsageInvalidFormat(
                            format!("line {line_no}: missing timestamp"),
                        ))?;
                        let input = usage.input_tokens.unwrap_or(0);
                        let output = usage.output_tokens.unwrap_or(0);
                        return Ok(UsageRecordInput {
                            agent_type: agent,
                            model,
                            provider_name: claude.provider.unwrap_or_else(|| "unknown".into()),
                            occurred_at: ts,
                            session_id: claude.session_id,
                            request_id: claude.request_id,
                            input_tokens: input,
                            output_tokens: output,
                            cache_read_input_tokens: usage.cache_read_input_tokens.unwrap_or(0),
                            cache_creation_input_tokens: usage.cache_creation_input_tokens.unwrap_or(0),
                            reasoning_tokens: 0,
                            usage_details: Some(line.to_string()),
                        });
                    }
                }
                if let Ok(codex) = serde_json::from_str::<CodexRow>(line) {
                    if let Some(usage) = codex.usage {
                        let agent = codex.agent
                            .or_else(|| source_hint.map(|s| s.to_string()))
                            .ok_or_else(|| AppError::TokenUsageInvalidFormat(
                                format!("line {line_no}: missing agent_type"),
                            ))?;
                        let model = codex.model.ok_or_else(|| AppError::TokenUsageInvalidFormat(
                            format!("line {line_no}: missing model"),
                        ))?;
                        let ts = codex.timestamp.ok_or_else(|| AppError::TokenUsageInvalidFormat(
                            format!("line {line_no}: missing timestamp"),
                        ))?;
                        let input = usage.prompt_tokens.unwrap_or(0);
                        let output = usage.completion_tokens.unwrap_or(0);
                        return Ok(UsageRecordInput {
                            agent_type: agent,
                            model,
                            provider_name: codex.provider.unwrap_or_else(|| "unknown".into()),
                            occurred_at: ts,
                            session_id: codex.session_id,
                            request_id: codex.request_id,
                            input_tokens: input,
                            output_tokens: output,
                            cache_read_input_tokens: 0,
                            cache_creation_input_tokens: 0,
                            reasoning_tokens: 0,
                            usage_details: Some(line.to_string()),
                        });
                    }
                }
                Err(AppError::TokenUsageInvalidFormat(
                    format!("line {line_no}: unrecognised JSONL shape (expected Claude or Codex)"),
                ))
            })();

            match parsed {
                Ok(input) => {
                    let id = deterministic_id(&input);
                    match self.record_usage(&id, input) {
                        Ok(_) => imported += 1,
                        Err(AppError::TokenUsageDuplicate(_)) => skipped += 1,
                        Err(e) => errors.push(ImportError {
                            line: line_no,
                            message: e.to_string(),
                        }),
                    }
                }
                Err(e) => errors.push(ImportError {
                    line: line_no,
                    message: e.to_string(),
                }),
            }
        }

        Ok(ImportResult { imported, skipped, errors })
    }

    pub fn import_csv(&self, content: &str) -> Result<ImportResult, AppError> {
        let mut lines = content.lines();
        let header_line = lines.next().ok_or_else(|| AppError::TokenUsageInvalidFormat(
            "empty CSV (no header)".into(),
        ))?;
        let header = parse_csv_line(header_line);
        let cols = detect_csv_columns(&header)?;

        let mut imported: u32 = 0;
        let mut skipped: u32 = 0;
        let mut errors: Vec<ImportError> = Vec::new();

        for (idx, raw_line) in lines.enumerate() {
            let line_no = (idx as u32) + 2;
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            let row = parse_csv_line(line);

            let get = |k: Option<usize>| -> Result<String, AppError> {
                Ok(get_cell(&row, k)
                    .ok_or_else(|| AppError::TokenUsageInvalidFormat(format!("line {line_no}: missing column")))?
                    .trim()
                    .to_string())
            };

            let parsed: Result<UsageRecordInput, AppError> = (|| {
                let ts_str = get(cols.timestamp)?;
                let ts: i64 = ts_str.parse().map_err(|_| AppError::TokenUsageInvalidFormat(
                    format!("line {line_no}: invalid timestamp '{ts_str}'"),
                ))?;
                let input: i64 = get(cols.input)?.parse().map_err(|_| AppError::TokenUsageInvalidFormat(
                    format!("line {line_no}: invalid input_tokens"),
                ))?;
                let output: i64 = get(cols.output)?.parse().map_err(|_| AppError::TokenUsageInvalidFormat(
                    format!("line {line_no}: invalid output_tokens"),
                ))?;
                let cache_read = cols.cache_read
                    .and_then(|i| row.get(i))
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                let cache_creation = cols.cache_creation
                    .and_then(|i| row.get(i))
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                let reasoning = cols.reasoning
                    .and_then(|i| row.get(i))
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                Ok(UsageRecordInput {
                    agent_type: get(cols.agent)?,
                    model: get(cols.model)?,
                    provider_name: cols.provider
                        .and_then(|i| row.get(i))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|| "unknown".into()),
                    occurred_at: ts,
                    session_id: None,
                    request_id: None,
                    input_tokens: input,
                    output_tokens: output,
                    cache_read_input_tokens: cache_read,
                    cache_creation_input_tokens: cache_creation,
                    reasoning_tokens: reasoning,
                    usage_details: Some(line.to_string()),
                })
            })();

            match parsed {
                Ok(input) => {
                    let id = deterministic_id(&input);
                    match self.record_usage(&id, input) {
                        Ok(_) => imported += 1,
                        Err(AppError::TokenUsageDuplicate(_)) => skipped += 1,
                        Err(e) => errors.push(ImportError {
                            line: line_no,
                            message: e.to_string(),
                        }),
                    }
                }
                Err(e) => errors.push(ImportError {
                    line: line_no,
                    message: e.to_string(),
                }),
            }
        }

        Ok(ImportResult { imported, skipped, errors })
    }

    /// Import token usage from an opencode.db SQLite file (READ ONLY).
    /// Delegates to `parse_opencode_db_records` for the pure row-extraction,
    /// then feeds each row through `record_usage` so FNV-1a dedup applies.
    pub fn import_opencode_db(&self, db_path: &std::path::Path) -> Result<ImportResult, AppError> {
        let records = parse_opencode_db_records(db_path)?;
        let mut imported: u32 = 0;
        let mut skipped: u32 = 0;
        let mut errors: Vec<ImportError> = Vec::new();

        for (idx, input) in records.into_iter().enumerate() {
            let id = input.session_id.clone().unwrap_or_default();
            match self.record_usage(&id, input) {
                Ok(_) => imported += 1,
                Err(AppError::TokenUsageDuplicate(_)) => skipped += 1,
                Err(e) => errors.push(ImportError {
                    line: (idx + 1) as u32,
                    message: e.to_string(),
                }),
            }
        }

        Ok(ImportResult { imported, skipped, errors })
    }

    pub fn refresh_daily_rollups(&self, date: &str) -> Result<(), AppError> {
        let db = self.db.lock().map_err(|e| {
            AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        })?;

        let conn = db.conn();
        let tx = conn.unchecked_transaction().map_err(AppError::Database)?;

        tx.execute(
            "DELETE FROM daily_agent_model_usage WHERE date = ?1",
            [date],
        ).map_err(AppError::Database)?;
        tx.execute(
            "DELETE FROM daily_model_usage WHERE date = ?1",
            [date],
        ).map_err(AppError::Database)?;

        let start = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|e| AppError::TokenUsageInvalidFormat(format!("bad date '{date}': {e}")))?
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        let end = start + 86400 * 1000;

        // Recompute from raw records
        let mut stmt = tx.prepare(
            "SELECT agent_type, model, provider_name, COUNT(*),
             SUM(input_tokens), SUM(output_tokens), SUM(total_tokens), SUM(total_cost)
             FROM token_usage_records
             WHERE occurred_at >= ?1 AND occurred_at < ?2
             GROUP BY agent_type, model, provider_name"
        ).map_err(AppError::Database)?;

        let rows: Vec<(String, String, String, i64, i64, i64, i64, f64)> = stmt
            .query_map([start, end], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, f64>(7)?,
                ))
            })
            .map_err(AppError::Database)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database)?;
        drop(stmt);

        for (agent, model, provider, count, inp, out, total, cost) in rows {
            tx.execute(
                "INSERT INTO daily_agent_model_usage
                 (date, agent_type, model, provider, request_count, input_tokens, output_tokens, total_tokens, total_cost)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![date, agent, model, provider, count, inp, out, total, cost],
            ).map_err(AppError::Database)?;
        }

        let mut stmt2 = tx.prepare(
            "SELECT model, provider_name, COUNT(*),
             SUM(input_tokens), SUM(output_tokens), SUM(total_tokens), SUM(total_cost)
             FROM token_usage_records
             WHERE occurred_at >= ?1 AND occurred_at < ?2
             GROUP BY model, provider_name"
        ).map_err(AppError::Database)?;
        let rows2: Vec<(String, String, i64, i64, i64, i64, f64)> = stmt2
            .query_map([start, end], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, f64>(6)?,
                ))
            })
            .map_err(AppError::Database)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database)?;
        drop(stmt2);

        for (model, provider, count, inp, out, total, cost) in rows2 {
            tx.execute(
                "INSERT INTO daily_model_usage
                 (date, model, provider, request_count, input_tokens, output_tokens, total_tokens, total_cost)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![date, model, provider, count, inp, out, total, cost],
            ).map_err(AppError::Database)?;
        }

        tx.commit().map_err(AppError::Database)?;
        Ok(())
    }

    /// 重算 [from_epoch, to_epoch) 范围内的所有 token_usage_records 的成本字段。
    /// 联动重算受影响日期的 daily_*_usage 汇总表。
    /// 返回 (recomputed 行数, dates_refreshed 日期数)。
    pub fn recompute_costs(&self, from_epoch: i64, to_epoch: i64) -> Result<RecomputeResult, AppError> {
        if from_epoch > to_epoch {
            return Err(AppError::TokenUsageInvalidFormat(
                "from_date must be <= to_date".into(),
            ));
        }

        // 1. 查询所有受影响记录
        let rows: Vec<(String, String, i64, i64, i64, i64, i64, i64)> = {
            let db = self.db.lock().map_err(|e| {
                AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;
            let conn = db.conn();
            let mut stmt = conn.prepare(
                "SELECT id, model, input_tokens, output_tokens, cache_read_input_tokens,
                        cache_creation_input_tokens, reasoning_tokens, occurred_at
                 FROM token_usage_records
                 WHERE occurred_at >= ?1 AND occurred_at < ?2",
            ).map_err(AppError::Database)?;
            let rows = stmt
                .query_map(rusqlite::params![from_epoch, to_epoch], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                    ))
                })
                .map_err(AppError::Database)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(AppError::Database)?;
            rows
        };

        if rows.is_empty() {
            return Ok(RecomputeResult { recomputed: 0, dates_refreshed: 0 });
        }

        let pricing_version = self.pricing.version().to_string();
        let mut recomputed: u32 = 0;
        let mut affected_dates: std::collections::HashSet<String> = std::collections::HashSet::new();

        {
            let db = self.db.lock().map_err(|e| {
                AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;
            let conn = db.conn();
            for (id, model, input, output, cache_read, cache_creation, reasoning, occurred_at) in &rows {
                let cost = self.pricing.calculate_token_usage_cost(
                    model, *input, *output, *cache_read, *cache_creation, *reasoning,
                )?;

                let cost_details = serde_json::json!({
                    "currency": cost.currency,
                    "input": cost.input_cost,
                    "output": cost.output_cost,
                    "cache_read": cost.cache_read_cost,
                    "cache_creation": cost.cache_creation_cost,
                    "reasoning": cost.reasoning_cost,
                    "total": cost.total_cost,
                    "pricing_missing_for": cost.pricing_missing_for,
                })
                .to_string();

                conn.execute(
                    "UPDATE token_usage_records
                     SET prompt_cost = ?1, completion_cost = ?2,
                         cache_read_cost = ?3, cache_creation_cost = ?4,
                         reasoning_cost = ?5,
                         total_cost = ?6,
                         pricing_version = ?7, cost_details = ?8
                     WHERE id = ?9",
                    rusqlite::params![
                        cost.input_cost,
                        cost.output_cost,
                        cost.cache_read_cost,
                        cost.cache_creation_cost,
                        cost.reasoning_cost,
                        cost.total_cost,
                        pricing_version,
                        cost_details,
                        id,
                    ],
                )
                .map_err(AppError::Database)?;
                recomputed += 1;

                // 累计受影响日期(从 occurred_at epoch 秒算 ISO 日期)
                if let Some(dt) = chrono::DateTime::from_timestamp_millis(*occurred_at) {
                    let date_str = dt.format("%Y-%m-%d").to_string();
                    affected_dates.insert(date_str);
                }
            }
        }

        // 重算受影响日期的 daily rollups
        let dates_refreshed = affected_dates.len() as u32;
        for date in &affected_dates {
            self.refresh_daily_rollups(date)?;
        }

        Ok(RecomputeResult { recomputed, dates_refreshed })
    }
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_service() -> TokenUsageService {
        let db = Database::open_in_memory().unwrap();
        db.setup_schema().unwrap();
        db.migrate().unwrap();
        let pricing = Arc::new(PricingService::new());
        TokenUsageService::new(Arc::new(Mutex::new(db)), pricing)
    }

    fn make_input(model: &str, input: i64, output: i64, occurred_at: i64) -> UsageRecordInput {
        UsageRecordInput {
            agent_type: "claude-code".into(),
            model: model.into(),
            provider_name: "test".into(),
            occurred_at,
            session_id: None,
            request_id: None,
            input_tokens: input,
            output_tokens: output,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            reasoning_tokens: 0,
            usage_details: None,
        }
    }

    #[test]
    fn record_usage_happy_path() {
        let svc = make_service();
        let input = make_input("gpt-4o", 1000, 500, 1700000000000);
        let rec = svc.record_usage("rec-1", input).unwrap();
        assert_eq!(rec.id, "rec-1");
        assert_eq!(rec.input_tokens, 1000);
        assert_eq!(rec.output_tokens, 500);
        assert!(rec.total_cost > 0.0);
    }

    #[test]
    fn record_usage_duplicate() {
        let svc = make_service();
        let input = make_input("gpt-4o", 1000, 500, 1700000000000);
        svc.record_usage("rec-dup", input.clone()).unwrap();
        let err = svc.record_usage("rec-dup", input).unwrap_err();
        assert!(matches!(err, AppError::TokenUsageDuplicate(_)));
    }

    #[test]
    fn cost_calculation_known_model() {
        let svc = make_service();
        // gpt-4o: input $2.50/1M, output $10.00/1M, cache_read $1.25/1M, cache_creation $2.50/1M
        let input = make_input("gpt-4o", 1_000_000, 1_000_000, 1700000000000);
        let rec = svc.record_usage("rec-cost", input).unwrap();
        // expected: 2.50 + 10.00 = 12.50
        assert!((rec.total_cost - 12.50).abs() < 0.001);
        assert!((rec.prompt_cost - 2.50).abs() < 0.001);
        assert!((rec.completion_cost - 10.00).abs() < 0.001);
        assert!((rec.cache_read_cost - 0.0).abs() < 0.001);
        assert!((rec.cache_creation_cost - 0.0).abs() < 0.001);
        assert!((rec.reasoning_cost - 0.0).abs() < 0.001);
        // cost_details JSON has all 5 dimensions + currency + total
        let details: serde_json::Value =
            serde_json::from_str(&rec.cost_details.unwrap()).unwrap();
        assert_eq!(details["currency"], "USD");
        assert_eq!(details["input"], 2.50);
        assert_eq!(details["output"], 10.00);
        assert_eq!(details["cache_read"], 0.0);
        assert_eq!(details["cache_creation"], 0.0);
        assert_eq!(details["reasoning"], 0.0);
        assert_eq!(details["total"], 12.50);
        assert!(details.get("pricing_missing_for").is_none());
    }

    #[test]
    fn cost_calculation_unknown_model() {
        let svc = make_service();
        let input = make_input("unknown-model-xyz", 1000, 500, 1700000000000);
        let rec = svc.record_usage("rec-unknown", input).unwrap();
        // All per-dim costs 0, total 0, cost_details still emitted with pricing_missing_for
        assert_eq!(rec.prompt_cost, 0.0);
        assert_eq!(rec.completion_cost, 0.0);
        assert_eq!(rec.cache_read_cost, 0.0);
        assert_eq!(rec.cache_creation_cost, 0.0);
        assert_eq!(rec.reasoning_cost, 0.0);
        assert_eq!(rec.total_cost, 0.0);
        let details: serde_json::Value =
            serde_json::from_str(&rec.cost_details.unwrap()).unwrap();
        assert_eq!(details["pricing_missing_for"], "unknown-model-xyz");
        assert_eq!(details["input"], 0.0);
        assert_eq!(details["total"], 0.0);
    }

    #[test]
    fn cost_calculation_with_cache_read() {
        let svc = make_service();
        // gpt-4o: cache_read $1.25/1M; 1M cache_read tokens = $1.25
        let mut input = make_input("gpt-4o", 0, 0, 1700000000000);
        input.cache_read_input_tokens = 1_000_000;
        let rec = svc.record_usage("rec-cache-read", input).unwrap();
        assert!((rec.cache_read_cost - 1.25).abs() < 0.001);
        assert!((rec.total_cost - 1.25).abs() < 0.001);
        assert_eq!(rec.prompt_cost, 0.0);
        assert_eq!(rec.completion_cost, 0.0);
        assert_eq!(rec.cache_creation_cost, 0.0);
        let details: serde_json::Value =
            serde_json::from_str(&rec.cost_details.unwrap()).unwrap();
        assert_eq!(details["cache_read"], 1.25);
        assert_eq!(details["total"], 1.25);
    }

    #[test]
    fn cost_calculation_with_cache_creation() {
        let svc = make_service();
        // gpt-4o: cache_creation $2.50/1M; 1M cache_creation tokens = $2.50
        let mut input = make_input("gpt-4o", 0, 0, 1700000000000);
        input.cache_creation_input_tokens = 1_000_000;
        let rec = svc.record_usage("rec-cache-create", input).unwrap();
        assert!((rec.cache_creation_cost - 2.50).abs() < 0.001);
        assert!((rec.total_cost - 2.50).abs() < 0.001);
        assert_eq!(rec.cache_read_cost, 0.0);
        let details: serde_json::Value =
            serde_json::from_str(&rec.cost_details.unwrap()).unwrap();
        assert_eq!(details["cache_creation"], 2.50);
        assert_eq!(details["total"], 2.50);
    }

    #[test]
    fn list_records_filtered_by_date() {
        let svc = make_service();
        svc.record_usage("a", make_input("gpt-4o", 100, 50, 1700000000000)).unwrap();
        svc.record_usage("b", make_input("gpt-4o", 100, 50, 1700100000000)).unwrap();
        svc.record_usage("c", make_input("gpt-4o", 100, 50, 1700200000000)).unwrap();

        let filter = UsageFilter {
            date_from: Some(1700050000000),
            date_to: Some(1700150000000),
            ..Default::default()
        };
        let recs = svc.list_records(filter, 1, 10).unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].id, "b");
    }

    #[test]
    fn list_records_filtered_by_agent() {
        let svc = make_service();
        let mut i1 = make_input("gpt-4o", 100, 50, 1700000000000);
        i1.agent_type = "claude-code".into();
        svc.record_usage("x", i1).unwrap();

        let mut i2 = make_input("gpt-4o", 100, 50, 1700000000000);
        i2.agent_type = "codex".into();
        svc.record_usage("y", i2).unwrap();

        let filter = UsageFilter {
            agent_type: Some("codex".into()),
            ..Default::default()
        };
        let recs = svc.list_records(filter, 1, 10).unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].agent_type, "codex");
    }

    #[test]
    fn get_summary_aggregates_correctly() {
        let svc = make_service();
        svc.record_usage("s1", make_input("gpt-4o", 1000, 500, 1700000000000)).unwrap();
        svc.record_usage("s2", make_input("gpt-4o", 2000, 1000, 1700003600000)).unwrap(); // +1h, same day
        let summary = svc.get_summary(UsageFilter::default()).unwrap();
        assert_eq!(summary.total_requests, 2);
        assert_eq!(summary.total_tokens, 4500);
        assert!(summary.total_cost > 0.0);
        assert_eq!(summary.agent_pairs.len(), 1);
    }

    #[test]
    fn get_periods_summary_returns_three_periods() {
        let svc = make_service();

        let now = chrono::Local::now();
        let now_ms = now.timestamp_millis();

        // Today record: right now (unambiguously within today's window).
        let today_input = make_input("gpt-4o", 100, 50, now_ms);
        let _ = svc.record_usage(&deterministic_id(&today_input), today_input);

        // Month record: noon on the 15th (or 1st if today is near the 15th).
        // Pick a day at least 2 days from today so TZ offset (≤14h) can't make
        // the month record's UTC date collide with today's UTC date window.
        use chrono::{Datelike, TimeZone};
        let today_day = now.date_naive().day();
        let pick_day: u32 = if today_day <= 12 { 15 } else { 1 };
        let month_pick = now
            .date_naive()
            .with_day(1)
            .unwrap()
            .with_day(pick_day)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let month_ago_ms = chrono::Local
            .from_local_datetime(&month_pick)
            .unwrap()
            .timestamp_millis();
        let month_input = make_input("gpt-4o", 200, 100, month_ago_ms);
        let _ = svc.record_usage(&deterministic_id(&month_input), month_input);

        // Year record: 400 days ago (definitely outside this month).
        let year_ago_ms = now_ms - 400 * 24 * 3600 * 1000;
        let year_input = make_input("gpt-4o", 1000, 500, year_ago_ms);
        let _ = svc.record_usage(&deterministic_id(&year_input), year_input);

        let filter = UsageFilter::default();
        let summary = svc.get_periods_summary(&filter).unwrap();

        // today 应只有 1 条(150 token)
        assert_eq!(summary.periods.today.total_requests, 1);
        // month 应有 2 条(today + month_ago)
        assert_eq!(summary.periods.month.total_requests, 2);
        // all_time 应有 3 条
        assert_eq!(summary.periods.all_time.total_requests, 3);

        // period_windows.key 格式
        let today_key = chrono::Local::now().format("%Y-%m-%d").to_string();
        assert_eq!(summary.period_windows.today.key, today_key);
        let month_key = chrono::Local::now().format("%Y-%m").to_string();
        assert_eq!(summary.period_windows.month.key, month_key);

        // ends_at 是 ISO 字符串(包含时区)
        assert!(summary.period_windows.today.ends_at.contains("T"));

        // limits 为 None(未注入 quota_cache 数据)
        assert!(summary.limits.is_none());
    }

    #[test]
    fn aggregate_client_models_groups_by_agent_and_model() {
        let svc = make_service();
        let now_ms = chrono::Local::now().timestamp_millis();

        // claude-code + gpt-4o (make_input defaults agent_type to "claude-code",
        // which record_usage normalizes to "claude")
        let i1 = make_input("gpt-4o", 100, 50, now_ms);
        let _ = svc.record_usage(&deterministic_id(&i1), i1);

        // codex + gpt-4o
        let mut i2 = make_input("gpt-4o", 200, 100, now_ms);
        i2.agent_type = "codex".into();
        let _ = svc.record_usage(&deterministic_id(&i2), i2);

        // codex + gpt-4-turbo
        let mut i3 = make_input("gpt-4-turbo", 300, 150, now_ms);
        i3.agent_type = "codex".into();
        let _ = svc.record_usage(&deterministic_id(&i3), i3);

        let filter = UsageFilter::default();
        let result = svc.aggregate_client_models(&filter).unwrap();

        // 应有 2 个 agent_type("claude" 是 "claude-code" 规范化后的结果)
        assert_eq!(result.len(), 2);
        assert!(result.contains_key("claude"));
        assert!(result.contains_key("codex"));

        // codex 应有 2 个 model
        let codex_models = result.get("codex").unwrap();
        assert_eq!(codex_models.len(), 2);
        assert!(codex_models.contains_key("gpt-4o"));
        assert!(codex_models.contains_key("gpt-4-turbo"));

        // claude 的 gpt-4o 总 token = 100 + 50 = 150
        let claude_models = result.get("claude").unwrap();
        assert_eq!(claude_models.get("gpt-4o"), Some(&150));
    }

    #[test]
    fn aggregate_limits_summary_returns_none_when_empty() {
        let svc = make_service();
        let result = svc.aggregate_limits_summary().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn aggregate_limits_summary_handles_usd_and_cny() {
        let svc = make_service();

        // 直接 INSERT 2 条 quota_cache 测试数据
        let openai_snap = serde_json::json!({
            "total": 100.0, "used": 30.0, "remaining": 70.0, "unit": "USD", "level": "green", "reset_at": null,
            "windows": [], "status": "ok", "source": "api", "source_detail": "app",
            "balance_usd": 70.0, "used_usd": 30.0
        })
        .to_string();
        let deepseek_snap = serde_json::json!({
            "total": 200.0, "used": 50.0, "remaining": 150.0, "unit": "CNY", "level": "green", "reset_at": null,
            "windows": [], "status": "ok", "source": "api", "source_detail": "app",
            "balance": {"amount": 150.0, "currency": "CNY"},
            "used_amount": {"amount": 50.0, "currency": "CNY"}
        })
        .to_string();

        {
            let db = svc.db.lock().unwrap();
            db.conn()
                .execute(
                    "INSERT INTO providers (name, preset, is_preset, category_id, pinned, sort_index, created_at, updated_at)
                     VALUES ('OpenAI', 'openai', 1, 1, 0, 1, 0, 0)",
                    [],
                )
                .unwrap();
            db.conn()
                .execute(
                    "INSERT INTO providers (name, preset, is_preset, category_id, pinned, sort_index, created_at, updated_at)
                     VALUES ('DeepSeek', 'deepseek', 1, 1, 0, 2, 0, 0)",
                    [],
                )
                .unwrap();
            db.conn()
                .execute(
                    "INSERT INTO quota_cache (provider_id, snapshot_json, fetched_at, source)
                     VALUES (1, ?1, 1000, 'auto')",
                    rusqlite::params![openai_snap],
                )
                .unwrap();
            db.conn()
                .execute(
                    "INSERT INTO quota_cache (provider_id, snapshot_json, fetched_at, source)
                     VALUES (2, ?1, 2000, 'auto')",
                    rusqlite::params![deepseek_snap],
                )
                .unwrap();
        }

        let result = svc.aggregate_limits_summary().unwrap().unwrap();
        assert_eq!(result.providers.len(), 2);
        // ORDER BY p.name ASC → "DeepSeek" before "OpenAI"
        assert_eq!(result.providers[0].provider, "DeepSeek");
        assert_eq!(result.providers[1].provider, "OpenAI");
        // updated_at = max(fetched_at) = 2000
        assert_eq!(result.updated_at, 2000);
        // OpenAI's balance_usd from JSON
        assert_eq!(result.providers[1].balance_usd, Some(70.0));
        assert_eq!(result.providers[1].used_usd, Some(30.0));
        // DeepSeek's balance (CNY MoneyAmount)
        assert!(result.providers[0].balance.is_some());
        assert_eq!(
            result.providers[0]
                .balance
                .as_ref()
                .unwrap()
                .currency,
            "CNY"
        );
    }

    #[test]
    fn import_jsonl_claude_format() {
        let svc = make_service();
        let jsonl = r#"{"agent":"claude-code","model":"gpt-4o","timestamp":1700000000000,"usage":{"input_tokens":100,"output_tokens":50}}
{"agent":"claude-code","model":"gpt-4o","timestamp":1700003600000,"usage":{"input_tokens":200,"output_tokens":100}}
"#;
        let r = svc.import_jsonl(jsonl, None).unwrap();
        assert_eq!(r.imported, 2);
        assert_eq!(r.skipped, 0);
        assert_eq!(r.errors.len(), 0);
    }

    #[test]
    fn import_jsonl_codex_format() {
        let svc = make_service();
        let jsonl = r#"{"agent":"codex","model":"gpt-4o","timestamp":1700000000000,"usage":{"prompt_tokens":300,"completion_tokens":150}}
"#;
        let r = svc.import_jsonl(jsonl, None).unwrap();
        assert_eq!(r.imported, 1);
        assert_eq!(r.errors.len(), 0);
    }

    #[test]
    fn import_jsonl_codex_format_with_cache() {
        // Codex JSONL only carries prompt_tokens / completion_tokens; the
        // cache_read / cache_creation / reasoning dimensions do not exist
        // in the Codex schema and must be persisted as 0.
        let svc = make_service();
        let jsonl = r#"{"agent":"codex","model":"gpt-4o","timestamp":1700000000000,"usage":{"prompt_tokens":300,"completion_tokens":150}}"#;

        let r = svc.import_jsonl(jsonl, None).unwrap();
        assert_eq!(r.imported, 1, "should import 1 record");
        assert!(r.errors.is_empty(), "errors: {:?}", r.errors);

        let records = svc.list_records(UsageFilter::default(), 1, 10).unwrap();
        assert_eq!(records.len(), 1);
        let rec = &records[0];

        assert_eq!(rec.agent_type, "codex");
        assert_eq!(rec.model, "gpt-4o");
        assert_eq!(rec.input_tokens, 300, "input_tokens should map from prompt_tokens");
        assert_eq!(rec.output_tokens, 150, "output_tokens should map from completion_tokens");
        assert_eq!(rec.cache_read_input_tokens, 0, "cache_read should be 0 for Codex");
        assert_eq!(rec.cache_creation_input_tokens, 0, "cache_creation should be 0 for Codex");
        assert_eq!(rec.reasoning_tokens, 0, "reasoning should be 0 for Codex");
    }

    #[test]
    fn import_jsonl_dedup() {
        let svc = make_service();
        let jsonl = r#"{"agent":"claude-code","model":"gpt-4o","timestamp":1700000000000,"usage":{"input_tokens":100,"output_tokens":50}}
{"agent":"claude-code","model":"gpt-4o","timestamp":1700000000000,"usage":{"input_tokens":100,"output_tokens":50}}
"#;
        let r = svc.import_jsonl(jsonl, None).unwrap();
        assert_eq!(r.imported, 1);
        assert_eq!(r.skipped, 1);
    }

    #[test]
    fn import_csv_basic() {
        let svc = make_service();
        let csv = "timestamp,agent_type,model,provider_name,input_tokens,output_tokens\n1700000000000,claude-code,gpt-4o,openai,500,250\n1700003600000,claude-code,gpt-4o,openai,700,300\n";
        let r = svc.import_csv(csv).unwrap();
        assert_eq!(r.imported, 2);
        assert_eq!(r.errors.len(), 0);
    }

    #[test]
    fn import_csv_bad_header() {
        let svc = make_service();
        let csv = "foo,bar\n1,2\n";
        // Bad header → detect_csv_columns returns Err before any row is processed,
        // so import_csv returns Err directly (not Ok with errors[]).
        let err = svc.import_csv(csv).unwrap_err();
        assert!(matches!(err, AppError::TokenUsageInvalidFormat(_)));
    }

    #[test]
    fn refresh_daily_rollups_recomputes() {
        let svc = make_service();
        svc.record_usage("r1", make_input("gpt-4o", 1000, 500, 1700000000000)).unwrap();
        svc.record_usage("r2", make_input("gpt-4o", 2000, 1000, 1700000000000)).unwrap();
        let date = "2023-11-14"; // 1700000000000 millis

        // First get summary to confirm rollup exists
        let before = svc.get_summary(UsageFilter::default()).unwrap();
        assert_eq!(before.total_requests, 2);

        // Manually corrupt daily_agent_model_usage to force recompute
        {
            let db = svc.db.lock().unwrap();
            db.conn().execute(
                "UPDATE daily_agent_model_usage SET request_count = 999 WHERE date = ?1",
                [date],
            ).unwrap();
        }

        svc.refresh_daily_rollups(date).unwrap();
        let after = svc.get_summary(UsageFilter::default()).unwrap();
        assert_eq!(after.total_requests, 2); // recomputed back
    }

    #[test]
    fn import_opencode_db_basic() {
        use rusqlite::Connection;
        let dir = std::env::temp_dir();
        let path = dir.join(format!("kp_test_opencode_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);

        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE session (
                    id TEXT PRIMARY KEY, model TEXT, cost REAL DEFAULT 0 NOT NULL,
                    tokens_input INTEGER DEFAULT 0 NOT NULL,
                    tokens_output INTEGER DEFAULT 0 NOT NULL,
                    tokens_reasoning INTEGER DEFAULT 0 NOT NULL,
                    tokens_cache_read INTEGER DEFAULT 0 NOT NULL,
                    tokens_cache_write INTEGER DEFAULT 0 NOT NULL,
                    time_created INTEGER NOT NULL
                 );
                 INSERT INTO session VALUES
                   ('s1','minimax-cn-coding-plan/MiniMax-M2.7',0.001,1000,500,0,200,0,1700000000000),
                   ('s2','openai/gpt-4o',0.05,2000,1000,0,0,500,1700000001000),
                   ('s3',NULL,0,0,0,0,0,0,1700000002000);",
            ).unwrap();
        }

        let svc = make_service();
        let r = svc.import_opencode_db(&path).unwrap();
        assert_eq!(r.imported, 2);
        assert_eq!(r.skipped, 0);
        assert!(r.errors.is_empty(), "errors: {:?}", r.errors);

        let summary = svc.get_summary(UsageFilter::default()).unwrap();
        assert!(summary.agent_pairs.iter().any(|p| p.agent_type == "opencode"));
        let _ = std::fs::remove_file(&path);
    }

    /// opencode Go v1.17+ stores `session.model` as a JSON object
    /// `{"id":"kimi-k2.7-code","providerID":"opencode-go","variant":"max"}`
    /// instead of the legacy `vendor/model` slash string.  Verify the parser
    /// unwraps it cleanly: `provider_name` = `providerID`, `model` = `id`.
    #[test]
    fn import_opencode_db_unwraps_json_model() {
        use rusqlite::Connection;
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "kp_test_opencode_json_{}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        let json_model = serde_json::json!({
            "id": "kimi-k2.7-code",
            "providerID": "opencode-go",
            "variant": "max"
        })
        .to_string();
        let json_model_2 = serde_json::json!({
            "id": "MiniMax-M2.7",
            "providerID": "minimax-cn-coding-plan",
            "variant": "default"
        })
        .to_string();

        {
            let conn = Connection::open(&path).unwrap();
            let sql = format!(
                "CREATE TABLE session (
                    id TEXT PRIMARY KEY, model TEXT, cost REAL DEFAULT 0 NOT NULL,
                    tokens_input INTEGER DEFAULT 0 NOT NULL,
                    tokens_output INTEGER DEFAULT 0 NOT NULL,
                    tokens_reasoning INTEGER DEFAULT 0 NOT NULL,
                    tokens_cache_read INTEGER DEFAULT 0 NOT NULL,
                    tokens_cache_write INTEGER DEFAULT 0 NOT NULL,
                    time_created INTEGER NOT NULL
                 );
                 INSERT INTO session VALUES
                   ('json1','{}',0.42,100,50,0,0,0,1700000010000),
                   ('json2','{}',0.0,200,80,0,0,0,1700000020000);",
                json_model, json_model_2
            );
            conn.execute_batch(&sql).unwrap();
        }

        let records = parse_opencode_db_records(&path).unwrap();
        assert_eq!(records.len(), 2);

        // Most-recent first (ORDER BY time_created ASC + len 2 = [json1, json2]).
        let r1 = &records[0];
        assert_eq!(r1.agent_type, "opencode");
        assert_eq!(r1.provider_name, "opencode-go", "providerID from JSON");
        assert_eq!(r1.model, "kimi-k2.7-code", "id from JSON, not the JSON blob");
        assert_eq!(r1.input_tokens, 100);

        let r2 = &records[1];
        assert_eq!(r2.provider_name, "minimax-cn-coding-plan");
        assert_eq!(r2.model, "MiniMax-M2.7");

        let _ = std::fs::remove_file(&path);
    }

    /// Garbage that starts with `{` but is not valid JSON must NOT crash —
    /// fall back to the legacy "treat model as opaque string" path.
    #[test]
    fn import_opencode_db_handles_invalid_json_model() {
        use rusqlite::Connection;
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "kp_test_opencode_badjson_{}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE session (
                    id TEXT PRIMARY KEY, model TEXT, cost REAL DEFAULT 0 NOT NULL,
                    tokens_input INTEGER DEFAULT 0 NOT NULL,
                    tokens_output INTEGER DEFAULT 0 NOT NULL,
                    tokens_reasoning INTEGER DEFAULT 0 NOT NULL,
                    tokens_cache_read INTEGER DEFAULT 0 NOT NULL,
                    tokens_cache_write INTEGER DEFAULT 0 NOT NULL,
                    time_created INTEGER NOT NULL
                 );
                 INSERT INTO session VALUES
                   ('b1','{not valid json',0.01,10,5,0,0,0,1700000030000);",
            ).unwrap();
        }

        let records = parse_opencode_db_records(&path).unwrap();
        assert_eq!(records.len(), 1);
        let r = &records[0];
        // Fallback: provider = "opencode" (legacy default), model = raw string.
        assert_eq!(r.provider_name, "opencode");
        assert_eq!(r.model, "{not valid json");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn import_opencode_db_dedup() {
        use rusqlite::Connection;
        let dir = std::env::temp_dir();
        let path = dir.join(format!("kp_test_opencode_dedup_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);

        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE session (id TEXT PRIMARY KEY, model TEXT, cost REAL DEFAULT 0,
                 tokens_input INT DEFAULT 0, tokens_output INT DEFAULT 0,
                 tokens_reasoning INT DEFAULT 0, tokens_cache_read INT DEFAULT 0,
                 tokens_cache_write INT DEFAULT 0, time_created INT NOT NULL);
                 INSERT INTO session VALUES ('dup1','openai/gpt-4o',0.01,500,250,0,0,0,1700000000000);",
            ).unwrap();
        }

        let svc = make_service();
        let r1 = svc.import_opencode_db(&path).unwrap();
        assert_eq!(r1.imported, 1);
        let r2 = svc.import_opencode_db(&path).unwrap();
        assert_eq!(r2.imported, 0);
        assert_eq!(r2.skipped, 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn import_opencode_db_missing_file() {
        let svc = make_service();
        let r = svc.import_opencode_db(std::path::Path::new("Z:/nonexistent/kp_opencode_xyz.db"));
        assert!(r.is_err());
    }

    #[test]
    fn recompute_costs_invalid_range_returns_error() {
        let svc = make_service();
        let err = svc.recompute_costs(200, 100).unwrap_err();
        match err {
            AppError::TokenUsageInvalidFormat(msg) => {
                assert_eq!(msg, "from_date must be <= to_date");
            }
            other => panic!("expected TokenUsageInvalidFormat, got: {other:?}"),
        }
    }

    #[test]
    fn recompute_costs_updates_unknown_model_pricing_missing_for_null() {
        // Scenario: pricing.json upgrade makes an unknown model known.
        // Before recompute: cost_details.pricing_missing_for = "unknown-model-xyz", costs = 0.
        // After recompute:  cost_details.pricing_missing_for = null, costs > 0.
        use crate::types::PricingEntry;

        let svc = make_service();
        let occurred_at = 1700000000000; // 2023-11-14
        svc.record_usage(
            "rec-recompute-unk",
            make_input("unknown-model-xyz", 1_000_000, 500_000, occurred_at),
        )
        .unwrap();

        // Verify initial state: pricing_missing_for is Some, costs are 0.
        let recs_before = svc.list_records(UsageFilter::default(), 1, 10).unwrap();
        assert_eq!(recs_before.len(), 1);
        let details_before: serde_json::Value =
            serde_json::from_str(recs_before[0].cost_details.as_ref().unwrap()).unwrap();
        assert_eq!(details_before["pricing_missing_for"], "unknown-model-xyz");
        assert_eq!(details_before["total"], 0.0);
        assert_eq!(recs_before[0].total_cost, 0.0);

        // Build a new service sharing the same db but with custom pricing that
        // now recognises "unknown-model-xyz".
        let custom_pricing = PricingService::from_models(vec![PricingEntry {
            model: "unknown-model-xyz".into(),
            provider: "TestProvider".into(),
            input_price_per_1m: Some(2.0),
            output_price_per_1m: Some(8.0),
            cache_read_price_per_1m: None,
            cache_creation_price_per_1m: None,
            reasoning_price_per_1m: None,
        }]);
        let svc2 = TokenUsageService::new(svc.db.clone(), Arc::new(custom_pricing));

        let result = svc2.recompute_costs(occurred_at, occurred_at + 86400 * 1000).unwrap();
        assert_eq!(result.recomputed, 1);
        assert_eq!(result.dates_refreshed, 1);

        // Read back and verify pricing_missing_for is now null, costs populated.
        let recs_after = svc2.list_records(UsageFilter::default(), 1, 10).unwrap();
        assert_eq!(recs_after.len(), 1);
        let details_after: serde_json::Value =
            serde_json::from_str(recs_after[0].cost_details.as_ref().unwrap()).unwrap();
        assert!(
            details_after.get("pricing_missing_for").is_some(),
            "key should still exist (as null)"
        );
        assert!(
            details_after["pricing_missing_for"].is_null(),
            "should be null after recompute, got: {details_after}"
        );
        // 1M input * $2/1M = $2; 500k output * $8/1M = $4; total $6.
        assert!(
            (details_after["total"].as_f64().unwrap() - 6.0).abs() < 0.001,
            "total cost mismatch"
        );
        assert!(
            (recs_after[0].total_cost - 6.0).abs() < 0.001,
            "total_cost column mismatch"
        );
        assert!(
            (recs_after[0].prompt_cost - 2.0).abs() < 0.001,
            "prompt_cost mismatch"
        );
        assert!(
            (recs_after[0].completion_cost - 4.0).abs() < 0.001,
            "completion_cost mismatch"
        );

        // Daily rollups should reflect the new cost.
        let summary = svc2.get_summary(UsageFilter::default()).unwrap();
        assert!((summary.total_cost - 6.0).abs() < 0.001);
    }

    #[test]
    fn recompute_costs_empty_range_returns_zero() {
        // No records in range → recomputed=0, dates_refreshed=0, no error.
        let svc = make_service();
        let result = svc.recompute_costs(100, 100).unwrap();
        assert_eq!(result.recomputed, 0);
        assert_eq!(result.dates_refreshed, 0);
    }

    #[test]
    fn normalize_agent_type_claude_variants() {
        assert_eq!(normalize_agent_type("ClaudeCode"), "claude");
        assert_eq!(normalize_agent_type("claude-code"), "claude");
        assert_eq!(normalize_agent_type("claude code"), "claude");
        assert_eq!(normalize_agent_type("CLAUDE"), "claude");
    }

    #[test]
    fn normalize_agent_type_codex() {
        assert_eq!(normalize_agent_type("codex"), "codex");
        assert_eq!(normalize_agent_type("Codex"), "codex");
        assert_eq!(normalize_agent_type("CODEX-CLI"), "codex");
    }

    #[test]
    fn normalize_agent_type_unknown() {
        assert_eq!(normalize_agent_type("some-new-tool"), "unknown");
    }

    #[test]
    fn normalize_agent_type_empty() {
        assert_eq!(normalize_agent_type(""), "unknown");
    }
}