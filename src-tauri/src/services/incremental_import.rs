

















use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use notify::{RecursiveMode};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, DebouncedEvent, Debouncer, RecommendedCache};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::database::{AgentFileCursor, Database};
use crate::error::AppError;
use crate::services::agent_parser::{default_parsers, AgentParser, ParseOutcome};
use crate::services::token_usage::{deterministic_id, TokenUsageService};
use crate::types::UsageRecordInput;



pub const TOKEN_USAGE_TICK_EVENT: &str = "token_usage_tick";




const DEBOUNCE_WINDOW: Duration = Duration::from_millis(300);




const FALLBACK_POLL_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize)]
pub struct TokenUsageTickPayload {
    pub agent_type: String,
    
    pub imported: u32,
    
    pub skipped: u32,
    
    pub latest_at: Option<i64>,
    
    
    pub total_today_tokens: i64,
    
    pub total_today_cost_usd: f64,
}



pub struct IncrementalImporter {
    
    _debouncer: Option<Debouncer<notify::RecommendedWatcher, RecommendedCache>>,
    _shutdown_tx: Option<mpsc::Sender<()>>,
    _watcher_thread: Option<thread::JoinHandle<()>>,
}

impl IncrementalImporter {
    
    
    
    
    
    
    
    pub fn start(
        app: AppHandle,
        db: Arc<std::sync::Mutex<Database>>,
        pricing: Arc<crate::services::pricing::PricingService>,
        parsers: Vec<Box<dyn AgentParser>>,
    ) -> Self {
        
        
        
        
        let parsers = Arc::new(parsers);
        let parsers_for_initial = Arc::clone(&parsers);
        let parsers_for_watcher = Arc::clone(&parsers);
        let parsers_for_fallback = Arc::clone(&parsers);

        
        
        
        let svc = TokenUsageService::new(db.clone(), pricing.clone());
        let app_for_initial = app.clone();
        let db_for_initial = db.clone();
        let svc_for_initial = svc.clone();
        thread::spawn(move || {
            run_initial_catchup(&db_for_initial, &svc_for_initial, &app_for_initial, &parsers_for_initial);
        });

        
        let (debouncer, watch_paths) = build_watcher(
            app.clone(),
            db.clone(),
            svc.clone(),
            parsers_for_watcher,
        );

        
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
        let watcher_thread = thread::spawn(move || {
            fallback_poll_loop(app, db, svc, watch_paths, parsers_for_fallback, shutdown_rx);
        });

        Self {
            _debouncer: Some(debouncer),
            _shutdown_tx: Some(shutdown_tx),
            _watcher_thread: Some(watcher_thread),
        }
    }

}



fn run_initial_catchup(
    db: &Arc<std::sync::Mutex<Database>>,
    svc: &TokenUsageService,
    app: &AppHandle,
    parsers: &[Box<dyn AgentParser>],
) {
    let cursors = match db.lock() {
        Ok(d) => match d.list_all_cursors() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[incremental_import] catchup: list_all_cursors failed: {e}");
                return;
            }
        },
        Err(e) => {
            eprintln!("[incremental_import] catchup: db lock failed: {e}");
            return;
        }
    };

    for cursor in &cursors {
        if let Err(e) = process_one_file(&cursor.file_path, &cursor.agent_type, db, svc, parsers, app) {
            eprintln!(
                "[incremental_import] catchup failed for {}: {e}",
                cursor.file_path
            );
        }
    }
}



