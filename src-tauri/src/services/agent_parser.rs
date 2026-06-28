//! Agent parser abstraction — one parser per agent source.
//!
//! Adding a new agent = (1) implement `AgentParser` for it, (2) add ONE line
//! to `default_parsers()`.  Frontend, heatmap, and display components do NOT
//! change — they consume the canonical `UsageRecordInput` shape regardless of
//! source.

use std::path::PathBuf;

use crate::error::AppError;

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

    /// Read source data and return canonical `UsageRecordInput` rows.
    /// The caller will feed each row through `TokenUsageService::record_usage`.
    fn parse(&self) -> Result<Vec<crate::types::UsageRecordInput>, AppError>;
}

/// Factory — one parser instance per supported agent.
pub fn default_parsers() -> Vec<Box<dyn AgentParser>> {
    vec![
        Box::new(crate::services::agent_parser_opencode::OpencodeParser::new()),
        Box::new(crate::services::agent_parser_claude_code::ClaudeCodeParser::new()),
    ]
}