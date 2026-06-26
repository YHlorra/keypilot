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
    DailySeries, ImportError, ImportResult, TokenUsageRecord, UsageFilter,
    UsageRecordInput, UsageSummary, UsageSummaryAgentPair,
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

fn deterministic_id(agent: &str, model: &str, occurred_at: i64, input: i64, output: i64) -> String {
    let key = format!("{agent}|{model}|{occurred_at}|{input}|{output}");
    format!("{:016x}", fnv1a_64(key.as_bytes()))
}

fn iso_date(epoch: i64) -> String {
    chrono::DateTime::from_timestamp(epoch, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string())
}

// ---------- JSONL row shapes ----------

#[derive(Debug, Deserialize)]
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

// ---------- Service ----------

pub struct TokenUsageService {
    db: Arc<Mutex<Database>>,
    pricing: Arc<PricingService>,
}

impl TokenUsageService {
    pub fn new(db: Arc<Mutex<Database>>, pricing: Arc<PricingService>) -> Self {
        Self { db, pricing }
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
            + input.cache_creation_input_tokens
            + input.reasoning_tokens;

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
            agent_type: input.agent_type.clone(),
            model: input.model.clone(),
            provider_name: input.provider_name.clone(),
            occurred_at: input.occurred_at,
            recorded_at: chrono::Utc::now().timestamp(),
            session_id: input.session_id.clone(),
            request_id: input.request_id.clone(),
            prompt_tokens: input.input_tokens,
            completion_tokens: input.output_tokens,
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

        // Collect distinct dates in range from daily_agent_model_usage
        let (date_clause, date_params): (String, Vec<String>) = if filter.date_from.is_some() || filter.date_to.is_some() {
            let mut s = String::from(" WHERE 1=1");
            let mut p = Vec::new();
            if let Some(from) = filter.date_from {
                s.push_str(&format!(" AND date >= '{}'", iso_date(from)));
                p.push(iso_date(from));
            }
            if let Some(to) = filter.date_to {
                s.push_str(&format!(" AND date <= '{}'", iso_date(to)));
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
            "SELECT agent_type, model, SUM(request_count), SUM(input_tokens), SUM(output_tokens),
             SUM(total_tokens), SUM(total_cost)
             FROM daily_agent_model_usage{date_clause}
             GROUP BY agent_type, model ORDER BY SUM(total_tokens) DESC LIMIT 10"
        )).map_err(AppError::Database)?;
        let pair_rows = pair_stmt
            .query_map(rusqlite::params_from_iter(date_params.iter()), |row| {
                Ok(UsageSummaryAgentPair {
                    agent_type: row.get(0)?,
                    model: row.get(1)?,
                    request_count: row.get::<_, i64>(2)?,
                    input_tokens: row.get::<_, i64>(3)?,
                    output_tokens: row.get::<_, i64>(4)?,
                    total_tokens: row.get::<_, i64>(5)?,
                    total_cost: row.get::<_, f64>(6)?,
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
                    let id = deterministic_id(
                        &input.agent_type,
                        &input.model,
                        input.occurred_at,
                        input.input_tokens,
                        input.output_tokens,
                    );
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
                    let id = deterministic_id(
                        &input.agent_type,
                        &input.model,
                        input.occurred_at,
                        input.input_tokens,
                        input.output_tokens,
                    );
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
            .timestamp();
        let end = start + 86400;

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
        let input = make_input("gpt-4o", 1000, 500, 1700000000);
        let rec = svc.record_usage("rec-1", input).unwrap();
        assert_eq!(rec.id, "rec-1");
        assert_eq!(rec.input_tokens, 1000);
        assert_eq!(rec.output_tokens, 500);
        assert!(rec.total_cost > 0.0);
    }

    #[test]
    fn record_usage_duplicate() {
        let svc = make_service();
        let input = make_input("gpt-4o", 1000, 500, 1700000000);
        svc.record_usage("rec-dup", input.clone()).unwrap();
        let err = svc.record_usage("rec-dup", input).unwrap_err();
        assert!(matches!(err, AppError::TokenUsageDuplicate(_)));
    }

    #[test]
    fn cost_calculation_known_model() {
        let svc = make_service();
        // gpt-4o: input $2.50/1M, output $10.00/1M, cache_read $1.25/1M, cache_creation $2.50/1M
        let input = make_input("gpt-4o", 1_000_000, 1_000_000, 1700000000);
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
        let input = make_input("unknown-model-xyz", 1000, 500, 1700000000);
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
        let mut input = make_input("gpt-4o", 0, 0, 1700000000);
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
        let mut input = make_input("gpt-4o", 0, 0, 1700000000);
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
        svc.record_usage("a", make_input("gpt-4o", 100, 50, 1700000000)).unwrap();
        svc.record_usage("b", make_input("gpt-4o", 100, 50, 1700100000)).unwrap();
        svc.record_usage("c", make_input("gpt-4o", 100, 50, 1700200000)).unwrap();

        let filter = UsageFilter {
            date_from: Some(1700050000),
            date_to: Some(1700150000),
            ..Default::default()
        };
        let recs = svc.list_records(filter, 1, 10).unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].id, "b");
    }

    #[test]
    fn list_records_filtered_by_agent() {
        let svc = make_service();
        let mut i1 = make_input("gpt-4o", 100, 50, 1700000000);
        i1.agent_type = "claude-code".into();
        svc.record_usage("x", i1).unwrap();

        let mut i2 = make_input("gpt-4o", 100, 50, 1700000000);
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
        svc.record_usage("s1", make_input("gpt-4o", 1000, 500, 1700000000)).unwrap();
        svc.record_usage("s2", make_input("gpt-4o", 2000, 1000, 1700003600)).unwrap(); // +1h, same day
        let summary = svc.get_summary(UsageFilter::default()).unwrap();
        assert_eq!(summary.total_requests, 2);
        assert_eq!(summary.total_tokens, 4500);
        assert!(summary.total_cost > 0.0);
        assert_eq!(summary.agent_pairs.len(), 1);
    }

    #[test]
    fn import_jsonl_claude_format() {
        let svc = make_service();
        let jsonl = r#"{"agent":"claude-code","model":"gpt-4o","timestamp":1700000000,"usage":{"input_tokens":100,"output_tokens":50}}
{"agent":"claude-code","model":"gpt-4o","timestamp":1700003600,"usage":{"input_tokens":200,"output_tokens":100}}
"#;
        let r = svc.import_jsonl(jsonl, None).unwrap();
        assert_eq!(r.imported, 2);
        assert_eq!(r.skipped, 0);
        assert_eq!(r.errors.len(), 0);
    }

    #[test]
    fn import_jsonl_codex_format() {
        let svc = make_service();
        let jsonl = r#"{"agent":"codex","model":"gpt-4o","timestamp":1700000000,"usage":{"prompt_tokens":300,"completion_tokens":150}}
"#;
        let r = svc.import_jsonl(jsonl, None).unwrap();
        assert_eq!(r.imported, 1);
        assert_eq!(r.errors.len(), 0);
    }

    #[test]
    fn import_jsonl_dedup() {
        let svc = make_service();
        let jsonl = r#"{"agent":"claude-code","model":"gpt-4o","timestamp":1700000000,"usage":{"input_tokens":100,"output_tokens":50}}
{"agent":"claude-code","model":"gpt-4o","timestamp":1700000000,"usage":{"input_tokens":100,"output_tokens":50}}
"#;
        let r = svc.import_jsonl(jsonl, None).unwrap();
        assert_eq!(r.imported, 1);
        assert_eq!(r.skipped, 1);
    }

    #[test]
    fn import_csv_basic() {
        let svc = make_service();
        let csv = "timestamp,agent_type,model,provider_name,input_tokens,output_tokens\n1700000000,claude-code,gpt-4o,openai,500,250\n1700003600,claude-code,gpt-4o,openai,700,300\n";
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
        svc.record_usage("r1", make_input("gpt-4o", 1000, 500, 1700000000)).unwrap();
        svc.record_usage("r2", make_input("gpt-4o", 2000, 1000, 1700000000)).unwrap();
        let date = "2023-11-14"; // 1700000000 epoch

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
}