fn build_watcher(
    app: AppHandle,
    db: Arc<std::sync::Mutex<Database>>,
    svc: TokenUsageService,
    parsers: Arc<Vec<Box<dyn AgentParser>>>,
) -> (Debouncer<notify::RecommendedWatcher, RecommendedCache>, Vec<PathBuf>) {
    let watch_paths = collect_agent_dirs(&parsers);

    let mut debouncer = match new_debouncer(
        DEBOUNCE_WINDOW,
        None,
        move |result: DebounceEventResult| match result {
            Ok(events) => handle_debounced_events(events, &app, &db, &svc, &parsers),
            Err(errors) => {
                for e in errors {
                    eprintln!("[incremental_import] notify error: {e}");
                }
            }
        },
    ) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[incremental_import] failed to create debouncer: {e}");
            
            
            return (
                new_debouncer(DEBOUNCE_WINDOW, None, |_: DebounceEventResult| {})
                    .expect("dummy debouncer"),
                watch_paths,
            );
        }
    };

    for path in &watch_paths {
        if path.is_dir() {
            if let Err(e) = debouncer.watch(path, RecursiveMode::Recursive) {
                eprintln!(
                    "[incremental_import] failed to watch {}: {e}",
                    path.display()
                );
            }
        }
    }

    (debouncer, watch_paths)
}

fn collect_agent_dirs(parsers: &[Box<dyn AgentParser>]) -> Vec<PathBuf> {
    parsers
        .iter()
        .map(|p| p.default_path())
        .filter(|p| p.is_dir())
        .collect()
}





fn handle_debounced_events(
    events: Vec<DebouncedEvent>,
    app: &AppHandle,
    db: &Arc<std::sync::Mutex<Database>>,
    svc: &TokenUsageService,
    parsers: &[Box<dyn AgentParser>],
) {
    
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for event in events {
        for path in &event.paths {
            if !is_jsonl(path) {
                continue;
            }
            let canonical = match path.canonicalize() {
                Ok(p) => p.to_string_lossy().to_string(),
                Err(_) => path.to_string_lossy().to_string(),
            };
            
            
            
            
            let agent_type = infer_agent_type(path, parsers);
            seen.insert((agent_type, canonical));
        }
    }
    for (agent_type, path) in seen {
        if let Err(e) = process_path(&path, &agent_type, db, svc, parsers, app) {
            eprintln!("[incremental_import] process {path} failed: {e}");
        }
    }
}

fn is_jsonl(p: &Path) -> bool {
    p.extension().map(|e| e == "jsonl").unwrap_or(false)
}

fn infer_agent_type(path: &Path, parsers: &[Box<dyn AgentParser>]) -> String {
    let path_str = path.to_string_lossy().to_lowercase();
    for p in parsers {
        let default = p.default_path().to_string_lossy().to_lowercase();
        if path_str.starts_with(&default) {
            return p.agent_type().to_string();
        }
    }
    
    
    
    parsers
        .first()
        .map(|p| p.agent_type().to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn process_path(
    path_str: &str,
    agent_type: &str,
    db: &Arc<std::sync::Mutex<Database>>,
    svc: &TokenUsageService,
    _parsers: &[Box<dyn AgentParser>],
    app: &AppHandle,
) -> Result<(), AppError> {
    let db_guard = db.lock().map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("db lock: {e}"),
        ))
    })?;
    let cursor = db_guard
        .get_cursor(agent_type, path_str)?
        .unwrap_or(AgentFileCursor {
            agent_type: agent_type.to_string(),
            file_path: path_str.to_string(),
            byte_offset: 0,
            file_size: 0,
            last_scan_at: 0,
            last_event_at: None,
        });
    drop(db_guard);

    process_one_file(path_str, agent_type, db, svc, _parsers, app)?;
    let _ = cursor; 
    Ok(())
}






#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessOutcome {
    pub imported: u32,
    pub skipped: u32,
    pub latest_at: Option<i64>,
    pub truncated: bool, 
}





