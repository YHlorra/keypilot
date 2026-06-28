//! Parser for opencode's `opencode.db` session table (SQLite, READ ONLY).
//!
//! Calls `parse_opencode_db_records()` — the same pure row-parsing function
//! used by `TokenUsageService::import_opencode_db`, so there is zero duplicate
//! SQL/logic between the two call sites.

use std::path::PathBuf;

use crate::error::AppError;
use crate::services::agent_parser::AgentParser;
use crate::services::token_usage::parse_opencode_db_records;
use crate::types::UsageRecordInput;

/// Path to opencode.db on Windows.
fn default_db_path() -> PathBuf {
    std::env::var("LOCALAPPDATA")
        .map(|p| PathBuf::from(p).join("opencode").join("opencode.db"))
        .unwrap_or_else(|_| {
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("opencode")
                .join("opencode.db")
        })
}

pub struct OpencodeParser {
    path: PathBuf,
}

impl OpencodeParser {
    pub fn new() -> Self {
        Self {
            path: default_db_path(),
        }
    }
}

impl AgentParser for OpencodeParser {
    fn agent_type(&self) -> &'static str {
        "opencode"
    }

    fn display_name(&self) -> &'static str {
        "opencode"
    }

    fn default_path(&self) -> PathBuf {
        self.path.clone()
    }

    fn is_available(&self) -> bool {
        self.path.exists()
    }

    /// Returns session rows as `UsageRecordInput` — caller (`auto_import`)
    /// feeds them through `record_usage` so FNV-1a dedup applies.
    fn parse(&self) -> Result<Vec<UsageRecordInput>, AppError> {
        if !self.is_available() {
            return Ok(vec![]);
        }
        parse_opencode_db_records(&self.path)
    }
}

impl Default for OpencodeParser {
    fn default() -> Self {
        Self::new()
    }
}