//! Agent parser abstraction — one parser per agent source.
//!
//! Adding a new agent = (1) implement `AgentParser` for it, (2) add ONE line
//! to `default_parsers()`.  Frontend, heatmap, and display components do NOT
//! change — they consume the canonical `UsageRecordInput` shape regardless of
//! source.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::types::UsageRecordInput;

/// Scanner-level observability counters.  Surfaced via `last_auto_import`
/// meta JSON so the user can see WHY an import imported 0 rows
/// (instead of getting a silent `{imported:0, errors:0}` shrug).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParseStats {
    pub files_scanned: u32,
    pub lines_scanned: u32,
    pub lines_matched: u32,
    pub lines_parse_errored: u32,
    /// First 3 error messages, format `"{file}:{line_no}: {reason}"`.
    /// Bounded so a 385-file scan cannot produce a megabyte JSON blob.
    pub sample_errors: Vec<String>,
}

impl ParseStats {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn record_error(&mut self, file: &str, line_no: u32, reason: &str) {
        self.lines_parse_errored += 1;
        if self.sample_errors.len() < 3 {
            self.sample_errors.push(format!("{file}:{line_no}: {reason}"));
        }
    }
}

/// Combined output of one `parse()` call: the records to insert PLUS the
/// scan counters that explain how those records were found (or not).
#[derive(Debug, Clone)]
pub struct ParseOutcome {
    pub records: Vec<UsageRecordInput>,
    pub stats: ParseStats,
}

/// One parser per agent source (opencode.db, claude-code jsonl, codex, etc.).
/// Each parser knows how to read its own data format and emit canonical
/// `UsageRecordInput` rows which are fed through `TokenUsageService::record_usage`,
/// so existing dedup + daily rollup logic applies automatically.
pub trait AgentParser: Send + Sync {
    /// Stable identifier stored in `token_usage_records.agent_type`.
    fn agent_type(&self) -> &'static str;

    /// Human-readable name for Settings UI and logs.
    fn display_name(&self) -> &'static str;

    /// Default path where this agent stores its data on disk (Windows).
    /// Used both for `is_available()` checks and for Settings display.
    fn default_path(&self) -> PathBuf;

    /// True when `default_path()` exists and looks like a real agent data store.
    /// Cheap check only — do NOT open or parse here.
    fn is_available(&self) -> bool;

    /// Read source data and return canonical `UsageRecordInput` rows + scan
    /// stats.  The caller feeds each row through `TokenUsageService::record_usage`.
    fn parse(&self) -> Result<ParseOutcome, AppError>;
}

/// Factory — one parser instance per supported agent.
pub fn default_parsers() -> Vec<Box<dyn AgentParser>> {
    vec![
        Box::new(crate::services::agent_parser_opencode::OpencodeParser::new()),
        Box::new(crate::services::agent_parser_claude_code::ClaudeCodeParser::new()),
    ]
}
