








use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;

use crate::error::AppError;
use crate::services::agent_parser::{AgentParser, ParseOutcome, ParseStats};
use crate::services::pricing::PricingService;
use crate::types::UsageRecordInput;

pub struct CodexParser {
    path: PathBuf,
    pricing: Arc<PricingService>,
}

impl CodexParser {
    pub fn new(pricing: Arc<PricingService>) -> Self {
        Self {
            path: codex_sessions_dir(),
            pricing,
        }
    }

    #[cfg(test)]
    pub fn with_path(path: PathBuf, pricing: Arc<PricingService>) -> Self {
        Self { path, pricing }
    }
}

fn codex_sessions_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .map(|p| PathBuf::from(p).join(".codex").join("sessions"))
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".codex")
                .join("sessions")
        })
}

impl AgentParser for CodexParser {
    fn agent_type(&self) -> &'static str {
        "codex"
    }

    fn display_name(&self) -> &'static str {
        "Codex"
    }

    fn default_path(&self) -> PathBuf {
        self.path.clone()
    }

    fn is_available(&self) -> bool {
        self.path.is_dir()
    }

    fn parse(&self) -> Result<ParseOutcome, AppError> {
        if !self.is_available() {
            return Ok(ParseOutcome {
                records: vec![],
                stats: ParseStats::empty(),
            });
        }
        let mut records = Vec::new();
        let mut stats = ParseStats::empty();
        self.walk_jsonl_dir(&self.path, &mut records, &mut stats);
        Ok(ParseOutcome { records, stats })
    }
}



