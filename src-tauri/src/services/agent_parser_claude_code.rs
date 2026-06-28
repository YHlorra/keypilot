//! Parser for Claude Code's `~/.claude/projects/**/*.jsonl` files.
//!
//! Parses each `.jsonl` file into `UsageRecordInput` rows.  The caller
//! (`auto_import`) feeds them through `TokenUsageService::record_usage`
//! so FNV-1a dedup applies automatically.

use std::path::PathBuf;

use serde_json::Value;

use crate::error::AppError;
use crate::services::agent_parser::{AgentParser, ParseOutcome, ParseStats};
use crate::types::UsageRecordInput;

pub struct ClaudeCodeParser {
    path: PathBuf,
}

impl ClaudeCodeParser {
    pub fn new() -> Self {
        Self {
            path: dirs_next(),
        }
    }

    #[cfg(test)]
    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }
}

fn dirs_next() -> PathBuf {
    std::env::var("USERPROFILE")
        .map(|p| PathBuf::from(p).join(".claude").join("projects"))
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".claude")
                .join("projects")
        })
}

impl Default for ClaudeCodeParser {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentParser for ClaudeCodeParser {
    fn agent_type(&self) -> &'static str {
        "claude-code"
    }

    fn display_name(&self) -> &'static str {
        "Claude Code"
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
        walk_jsonl_dir(&self.path, &mut records, &mut stats);
        Ok(ParseOutcome { records, stats })
    }
}

// ---------- JSONL parsing helpers ----------

fn walk_jsonl_dir(
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
            walk_jsonl_dir(&p, out, stats);
        } else if p.extension().map(|e| e == "jsonl").unwrap_or(false) {
            let content = match std::fs::read_to_string(&p) {
                Ok(c) => c,
                Err(e) => {
                    stats.record_error(&p.to_string_lossy(), 0, &format!("read: {e}"));
                    continue;
                }
            };
            parse_jsonl_file(&p, &content, out, stats);
        }
    }
}

fn parse_jsonl_file(
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

        // Branch on outer type — only "assistant" carries usage.
        match v.get("type").and_then(|t| t.as_str()) {
            Some("assistant") => {
                match parse_assistant(&v, stats, &file_name, line_no) {
                    Some(rec) => {
                        stats.lines_matched += 1;
                        out.push(rec);
                    }
                    None => {} // error already recorded inside parse_assistant
                }
            }
            _ => {}
        }
    }
}

fn parse_assistant(
    v: &Value,
    stats: &mut ParseStats,
    file: &str,
    line_no: u32,
) -> Option<UsageRecordInput> {
    let message = match v.get("message") {
        Some(m) => m,
        None => {
            stats.record_error(file, line_no, "missing message");
            return None;
        }
    };

    let model = match message.get("model").and_then(|m| m.as_str()) {
        Some(m) => m.to_string(),
        None => {
            stats.record_error(file, line_no, "missing message.model");
            return None;
        }
    };

    let usage = match message.get("usage") {
        Some(u) => u,
        None => {
            stats.record_error(file, line_no, "missing message.usage");
            return None;
        }
    };

    let ts_str = match v.get("timestamp").and_then(|t| t.as_str()) {
        Some(t) => t,
        None => {
            stats.record_error(file, line_no, "missing top-level timestamp");
            return None;
        }
    };

    let occurred_at = match chrono::DateTime::parse_from_rfc3339(ts_str) {
        Ok(dt) => dt.timestamp_millis(),
        Err(e) => {
            stats.record_error(file, line_no, &format!("timestamp: {e}"));
            return None;
        }
    };

    let input_tokens = usage.get("input_tokens").and_then(|x| x.as_i64()).unwrap_or(0);
    let output_tokens = usage.get("output_tokens").and_then(|x| x.as_i64()).unwrap_or(0);
    let cache_read = usage
        .get("cache_read_input_tokens")
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    let cache_creation = usage
        .get("cache_creation_input_tokens")
        .and_then(|x| x.as_i64())
        .unwrap_or(0);

    let provider_name = derive_provider(&model);

    let session_id = v.get("sessionId").and_then(|x| x.as_str()).map(String::from);
    let request_id = v.get("uuid").and_then(|x| x.as_str()).map(String::from);

    Some(UsageRecordInput {
        agent_type: "claude-code".into(),
        model,
        provider_name,
        occurred_at,
        session_id,
        request_id,
        input_tokens,
        output_tokens,
        cache_read_input_tokens: cache_read,
        cache_creation_input_tokens: cache_creation,
        reasoning_tokens: 0,
        usage_details: None,
    })
}

