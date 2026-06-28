//! Parser for Claude Code's `~/.claude/projects/**/*.jsonl` files.
//!
//! Parses each `.jsonl` file into `UsageRecordInput` rows.  The caller
//! (`auto_import`) feeds them through `TokenUsageService::record_usage`
//! so FNV-1a dedup applies automatically.

use std::path::PathBuf;

use crate::error::AppError;
use crate::services::agent_parser::AgentParser;
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

    /// Recursively glob `~/.claude/projects/**/*.jsonl`, parse every file,
    /// and return all `UsageRecordInput` rows.  The caller deduplicates via
    /// `record_usage` FNV-1a ID.
    fn parse(&self) -> Result<Vec<UsageRecordInput>, AppError> {
        if !self.is_available() {
            return Ok(vec![]);
        }
        let mut all = Vec::new();
        walk_jsonl_dir(&self.path, &mut all);
        Ok(all)
    }
}

impl Default for ClaudeCodeParser {
    fn default() -> Self {
        Self::new()
    }
}

// ---------- JSONL parsing helpers ----------

#[derive(Debug, serde::Deserialize)]
struct ClaudeUsage {
    #[serde(default)]
    input_tokens: Option<i64>,
    #[serde(default)]
    output_tokens: Option<i64>,
    #[serde(default)]
    cache_creation_input_tokens: Option<i64>,
    #[serde(default)]
    cache_read_input_tokens: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
struct ClaudeRow {
    #[serde(default)]
    agent: Option<String>,
    #[serde(default)]
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

#[derive(Debug, serde::Deserialize)]
struct CodexRow {
    #[serde(default)]
    agent: Option<String>,
    #[serde(default)]
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
    usage: Option<serde_json::Value>,
}

fn walk_jsonl_dir(dir: &std::path::Path, out: &mut Vec<UsageRecordInput>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk_jsonl_dir(&p, out);
        } else if p.extension().map(|e| e == "jsonl").unwrap_or(false) {
            let content = match std::fs::read_to_string(&p) {
                Ok(c) => c,
                Err(_) => continue,
            };
            parse_jsonl_file(&content, out);
        }
    }
}

fn parse_jsonl_file(content: &str, out: &mut Vec<UsageRecordInput>) {
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Try Claude shape first
        if let Ok(row) = serde_json::from_str::<ClaudeRow>(line) {
            if let Some(usage) = row.usage {
                let agent = match &row.agent {
                    Some(a) => a.clone(),
                    None => continue,
                };
                let model = match &row.model {
                    Some(m) => m.clone(),
                    None => continue,
                };
                let ts = match row.timestamp {
                    Some(t) => t,
                    None => continue,
                };
                let input = usage.input_tokens.unwrap_or(0);
                let output = usage.output_tokens.unwrap_or(0);
                out.push(UsageRecordInput {
                    agent_type: agent,
                    model,
                    provider_name: row.provider.clone().unwrap_or_else(|| "unknown".into()),
                    occurred_at: ts,
                    session_id: row.session_id.clone(),
                    request_id: row.request_id.clone(),
                    input_tokens: input,
                    output_tokens: output,
                    cache_read_input_tokens: usage.cache_read_input_tokens.unwrap_or(0),
                    cache_creation_input_tokens: usage.cache_creation_input_tokens.unwrap_or(0),
                    reasoning_tokens: 0,
                    usage_details: Some(line.to_string()),
                });
                continue;
            }
        }
        // Try Codex shape
        if let Ok(row) = serde_json::from_str::<CodexRow>(line) {
            if let Some(usage) = row.usage {
                let agent = match &row.agent {
                    Some(a) => a.clone(),
                    None => continue,
                };
                let model = match &row.model {
                    Some(m) => m.clone(),
                    None => continue,
                };
                let ts = match row.timestamp {
                    Some(t) => t,
                    None => continue,
                };
                // Codex uses prompt_tokens / completion_tokens
                let input = usage.get("prompt_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                let output = usage.get("completion_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                out.push(UsageRecordInput {
                    agent_type: agent,
                    model,
                    provider_name: row.provider.clone().unwrap_or_else(|| "unknown".into()),
                    occurred_at: ts,
                    session_id: row.session_id.clone(),
                    request_id: row.request_id.clone(),
                    input_tokens: input,
                    output_tokens: output,
                    cache_read_input_tokens: 0,
                    cache_creation_input_tokens: 0,
                    reasoning_tokens: 0,
                    usage_details: Some(line.to_string()),
                });
            }
        }
    }
}