fn process_one_file_inner(
    path_str: &str,
    agent_type: &str,
    db: &Arc<std::sync::Mutex<Database>>,
    svc: &TokenUsageService,
    parsers: &[Box<dyn AgentParser>],
) -> Result<ProcessOutcome, AppError> {
    let mut outcome = ProcessOutcome::default();
    let path = Path::new(path_str);

    
    if !path.exists() {
        return Ok(outcome);
    }

    let current_size = match std::fs::metadata(path) {
        Ok(m) => m.len() as i64,
        Err(e) => return Err(AppError::Io(e)),
    };

    
    
    
    let mut cursor = {
        let db_guard = db.lock().map_err(|e| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("db lock: {e}"),
            ))
        })?;
        let c = db_guard
            .get_cursor(agent_type, path_str)?
            .unwrap_or(AgentFileCursor {
                agent_type: agent_type.to_string(),
                file_path: path_str.to_string(),
                byte_offset: 0,
                file_size: 0,
                last_scan_at: 0,
                last_event_at: None,
            });

        
        
        if current_size < c.byte_offset {
            let mut c = c;
            c.byte_offset = 0;
            outcome.truncated = true;
            c
        } else {
            c
        }
    };

    if current_size == cursor.byte_offset {
        
        cursor.last_event_at = Some(crate::timeutil::now_millis());
        cursor.last_scan_at = crate::timeutil::now_millis();
        let db_guard = db.lock().map_err(|e| {
            AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, format!("db lock: {e}")))
        })?;
        db_guard.upsert_cursor(&cursor)?;
        return Ok(outcome);
    }

    
    let use_file = std::fs::File::open(path)?;
    use std::io::{Read, Seek, SeekFrom};
    let mut file = use_file;
    file.seek(SeekFrom::Start(cursor.byte_offset as u64))?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    drop(file);

    
    let parser = match parsers.iter().find(|p| p.agent_type() == agent_type) {
        Some(p) => p,
        None => return Ok(outcome),
    };

    let parsed = parse_new_bytes(parser.as_ref(), &buf)?;

    
    
    for input in parsed.records {
        let id = deterministic_id(&input);
        match svc.record_usage(&id, input) {
            Ok(rec) => {
                outcome.imported += 1;
                outcome.latest_at =
                    Some(outcome.latest_at.map_or(rec.occurred_at, |cur| cur.max(rec.occurred_at)));
            }
            Err(AppError::TokenUsageDuplicate(_)) => {
                outcome.skipped += 1;
            }
            Err(e) => {
                eprintln!("[incremental_import] record_usage failed: {e}");
            }
        }
    }

    
    let now = crate::timeutil::now_millis();
    cursor.byte_offset = current_size;
    cursor.file_size = current_size;
    cursor.last_scan_at = now;
    cursor.last_event_at = Some(now);
    {
        let db_guard = db.lock().map_err(|e| {
            AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, format!("db lock: {e}")))
        })?;
        db_guard.upsert_cursor(&cursor)?;
    }

    Ok(outcome)
}

fn process_one_file(
    path_str: &str,
    agent_type: &str,
    db: &Arc<std::sync::Mutex<Database>>,
    svc: &TokenUsageService,
    parsers: &[Box<dyn AgentParser>],
    app: &AppHandle,
) -> Result<(), AppError> {
    let outcome = process_one_file_inner(path_str, agent_type, db, svc, parsers)?;
    if outcome.imported > 0 || outcome.skipped > 0 {
        emit_tick(
            app,
            agent_type,
            outcome.imported,
            outcome.skipped,
            outcome.latest_at,
        );
    }
    Ok(())
}




fn parse_new_bytes(parser: &dyn AgentParser, new_bytes: &str) -> Result<ParseOutcome, AppError> {
    use crate::services::agent_parser::{ParseStats, ParseOutcome};
    
    
    
    
    if parser.agent_type() == "opencode" {
        return parser.parse();
    }
    
    
    
    
    let mut records = Vec::new();
    let mut stats = ParseStats {
        files_scanned: 1,
        lines_scanned: 0,
        lines_matched: 0,
        lines_parse_errored: 0,
        sample_errors: Vec::new(),
    };

    for (idx, line) in new_bytes.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        stats.lines_scanned += 1;
        match parse_one_line(parser.agent_type(), line) {
            Some(input) => {
                stats.lines_matched += 1;
                let _ = idx; 
                records.push(input);
            }
            None => {
                
                
                let _ = idx;
            }
        }
    }

    Ok(ParseOutcome { records, stats })
}