/// Heuristic provider name from model identifier.  Claude Code's `message.model`
/// does not carry a top-level `provider` field — derive from prefix.
/// Falls back to "unknown" when the model name doesn't match any known pattern
/// (will price as $0 until PricingService grows an entry).
fn derive_provider(model: &str) -> String {
    if model.starts_with("claude-") {
        return "anthropic".into();
    }
    if model.starts_with("gpt-")
        || model.starts_with("o1-")
        || model.starts_with("o3-")
        || model.starts_with("o4-")
    {
        return "openai".into();
    }
    if model.starts_with("MiniMax-") {
        return "minimax-cn-coding-plan".into();
    }
    if let Some((prefix, _)) = model.split_once('/') {
        return prefix.to_string();
    }
    "unknown".into()
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic fixture — covers one valid `assistant` line, structural
    /// lines that should be ignored, and a malformed line that should
    /// increment `lines_parse_errored` (NOT silently dropped).
    const SAMPLE: &str = r#"
{"type":"worktree-state","worktreeSession":{"repo":"synth"},"sessionId":"sess-1"}
{"type":"user","message":{"role":"user","content":"hello"},"sessionId":"sess-1","timestamp":"2026-06-24T02:58:48.331Z"}
{"type":"assistant","message":{"id":"m1","type":"message","role":"assistant","model":"claude-test-model","usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":10,"cache_read_input_tokens":5}},"type":"assistant","uuid":"u1","timestamp":"2026-06-24T02:58:48.331Z","sessionId":"sess-1"}
{not even json
"#;

    #[test]
    fn parses_synthetic_fixture() {
        let tmp = std::env::temp_dir().join(format!("keypilot-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let jsonl_path = tmp.join("synth.jsonl");
        std::fs::write(&jsonl_path, SAMPLE.trim_start()).unwrap();

        let parser = ClaudeCodeParser::with_path(tmp.clone());
        assert!(parser.is_available());

        let outcome = parser.parse().unwrap();
        let s = outcome.stats;

        assert_eq!(s.files_scanned, 1, "1 jsonl file scanned");
        assert_eq!(s.lines_matched, 1, "exactly 1 assistant line produced a record");
        assert_eq!(s.lines_parse_errored, 1, "malformed line counted (not silently dropped)");
        assert_eq!(outcome.records.len(), 1);

        let r = &outcome.records[0];
        assert_eq!(r.agent_type, "claude-code");
        assert_eq!(r.model, "claude-test-model");
        assert_eq!(r.input_tokens, 100);
        assert_eq!(r.output_tokens, 50);
        assert_eq!(r.cache_read_input_tokens, 5);
        assert_eq!(r.cache_creation_input_tokens, 10);
        assert_eq!(r.session_id.as_deref(), Some("sess-1"));
        assert_eq!(r.request_id.as_deref(), Some("u1"));
        // ponytail: just verify timestamp was parsed (>2020 epoch ms), don't pin exact value
        assert!(r.occurred_at > 1_577_836_800_000, "ISO timestamp parsed to epoch ms");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn parse_returns_empty_when_path_missing() {
        let tmp = std::env::temp_dir().join(format!("keypilot-noexist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let parser = ClaudeCodeParser::with_path(tmp);
        assert!(!parser.is_available());

        let outcome = parser.parse().unwrap();
        assert!(outcome.records.is_empty());
        assert_eq!(outcome.stats.files_scanned, 0);
    }
}