impl CodexParser {
    fn walk_jsonl_dir(
        &self,
        dir: &std::path::Path,
        out: &mut Vec<UsageRecordInput>,
        stats: &mut ParseStats,
    ) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                stats.record_error(&dir.to_string_lossy(), 0, &format!("read_dir: {e}"));
                return;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    stats.record_error(&dir.to_string_lossy(), 0, &format!("dir entry: {e}"));
                    continue;
                }
            };
            let p = entry.path();
            if p.is_dir() {
                self.walk_jsonl_dir(&p, out, stats);
            } else if p.extension().map(|e| e == "jsonl").unwrap_or(false) {
                let content = match std::fs::read_to_string(&p) {
                    Ok(c) => c,
                    Err(e) => {
                        stats.record_error(&p.to_string_lossy(), 0, &format!("read: {e}"));
                        continue;
                    }
                };
                self.parse_jsonl_file(&p, &content, out, stats);
            }
        }
    }

    fn parse_jsonl_file(
        &self,
        path: &std::path::Path,
        content: &str,
        out: &mut Vec<UsageRecordInput>,
        stats: &mut ParseStats,
    ) {
        let file_name = path.to_string_lossy().to_string();
        stats.files_scanned += 1;

        for (idx, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            stats.lines_scanned += 1;
            let line_no = idx as u32 + 1;

            let v: Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(e) => {
                    stats.record_error(&file_name, line_no, &format!("json: {e}"));
                    continue;
                }
            };

            match self.parse_session_line(&v, stats, &file_name, line_no) {
                Some(rec) => {
                    stats.lines_matched += 1;
                    out.push(rec);
                }
                None => {}
            }
        }
    }

    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    fn parse_session_line(
        &self,
        v: &Value,
        stats: &mut ParseStats,
        file: &str,
        line_no: u32,
    ) -> Option<UsageRecordInput> {
        
        let usage = match v.get("usage") {
            Some(u) => u,
            None => return None,
        };

        
        let timestamp_secs = match v.get("timestamp").and_then(|t| t.as_i64()) {
            Some(t) => t,
            None => {
                stats.record_error(file, line_no, "missing/invalid top-level timestamp");
                return None;
            }
        };
        let occurred_at = timestamp_secs * 1000;

        let model = match v.get("model").and_then(|m| m.as_str()) {
            Some(m) => m.to_string(),
            None => {
                stats.record_error(file, line_no, "missing top-level model");
                return None;
            }
        };

        let input_tokens = usage
            .get("prompt_tokens")
            .and_then(|x| x.as_i64())
            .unwrap_or(0);
        let output_tokens = usage
            .get("completion_tokens")
            .and_then(|x| x.as_i64())
            .unwrap_or(0);
        let reasoning_tokens = usage
            .get("reasoning_tokens")
            .and_then(|x| x.as_i64())
            .unwrap_or(0);

        
        let cache_read_input_tokens = 0;
        let cache_creation_input_tokens = 0;

        let provider_name = self
            .pricing
            .lookup_provider_by_model(&model)
            .unwrap_or_else(|| "openai".to_string());

        let session_id = v
            .get("session_id")
            .and_then(|x| x.as_str())
            .map(String::from);
        let request_id = v
            .get("request_id")
            .and_then(|x| x.as_str())
            .map(String::from);

        Some(UsageRecordInput {
            agent_type: "codex".into(),
            model,
            provider_name,
            occurred_at,
            session_id,
            request_id,
            input_tokens,
            output_tokens,
            cache_read_input_tokens,
            cache_creation_input_tokens,
            reasoning_tokens,
            usage_details: None,
        })
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    fn test_pricing() -> Arc<PricingService> {
        Arc::new(PricingService::new())
    }

    
    
    
    
    
    #[test]
    fn parses_synthetic_codex_fixture() {
        let tmp = std::env::temp_dir().join(format!(
            "keypilot-codex-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let jsonl_path = tmp.join("synth.jsonl");
        std::fs::write(
            &jsonl_path,
            concat!(
                r#"{"timestamp":1718000000,"model":"gpt-4o","usage":{"prompt_tokens":1000,"completion_tokens":500,"reasoning_tokens":200},"session_id":"s1","request_id":"r1"}"#,
                "\n",
                r#"{"timestamp":1718000001,"model":"gpt-4o"}"#,
                "\n",
                r#"{not even json"#,
                "\n",
                r#"{"model":"gpt-4o","usage":{"prompt_tokens":1,"completion_tokens":1}}"#,
            ),
        )
        .unwrap();

        let parser = CodexParser::with_path(tmp.clone(), test_pricing());
        assert!(parser.is_available());

        let outcome = parser.parse().unwrap();
        let s = outcome.stats;

        assert_eq!(s.files_scanned, 1, "1 jsonl file scanned");
        assert_eq!(s.lines_matched, 1, "exactly 1 line produced a record");
        assert!(
            s.lines_parse_errored >= 1,
            "malformed + missing-timestamp lines counted as errored"
        );
        assert_eq!(outcome.records.len(), 1);

        let r = &outcome.records[0];
        assert_eq!(r.agent_type, "codex");
        assert_eq!(r.model, "gpt-4o");
        assert_eq!(r.input_tokens, 1000);
        assert_eq!(r.output_tokens, 500);
        assert_eq!(r.reasoning_tokens, 200);
        assert_eq!(r.cache_read_input_tokens, 0);
        assert_eq!(r.cache_creation_input_tokens, 0);
        assert_eq!(
            r.occurred_at, 1_718_000_000_000_i64,
            "seconds epoch x 1000 -> millis"
        );
        assert_eq!(r.session_id.as_deref(), Some("s1"));
        assert_eq!(r.request_id.as_deref(), Some("r1"));

        
        let expected = test_pricing().lookup_provider_by_model("gpt-4o");
        assert_eq!(Some(r.provider_name.clone()), expected);
        assert_eq!(r.provider_name, "OpenAI");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn parse_returns_empty_when_path_missing() {
        let tmp = std::env::temp_dir().join(format!(
            "keypilot-codex-noexist-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let parser = CodexParser::with_path(tmp, test_pricing());
        assert!(!parser.is_available());

        let outcome = parser.parse().unwrap();
        assert!(outcome.records.is_empty());
        assert_eq!(outcome.stats.files_scanned, 0);
    }

    #[test]
    fn parses_codex_timestamp_seconds_to_millis() {
        let tmp = std::env::temp_dir().join(format!(
            "keypilot-codex-ts-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let jsonl_path = tmp.join("ts.jsonl");
        std::fs::write(
            &jsonl_path,
            r#"{"timestamp":1700000000,"model":"gpt-4o","usage":{"prompt_tokens":1}}"#,
        )
        .unwrap();

        let parser = CodexParser::with_path(tmp.clone(), test_pricing());
        let outcome = parser.parse().unwrap();

        assert_eq!(outcome.records.len(), 1);
        assert_eq!(
            outcome.records[0].occurred_at, 1_700_000_000_000_i64,
            "seconds epoch multiplied by 1000 -> millis"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn skips_lines_without_usage() {
        let tmp = std::env::temp_dir().join(format!(
            "keypilot-codex-nousage-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let jsonl_path = tmp.join("nousage.jsonl");
        std::fs::write(
            &jsonl_path,
            concat!(
                r#"{"timestamp":1700000000,"model":"gpt-4o","usage":{"prompt_tokens":1,"completion_tokens":1}}"#,
                "\n",
                r#"{"timestamp":1700000000,"model":"gpt-4o"}"#,
            ),
        )
        .unwrap();

        let parser = CodexParser::with_path(tmp.clone(), test_pricing());
        let outcome = parser.parse().unwrap();

        assert_eq!(
            outcome.records.len(),
            1,
            "only the line with usage is recorded"
        );
        assert_eq!(
            outcome.stats.lines_parse_errored, 0,
            "missing usage is a skip, not an error"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