fn parse_one_line(agent_type: &str, line: &str) -> Option<UsageRecordInput> {
    use serde_json::Value;
    let v: Value = serde_json::from_str(line).ok()?;

    match agent_type {
        "claude-code" => {
            
            if v.get("type").and_then(|t| t.as_str()) != Some("assistant") {
                return None;
            }
            let message = v.get("message")?;
            let model = message.get("model").and_then(|m| m.as_str())?.to_string();
            let usage = message.get("usage")?;
            let ts_str = v.get("timestamp").and_then(|t| t.as_str())?;
            let occurred_at = chrono::DateTime::parse_from_rfc3339(ts_str)
                .ok()?
                .timestamp_millis();
            let input_tokens = usage.get("input_tokens").and_then(|x| x.as_i64()).unwrap_or(0);
            let output_tokens = usage.get("output_tokens").and_then(|x| x.as_i64()).unwrap_or(0);
            let cache_read = usage.get("cache_read_input_tokens").and_then(|x| x.as_i64()).unwrap_or(0);
            let cache_creation = usage.get("cache_creation_input_tokens").and_then(|x| x.as_i64()).unwrap_or(0);
            
            
            
            Some(UsageRecordInput {
                agent_type: "claude-code".into(),
                model,
                provider_name: "unknown".into(),
                occurred_at,
                session_id: v.get("sessionId").and_then(|x| x.as_str()).map(String::from),
                request_id: v.get("uuid").and_then(|x| x.as_str()).map(String::from),
                input_tokens,
                output_tokens,
                cache_read_input_tokens: cache_read,
                cache_creation_input_tokens: cache_creation,
                reasoning_tokens: 0,
                usage_details: None,
            })
        }
        "codex" => {
            
            let usage = v.get("usage")?;
            let timestamp_secs = v.get("timestamp").and_then(|t| t.as_i64())?;
            let model = v.get("model").and_then(|m| m.as_str())?.to_string();
            let input_tokens = usage.get("prompt_tokens").and_then(|x| x.as_i64()).unwrap_or(0);
            let output_tokens = usage.get("completion_tokens").and_then(|x| x.as_i64()).unwrap_or(0);
            let reasoning_tokens = usage.get("reasoning_tokens").and_then(|x| x.as_i64()).unwrap_or(0);
            Some(UsageRecordInput {
                agent_type: "codex".into(),
                model,
                provider_name: "openai".into(),
                occurred_at: timestamp_secs * 1000,
                session_id: v.get("session_id").and_then(|x| x.as_str()).map(String::from),
                request_id: v.get("request_id").and_then(|x| x.as_str()).map(String::from),
                input_tokens,
                output_tokens,
                cache_read_input_tokens: 0,
                cache_creation_input_tokens: 0,
                reasoning_tokens,
                usage_details: None,
            })
        }
        _ => None,
    }
}



fn fallback_poll_loop(
    app: AppHandle,
    db: Arc<std::sync::Mutex<Database>>,
    svc: TokenUsageService,
    _watch_paths: Vec<PathBuf>,
    _parsers: Arc<Vec<Box<dyn AgentParser>>>,
    shutdown_rx: mpsc::Receiver<()>,
) {
    
    
    
    let mut sleep_for = Duration::from_secs(0);
    loop {
        if shutdown_rx.recv_timeout(sleep_for).is_ok() {
            return;
        }
        sleep_for = FALLBACK_POLL_INTERVAL;

        let cursors = match db.lock() {
            Ok(d) => match d.list_all_cursors() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[incremental_import] fallback poll: list_all_cursors: {e}");
                    continue;
                }
            },
            Err(e) => {
                eprintln!("[incremental_import] fallback poll: db lock: {e}");
                continue;
            }
        };

        let parsers = default_parsers(svc.pricing());
        for cursor in &cursors {
            if let Err(e) =
                process_one_file(&cursor.file_path, &cursor.agent_type, &db, &svc, &parsers, &app)
            {
                eprintln!(
                    "[incremental_import] fallback poll {} failed: {e}",
                    cursor.file_path
                );
            }
        }
    }
}



