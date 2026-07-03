





use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::services::agent_parser::default_parsers;
use crate::services::token_usage::deterministic_id;
use crate::services::TokenUsageService;
use crate::timeutil;
#[cfg(test)]
use crate::types::UsageRecordInput;



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoImportSummary {
    pub entries: Vec<AgentImportEntry>,
    pub total_imported: u32,
    pub total_skipped: u32,
    pub total_errors: u32,
    pub started_at: i64,
    pub finished_at: i64,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentImportEntry {
    pub agent_type: String,
    pub display_name: String,
    pub path: String,
    pub available: bool,
    
    pub imported: u32,
    
    pub skipped: u32,
    
    pub errors: Vec<String>,
    
    
    pub parse_stats: crate::services::agent_parser::ParseStats,
}





pub fn scan_and_import(svc: &TokenUsageService) -> AutoImportSummary {
    let started_at = timeutil::now_millis();
    let parsers = default_parsers(svc.pricing());
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

        let mut parse_stats = crate::services::agent_parser::ParseStats::empty();
        if available {
            match parser.parse() {
                Ok(outcome) => {
                    parse_stats = outcome.stats;
                    for input in outcome.records {
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
            parse_stats,
        });
    }

    let finished_at = timeutil::now_millis();

    AutoImportSummary {
        entries,
        total_imported,
        total_skipped,
        total_errors,
        started_at,
        finished_at,
    }
}






pub fn scan_and_import_if_empty(svc: &TokenUsageService) -> AutoImportSummary {
    scan_and_import(svc)
}








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
    fn scan_and_import_if_empty_runs_when_under_threshold() {
        let svc = make_svc();
        
        
        
        for i in 0..5 {
            let input = make_input("claude-code", "gpt-4o", 100, 50, 1_700_000_000_000 + i);
            let id = deterministic_id(&input);
            let _ = svc.record_usage(&id, input);
        }
        let result = scan_and_import_if_empty(&svc);
        
        assert_eq!(result.entries.len(), 3);
        
        
    }

    
    
    
    
    
    #[test]
    fn scan_and_import_if_empty_runs_when_db_already_populated() {
        let svc = make_svc();
        
        
        
        for i in 0..3763u64 {
            let input = make_input(
                "claude",
                "fixture",
                100,
                50,
                1_700_000_000_000 + i as i64,
            );
            let id = deterministic_id(&input);
            let _ = svc.record_usage(&id, input);
        }
        let result = scan_and_import_if_empty(&svc);
        
        assert_eq!(result.entries.len(), 3);
        
        let opencode = result.entries.iter().find(|e| e.agent_type == "opencode").unwrap();
        let _ = opencode.parse_stats.files_scanned;
    }

    #[test]
    fn scan_and_import_returns_three_parser_entries() {
        let svc = make_svc();
        let result = scan_and_import(&svc);
        
        assert_eq!(result.entries.len(), 3);
        let opencode = result.entries.iter().find(|e| e.agent_type == "opencode").unwrap();
        let claude = result.entries.iter().find(|e| e.agent_type == "claude-code").unwrap();
        let codex = result.entries.iter().find(|e| e.agent_type == "codex").unwrap();
        
        
        
        
        
        assert!(opencode.parse_stats.files_scanned > 0 || !opencode.available);
        assert_eq!(opencode.parse_stats.lines_parse_errored, 0, "real fixtures must not error");
        
        
        assert!(claude.parse_stats.files_scanned > 0 || !claude.available);
        assert_eq!(claude.parse_stats.lines_parse_errored, 0, "real fixtures must not error");
        
        assert!(codex.parse_stats.files_scanned >= 0);
    }

    #[test]
    fn default_parsers_returns_three() {
        let pricing = std::sync::Arc::new(crate::services::pricing::PricingService::new());
        let parsers = default_parsers(pricing);
        assert_eq!(parsers.len(), 3);
        let types: Vec<&str> = parsers.iter().map(|p| p.agent_type()).collect();
        assert!(types.contains(&"opencode"));
        assert!(types.contains(&"claude-code"));
        assert!(types.contains(&"codex"));
    }
}