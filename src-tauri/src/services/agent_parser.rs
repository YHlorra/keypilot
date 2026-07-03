






use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::services::pricing::PricingService;
use crate::types::UsageRecordInput;




#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParseStats {
    pub files_scanned: u32,
    pub lines_scanned: u32,
    pub lines_matched: u32,
    pub lines_parse_errored: u32,
    
    
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



#[derive(Debug, Clone)]
pub struct ParseOutcome {
    pub records: Vec<UsageRecordInput>,
    pub stats: ParseStats,
}





pub trait AgentParser: Send + Sync {
    
    fn agent_type(&self) -> &'static str;

    
    fn display_name(&self) -> &'static str;

    
    
    fn default_path(&self) -> PathBuf;

    
    
    fn is_available(&self) -> bool;

    
    
    fn parse(&self) -> Result<ParseOutcome, AppError>;
}







pub fn default_parsers(pricing: Arc<PricingService>) -> Vec<Box<dyn AgentParser>> {
    vec![
        Box::new(crate::services::agent_parser_opencode::OpencodeParser::new()),
        Box::new(crate::services::agent_parser_claude_code::ClaudeCodeParser::new(pricing.clone())),
        Box::new(crate::services::agent_parser_codex::CodexParser::new(pricing)),
    ]
}
