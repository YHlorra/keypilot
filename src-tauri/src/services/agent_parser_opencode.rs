





use std::path::PathBuf;

use crate::error::AppError;
use crate::services::agent_parser::{AgentParser, ParseOutcome, ParseStats};
use crate::services::token_usage::parse_opencode_db_records;











pub(crate) fn candidate_paths() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(p) = std::env::var("LOCALAPPDATA") {
        dirs.push(PathBuf::from(p).join("opencode"));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/opencode"));
    }
    let mut out: Vec<PathBuf> = dirs.iter().flat_map(|d| filter_db_files(d)).collect();
    out.sort();
    out
}










fn filter_db_files(base_dir: &std::path::Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(base_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |n| n.starts_with("opencode") && n.ends_with(".db"))
        })
        .collect()
}

pub struct OpencodeParser {
    candidates: Vec<PathBuf>,
}

impl OpencodeParser {
    pub fn new() -> Self {
        Self {
            candidates: candidate_paths(),
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
        self.candidates
            .iter()
            .find(|p| p.exists())
            .cloned()
            .or_else(|| self.candidates.first().cloned())
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_default()
                    .join(".local/share/opencode/opencode.db")
            })
    }

    
    fn is_available(&self) -> bool {
        self.candidates.iter().any(|p| p.exists())
    }

    
    
    
    
    
    
    fn parse(&self) -> Result<ParseOutcome, AppError> {
        if !self.is_available() {
            return Ok(ParseOutcome { records: vec![], stats: ParseStats::empty() });
        }
        let mut records = Vec::new();
        let mut stats = ParseStats::empty();
        for path in &self.candidates {
            if !path.exists() {
                continue;
            }
            stats.files_scanned += 1;
            match parse_opencode_db_records(path) {
                Ok(rows) => {
                    stats.lines_scanned += rows.len() as u32;
                    stats.lines_matched += rows.len() as u32;
                    records.extend(rows);
                }
                Err(e) => {
                    stats.record_error(&path.to_string_lossy(), 0, &e.to_string());
                }
            }
        }
        Ok(ParseOutcome { records, stats })
    }
}

impl Default for OpencodeParser {
    fn default() -> Self {
        Self::new()
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    
    
    fn isolated_tempdir(tag: &str) -> std::path::PathBuf {
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("kp-opencode-{tag}-{pid}-{nanos}"))
    }

    #[test]
    fn filter_db_files_matches_opencode_db() {
        let dir = isolated_tempdir("find");
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("opencode.db");
        std::fs::write(&db, b"").unwrap();

        let found = filter_db_files(&dir);
        assert!(found.contains(&db), "opencode.db must match, got {found:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn filter_db_files_matches_channel_db() {
        let dir = isolated_tempdir("chan");
        std::fs::create_dir_all(&dir).unwrap();
        let chan = dir.join("opencode-beta.db");
        std::fs::write(&chan, b"").unwrap();

        let found = filter_db_files(&dir);
        assert!(found.contains(&chan), "opencode-beta.db must match, got {found:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    
    
    
    
    #[test]
    fn filter_db_files_excludes_wal_and_shm() {
        let dir = isolated_tempdir("wal");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("opencode.db-wal"), b"").unwrap();
        std::fs::write(dir.join("opencode.db-shm"), b"").unwrap();

        let found = filter_db_files(&dir);
        assert!(found.is_empty(), "side-files must not match, got {found:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    
    
    #[test]
    fn filter_db_files_ignores_non_opencode_dbs() {
        let dir = isolated_tempdir("other");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("app.db"), b"").unwrap();
        std::fs::write(dir.join("unrelated.sqlite"), b"").unwrap();

        let found = filter_db_files(&dir);
        assert!(found.is_empty(), "unrelated dbs must not match, got {found:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    
    #[test]
    fn filter_db_files_returns_empty_on_missing_dir() {
        let dir = isolated_tempdir("missing");
        
        let found = filter_db_files(&dir);
        assert!(found.is_empty());
    }
}