fn emit_tick(
    app: &AppHandle,
    agent_type: &str,
    imported: u32,
    skipped: u32,
    latest_at: Option<i64>,
) {
    let (total_today_tokens, total_today_cost_usd) =
        read_today_totals(app).unwrap_or((0, 0.0));

    let payload = TokenUsageTickPayload {
        agent_type: agent_type.to_string(),
        imported,
        skipped,
        latest_at,
        total_today_tokens,
        total_today_cost_usd,
    };
    if let Err(e) = app.emit(TOKEN_USAGE_TICK_EVENT, payload) {
        eprintln!("[incremental_import] emit failed: {e}");
    }
}




fn read_today_totals(_app: &AppHandle) -> Result<(i64, f64), AppError> {
    
    
    
    
    
    
    
    
    Ok((0, 0.0))
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::services::agent_parser::ParseStats;
    use crate::services::pricing::PricingService;
    use std::sync::{Arc, Mutex};

    #[test]
    fn parse_one_line_claude_code_assistant() {
        let line = r#"{"type":"assistant","message":{"model":"claude-x","usage":{"input_tokens":1,"output_tokens":2,"cache_read_input_tokens":3,"cache_creation_input_tokens":4}},"timestamp":"2026-06-29T00:00:00Z"}"#;
        let rec = parse_one_line("claude-code", line).expect("should parse");
        assert_eq!(rec.agent_type, "claude-code");
        assert_eq!(rec.model, "claude-x");
        assert_eq!(rec.input_tokens, 1);
        assert_eq!(rec.output_tokens, 2);
        assert_eq!(rec.cache_read_input_tokens, 3);
        assert_eq!(rec.cache_creation_input_tokens, 4);
    }

    #[test]
    fn parse_one_line_claude_code_skips_non_assistant() {
        let line = r#"{"type":"user","message":{"role":"user","content":"hi"},"timestamp":"2026-06-29T00:00:00Z"}"#;
        assert!(parse_one_line("claude-code", line).is_none());
    }

    #[test]
    fn parse_one_line_codex_basic() {
        let line = r#"{"timestamp":1718000000,"model":"gpt-4o","usage":{"prompt_tokens":10,"completion_tokens":20,"reasoning_tokens":5},"session_id":"s","request_id":"r"}"#;
        let rec = parse_one_line("codex", line).expect("should parse");
        assert_eq!(rec.agent_type, "codex");
        assert_eq!(rec.model, "gpt-4o");
        assert_eq!(rec.input_tokens, 10);
        assert_eq!(rec.output_tokens, 20);
        assert_eq!(rec.reasoning_tokens, 5);
        assert_eq!(rec.occurred_at, 1_718_000_000_000);
    }

    #[test]
    fn parse_one_line_codex_no_usage_skipped() {
        let line = r#"{"timestamp":1718000000,"model":"gpt-4o"}"#;
        assert!(parse_one_line("codex", line).is_none());
    }

    #[test]
    fn parse_one_line_unknown_agent_returns_none() {
        let line = r#"{"model":"x","usage":{}}"#;
        assert!(parse_one_line("mystery-agent", line).is_none());
    }

    
    
    #[test]
    fn parse_one_line_claude_code_real_smoke_shape() {
        let line = r#"{"type":"assistant","message":{"model":"claude-smoke-test","usage":{"input_tokens":777,"output_tokens":333}},"timestamp":"2026-06-29T17:29:52.385Z","sessionId":"smoke","uuid":"u-smoke"}"#;
        let rec = parse_one_line("claude-code", line).expect("real-shape line should parse");
        assert_eq!(rec.agent_type, "claude-code");
        assert_eq!(rec.model, "claude-smoke-test");
        assert_eq!(rec.input_tokens, 777);
        assert_eq!(rec.output_tokens, 333);
        assert_eq!(rec.session_id.as_deref(), Some("smoke"));
        
        let expected_ms = chrono::DateTime::parse_from_rfc3339("2026-06-29T17:29:52.385Z")
            .unwrap()
            .timestamp_millis();
        assert_eq!(rec.occurred_at, expected_ms);
    }

    #[test]
    fn cursor_roundtrip_via_db() {
        let db = Database::open_in_memory().unwrap();
        db.setup_schema().unwrap();
        let c = AgentFileCursor {
            agent_type: "claude-code".into(),
            file_path: "/tmp/foo.jsonl".into(),
            byte_offset: 12345,
            file_size: 99999,
            last_scan_at: 1718000000000,
            last_event_at: Some(1718000001000),
        };
        db.upsert_cursor(&c).unwrap();
        let got = db.get_cursor("claude-code", "/tmp/foo.jsonl").unwrap().unwrap();
        assert_eq!(got.byte_offset, 12345);
        assert_eq!(got.file_size, 99999);

        let all = db.list_cursors_for_agent("claude-code").unwrap();
        assert_eq!(all.len(), 1);

        db.delete_cursor("claude-code", "/tmp/foo.jsonl").unwrap();
        assert!(db.get_cursor("claude-code", "/tmp/foo.jsonl").unwrap().is_none());
    }

    

    
    fn make_runtime() -> (
        Arc<Mutex<Database>>,
        Arc<TokenUsageService>,
        Vec<Box<dyn AgentParser>>,
    ) {
        let db = Database::open_in_memory().unwrap();
        db.setup_schema().unwrap();
        let db = Arc::new(Mutex::new(db));
        let pricing = Arc::new(PricingService::new());
        let svc = Arc::new(TokenUsageService::new(db.clone(), pricing.clone()));
        let parsers = default_parsers(pricing);
        (db, svc, parsers)
    }

    
    fn write_jsonl(dir: &std::path::Path, name: &str, lines: &[&str]) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, lines.join("\n")).unwrap();
        path
    }

    
    
    #[test]
    fn integration_initial_scan_ingests_all_lines_and_parks_cursor() {
        let tmp = std::env::temp_dir().join(format!("kp-int1-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let lines = [
            r#"{"type":"user","message":{"role":"user","content":"hi"},"timestamp":"2026-06-29T00:00:00Z"}"#,
            r#"{"type":"assistant","message":{"model":"claude-test-1","usage":{"input_tokens":10,"output_tokens":5}},"timestamp":"2026-06-29T00:00:01Z","sessionId":"s1","uuid":"u1"}"#,
            r#"{"type":"assistant","message":{"model":"claude-test-2","usage":{"input_tokens":20,"output_tokens":10}},"timestamp":"2026-06-29T00:00:02Z","sessionId":"s2","uuid":"u2"}"#,
        ];
        let path = write_jsonl(&tmp, "session.jsonl", &lines);
        let path_str = path.to_string_lossy().to_string();
        eprintln!("[int1] path={}", path_str);

        let (db, svc, parsers) = make_runtime();
        eprintln!("[int1] runtime built");

        let outcome = process_one_file_inner(
            &path_str,
            "claude-code",
            &db,
            &svc,
            &parsers,
        )
        .expect("process should succeed");
        eprintln!("[int1] outcome={:?}", outcome);

        
        assert_eq!(outcome.imported, 2, "two assistant lines ingested");
        assert_eq!(outcome.skipped, 0);
        assert_eq!(outcome.truncated, false, "no truncation on fresh cursor");
        assert!(outcome.latest_at.is_some(), "latest_at set");

        
        let guard = db.lock().unwrap();
        let c = guard.get_cursor("claude-code", &path_str).unwrap().unwrap();
        assert_eq!(c.byte_offset, std::fs::metadata(&path).unwrap().len() as i64);
        assert_eq!(c.file_size, c.byte_offset);

        
        let total = guard.count_token_usage_records_filtered(None, None, None, None, None).unwrap();
        assert_eq!(total, 2, "two rows persisted");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    
    
    #[test]
    fn integration_incremental_scan_advances_cursor_and_skips_existing() {
        let tmp = std::env::temp_dir().join(format!("kp-int2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let initial = [
            r#"{"type":"assistant","message":{"model":"m1","usage":{"input_tokens":1,"output_tokens":1}},"timestamp":"2026-06-29T00:00:00Z","sessionId":"s1","uuid":"u1"}"#,
        ];
        let path = write_jsonl(&tmp, "s.jsonl", &initial);
        let path_str = path.to_string_lossy().to_string();

        let (db, svc, parsers) = make_runtime();

        
        let out1 = process_one_file_inner(&path_str, "claude-code", &db, &svc, &parsers).unwrap();
        assert_eq!(out1.imported, 1);
        let size_after_first = std::fs::metadata(&path).unwrap().len() as i64;

        
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(f).unwrap();
        writeln!(
            f,
            r#"{{"type":"assistant","message":{{"model":"m2","usage":{{"input_tokens":2,"output_tokens":2}}}},"timestamp":"2026-06-29T00:00:01Z","sessionId":"s2","uuid":"u2"}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"user","message":{{"role":"user","content":"x"}},"timestamp":"2026-06-29T00:00:02Z"}}"#
        )
        .unwrap();
        drop(f);

        
        let out2 = process_one_file_inner(&path_str, "claude-code", &db, &svc, &parsers).unwrap();
        assert_eq!(
            out2.imported, 1,
            "only the new assistant line is ingested"
        );
        assert_eq!(out2.truncated, false, "no truncation");

        
        let guard = db.lock().unwrap();
        let c = guard.get_cursor("claude-code", &path_str).unwrap().unwrap();
        assert!(
            c.byte_offset > size_after_first,
            "cursor advanced past the new bytes"
        );
        assert_eq!(
            c.byte_offset,
            std::fs::metadata(&path).unwrap().len() as i64
        );

        
        let total = guard.count_token_usage_records_filtered(None, None, None, None, None).unwrap();
        assert_eq!(total, 2, "no duplicate rows from FNV-1a dedup");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    
    
    #[test]
    fn integration_truncation_resets_cursor() {
        let tmp = std::env::temp_dir().join(format!("kp-int3-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let initial = [r#"{"type":"assistant","message":{"model":"m1","usage":{"input_tokens":1,"output_tokens":1}},"timestamp":"2026-06-29T00:00:00Z","sessionId":"s","uuid":"u"}"#];
        let path = write_jsonl(&tmp, "t.jsonl", &initial);
        let path_str = path.to_string_lossy().to_string();

        let (db, svc, parsers) = make_runtime();
        process_one_file_inner(&path_str, "claude-code", &db, &svc, &parsers).unwrap();

        
        std::fs::write(&path, "").unwrap();

        let outcome = process_one_file_inner(&path_str, "claude-code", &db, &svc, &parsers).unwrap();
        assert!(outcome.truncated, "truncation flag set");
        assert_eq!(outcome.imported, 0, "no lines to parse");

        
        let guard = db.lock().unwrap();
        let c = guard.get_cursor("claude-code", &path_str).unwrap().unwrap();
        assert_eq!(c.byte_offset, 0);

        
        let total = guard.count_token_usage_records_filtered(None, None, None, None, None).unwrap();
        assert_eq!(total, 1, "no duplicate rows after truncate + rescan");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}


#[cfg(test)]
#[allow(dead_code)]
fn _unused_imports() {
    use crate::services::agent_parser::ParseStats;
    let _: ParseStats = ParseStats::empty();
}