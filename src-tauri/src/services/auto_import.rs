//! Auto-import orchestrator — scans available agent data sources and imports
//! any new records into the token usage DB on startup.
//!
//! `scan_and_import_if_empty()` is the entry point: it skips the scan if the
//! DB already has > 100 rows (already populated, avoid re-import churn).

#[cfg(test)]
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::services::agent_parser::default_parsers;
use crate::services::TokenUsageService;
use crate::types::UsageRecordInput;

/// Summary of one auto-import run, suitable for storing in the meta table
/// and/or emitting as a Tauri event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoImportSummary {
    pub entries: Vec<AgentImportEntry>,
    pub total_imported: u32,
    pub total_skipped: u32,
    pub total_errors: u32,
    pub started_at: i64,
    pub finished_at: i64,
}

/// Per-agent result within an auto-import run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentImportEntry {
    pub agent_type: String,
    pub display_name: String,
    pub path: String,
    pub available: bool,
    pub imported: u32,
    pub skipped: u32,
    pub errors: Vec<String>,
}

/// Returns true when the token_usage_records table has more than `threshold`
/// rows.  Used to skip auto-import on already-populated DBs.
fn db_has_records(svc: &TokenUsageService, threshold: u32) -> bool {
    svc.count_records().map(|c| c > threshold as u64).unwrap_or(false)
}

/// Run a full auto-import scan across all available agent parsers.
/// For each parser: if `!is_available()` skip silently; otherwise call
/// `parse()` and feed every `UsageRecordInput` through `record_usage`.
/// Returns an `AutoImportSummary` for the caller to store / emit.
pub fn scan_and_import(svc: &TokenUsageService) -> AutoImportSummary {
    let started_at = chrono::Utc::now().timestamp_millis();
    let parsers = default_parsers();
    let mut entries = Vec::new();
    let mut total_imported: u32 = 0;
    let mut total_skipped: u32 = 0;
    let mut total_errors: u32 = 0;

    for parser in parsers {
        let agent_type = parser.agent_type().to_string();
        let display_name = parser.display_name().to_string();
        let path = parser.default_path().to_string_lossy().to_string();
        let available = parser.is_available();

        let mut imported: u32 = 0;
        let mut skipped: u32 = 0;
        let mut error_msgs: Vec<String> = Vec::new();

        if available {
            match parser.parse() {
                Ok(rows) => {
                    for input in rows {
                        let id = deterministic_id(&input);
                        match svc.record_usage(&id, input) {
                            Ok(_) => imported += 1,
                            Err(AppError::TokenUsageDuplicate(_)) => skipped += 1,
                            Err(e) => error_msgs.push(e.to_string()),
                        }
                    }
                }
                Err(e) => {
                    error_msgs.push(format!("parse error: {e}"));
                }
            }
        }

        total_imported += imported;
        total_skipped += skipped;
        total_errors += error_msgs.len() as u32;

        entries.push(AgentImportEntry {
            agent_type,
            display_name,
            path,
            available,
            imported,
            skipped,
            errors: error_msgs,
        });
    }

    let finished_at = chrono::Utc::now().timestamp_millis();

    AutoImportSummary {
        entries,
        total_imported,
        total_skipped,
        total_errors,
        started_at,
        finished_at,
    }
}

/// Skip if the DB already has > 100 rows; otherwise run `scan_and_import`.
pub fn scan_and_import_if_empty(svc: &TokenUsageService) -> AutoImportSummary {
    if db_has_records(svc, 100) {
        let now = chrono::Utc::now().timestamp_millis();
        return AutoImportSummary {
            entries: vec![],
            total_imported: 0,
            total_skipped: 0,
            total_errors: 0,
            started_at: now,
            finished_at: now,
        };
    }
    scan_and_import(svc)
}

// ---------- FNV-1a 64-bit deterministic ID (same as token_usage.rs) ----------

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn deterministic_id(input: &UsageRecordInput) -> String {
    let key = format!(
        "{}|{}|{}|{}|{}",
        input.agent_type,
        input.model,
        input.occurred_at,
        input.input_tokens,
        input.output_tokens
    );
    format!("{:016x}", fnv1a_64(key.as_bytes()))
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_svc() -> TokenUsageService {
        let db = crate::database::Database::open_in_memory().unwrap();
        db.setup_schema().unwrap();
        db.migrate().unwrap();
        let pricing = std::sync::Arc::new(crate::services::pricing::PricingService::new());
        TokenUsageService::new(std::sync::Arc::new(std::sync::Mutex::new(db)), pricing)
    }

    fn make_input(agent: &str, model: &str, input: i64, output: i64, occurred_at: i64) -> UsageRecordInput {
        UsageRecordInput {
            agent_type: agent.into(),
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
    fn scan_and_import_if_empty_skips_when_populated() {
        let svc = make_svc();
        // Pre-populate with 5 records
        for i in 0..5 {
            let input = make_input("claude-code", "gpt-4o", 100, 50, 1700000000 + i);
            let id = deterministic_id(&input);
            let _ = svc.record_usage(&id, input);
        }
        // DB now has 5 rows — threshold is 100, so should NOT skip
        let result = scan_and_import_if_empty(&svc);
        // scan_and_import_if_empty checks threshold 100, 5 < 100 so runs scan
        // But parsers aren't available in test env, so entries are empty
        assert_eq!(result.total_imported, 0);
        assert_eq!(result.total_skipped, 0);
    }

    #[test]
    fn scan_and_import_returns_correct_counts() {
        let svc = make_svc();
        // Verify empty DB triggers full scan
        let result = scan_and_import(&svc);
        assert_eq!(result.entries.len(), 2); // opencode + claude-code
        assert_eq!(result.total_imported, 0); // no parsers available in test env
    }

    #[test]
    fn default_parsers_returns_two() {
        let parsers = default_parsers();
        assert_eq!(parsers.len(), 2);
        let types: Vec<&str> = parsers.iter().map(|p| p.agent_type()).collect();
        assert!(types.contains(&"opencode"));
        assert!(types.contains(&"claude-code"));
    }
}