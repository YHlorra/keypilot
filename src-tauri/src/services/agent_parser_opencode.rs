//! Parser for opencode's `opencode.db` session table (SQLite, READ ONLY).
//!
//! Calls `parse_opencode_db_records()` — the same pure row-parsing function
//! used by `TokenUsageService::import_opencode_db`, so there is zero duplicate
//! SQL/logic between the two call sites.

use std::path::PathBuf;

use crate::error::AppError;
use crate::services::agent_parser::{AgentParser, ParseOutcome, ParseStats};
use crate::services::token_usage::parse_opencode_db_records;

/// Discover opencode Go databases across platform conventions.
///
/// Aligns with token-monitor `discoverDbPaths` (Javis603/token-monitor):
///   1. `%LOCALAPPDATA%\opencode\opencode*.db` (Windows local convention)
///   2. `~/.local/share/opencode/opencode*.db` (XDG convention, also used by
///      opencode Go CLI v1.17+ on Windows via HOME)
/// Glob accepts `opencode.db` and `opencode-<channel>.db`
/// (channel = `[A-Za-z0-9._-]+`); WAL/SHM side-files (e.g. `opencode.db-wal`)
/// are rejected by `ends_with(".db")`. Returns paths in sorted order; may
/// be empty.
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

/// Filter `*.db` files in a single base directory that match the opencode
/// naming convention. Pulled out so tests can pass a controlled tempdir
/// without wrestling with `dirs::home_dir()` (which on Windows resolves via
/// `SHGetKnownFolderPath` and ignores `HOME` env overrides).
///
/// Naming: `starts_with("opencode") && ends_with(".db")` matches:
///   - `opencode.db`                          ✓
///   - `opencode-<channel>.db`                ✓ (channel = [A-Za-z0-9._-]+)
///   - `opencode.db-wal` / `opencode.db-shm`  ✗ (no .db suffix)
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

    /// UI display — first existing candidate, falling back to the XDG primary path.
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

    /// True if any candidate exists.
    fn is_available(&self) -> bool {
        self.candidates.iter().any(|p| p.exists())
    }

    /// Returns session rows as `UsageRecordInput` — caller (`auto_import`)
    /// feeds them through `record_usage` so FNV-1a dedup applies.
    /// Iterates every candidate; partial parse failures (schema drift on a
    /// single channel) do not abort the batch — they land in
    /// `stats.sample_errors` so the UI can surface WHY a channel contributed
    /// 0 rows instead of silently dropping it.
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

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: per-process isolated tempdir to keep parallel tests from
    /// stepping on each other.
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

    /// Reject WAL/SHM side-files: `opencode.db-wal` and `opencode.db-shm`
    /// must NOT appear in candidate paths even when the parent dir is full
    /// of them.  These are SQLite Write-Ahead Log + shared-memory side-files
    /// and are not standalone databases.
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

    /// Filter ignores non-opencode databases (e.g. a sibling app storing
    /// `app.db` in the same directory).
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

    /// Missing dir returns empty (read_dir fails silently).
    #[test]
    fn filter_db_files_returns_empty_on_missing_dir() {
        let dir = isolated_tempdir("missing");
        // Intentionally do NOT create the dir.
        let found = filter_db_files(&dir);
        assert!(found.is_empty());
    }
}