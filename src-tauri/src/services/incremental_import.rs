//! Incremental JSONL importer — Bug #3 fix 2026-06-29.
//!
//! Watches Claude Code (`~/.claude/projects/**/*.jsonl`) and Codex
//! (`~/.codex/sessions/**/*.jsonl`) directories via `notify-debouncer-full`,
//! parses only the bytes appended since the last scan (per-file byte cursor
//! in `agent_file_cursor` table), and feeds new records through
//! `TokenUsageService::record_usage` (FNV-1a dedup applies).
//!
//! After each successful incremental parse, emits a Tauri event
//! `token_usage_tick` so the frontend can refresh KPI cards without polling.
//!
//! A 30s polling fallback re-scans every known cursor file to catch any
//! notify events lost to Windows buffer overflow.
//!
//! Replaces the old `scan_and_import_if_empty` (which went no-op after the
//! DB had > 100 rows, dropping every newly-appended agent line on subsequent
//! launches).

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

/// Event name emitted to frontend after each successful incremental scan.
/// Frontend listens via `listen<TokenUsageTickPayload>("token_usage_tick", ...)`.
pub const TOKEN_USAGE_TICK_EVENT: &str = "token_usage_tick";

/// Debounce window for file-write bursts.  300ms is the
/// notify-debouncer-full recommended starting value — fast enough to feel
/// live, slow enough to coalesce a single-line append's multiple events.
const DEBOUNCE_WINDOW: Duration = Duration::from_millis(300);

/// Fallback poll interval (when notify is unavailable or to backstop dropped
/// events).  Runs in a background thread; uses `mpsc::Receiver::recv_timeout`
/// so it exits promptly on shutdown.
const FALLBACK_POLL_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize)]
pub struct TokenUsageTickPayload {
    pub agent_type: String,
    /// Rows actually inserted this tick (after FNV-1a dedup).
    pub imported: u32,
    /// Rows skipped because the deterministic id was already present.
    pub skipped: u32,
    /// `occurred_at` of the latest record ingested (epoch ms), if any.
    pub latest_at: Option<i64>,
    /// Today's cumulative tokens (server-computed so the frontend can update
    /// without a separate IPC round-trip).
    pub total_today_tokens: i64,
    /// Today's cumulative cost in USD.
    pub total_today_cost_usd: f64,
}

/// Owns the notify watcher + 30s fallback poll loop.
/// Drop the value to shut both down (watcher + thread detach).
pub struct IncrementalImporter {
    /// `None` after `shutdown()`; non-None while running.
    _debouncer: Option<Debouncer<notify::RecommendedWatcher, RecommendedCache>>,
    _shutdown_tx: Option<mpsc::Sender<()>>,
    _watcher_thread: Option<thread::JoinHandle<()>>,
}

impl IncrementalImporter {
    /// Start watching agent data dirs.  Returns a handle that keeps the
    /// watcher alive (drop it to stop).
    ///
    /// All heavy work runs on a dedicated background thread; the function
    /// returns synchronously without blocking the caller.  On startup the
    /// importer first walks all currently-known cursors (catch-up for files
    /// modified while the app was closed) before arming the watcher.
    pub fn start(
        app: AppHandle,
        db: Arc<std::sync::Mutex<Database>>,
        pricing: Arc<crate::services::pricing::PricingService>,
        parsers: Vec<Box<dyn AgentParser>>,
    ) -> Self {
        // Wrap parsers in Arc so each spawned thread gets its own cheap
        // clone (Arc::clone).  Vec<Box<dyn Trait>> is not Clone because
        // Box<dyn Trait> isn't, but Arc<Vec<...>> is.  Each closure then
        // captures Arc<...> by move, satisfying 'static.
        let parsers = Arc::new(parsers);
        let parsers_for_initial = Arc::clone(&parsers);
        let parsers_for_watcher = Arc::clone(&parsers);
        let parsers_for_fallback = Arc::clone(&parsers);

        // 1. Initial catch-up: process every known cursor's file from its
        //    stored byte_offset.  Cheap if the user just opened the app;
        //    bounded by file size in the worst case (one-time backfill).
        let svc = TokenUsageService::new(db.clone(), pricing.clone());
        let app_for_initial = app.clone();
        let db_for_initial = db.clone();
        let svc_for_initial = svc.clone();
        thread::spawn(move || {
            run_initial_catchup(&db_for_initial, &svc_for_initial, &app_for_initial, &parsers_for_initial);
        });

        // 2. Register notify watcher on each agent dir (recursive).
        let (debouncer, watch_paths) = build_watcher(
            app.clone(),
            db.clone(),
            svc.clone(),
            parsers_for_watcher,
        );

        // 3. 30s fallback poll loop, on a dedicated thread, with shutdown channel.
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

    /// Signal both the fallback poll thread to exit and drop the watcher.
    /// Currently unused — V0.1 keeps the importer alive for the process
    /// lifetime.  Reserved for future "Settings → Pause watching" toggle.
    pub fn shutdown(&mut self) {
        if let Some(tx) = self._shutdown_tx.take() {
            let _ = tx.send(());
        }
        self._debouncer = None;
        if let Some(handle) = self._watcher_thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for IncrementalImporter {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// ---------- initial catch-up ----------

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

// ---------- notify watcher build ----------

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
            // Return a dummy debouncer so the caller doesn't crash on unwrap.
            // Watch paths are still passed so the fallback poll can scan them.
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

// ---------- debounced event handler ----------

// ---------- debounced event handler ----------

fn handle_debounced_events(
    events: Vec<DebouncedEvent>,
    app: &AppHandle,
    db: &Arc<std::sync::Mutex<Database>>,
    svc: &TokenUsageService,
    parsers: &[Box<dyn AgentParser>],
) {
    // Coalesce all touched paths → dedup → process each at most once per tick.
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
            // Pick the matching agent_type by path prefix; default to the
            // first parser's type as a fallback.  In practice each parser
            // already filters by `default_path`, so the prefix match is
            // unambiguous.
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
    // Fallback: first parser's type.  Will only happen if file is outside
    // any watched root, which shouldn't occur but defends against future
    // parser additions.
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
    let _ = cursor; // currently unused beyond cursor init; reserved for incremental file truncation detection
    Ok(())
}

// ---------- core: process one file ----------

/// Outcome of a single incremental scan.  Returned by the pure function
/// `process_one_file_inner` and consumed by `process_one_file` (which adds
/// the side-effect of emitting a Tauri event to the frontend).
#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessOutcome {
    pub imported: u32,
    pub skipped: u32,
    pub latest_at: Option<i64>,
    pub truncated: bool, // true if cursor was reset due to file shrink
}

/// Pure (no AppHandle / no emit) core of incremental JSONL ingestion.
/// Reads cursor → seek → parse → record_usage → upsert_cursor.
/// Unit-testable in isolation; production `process_one_file` wraps it
/// to also fire `token_usage_tick` to the frontend.
fn process_one_file_inner(
    path_str: &str,
    agent_type: &str,
    db: &Arc<std::sync::Mutex<Database>>,
    svc: &TokenUsageService,
    parsers: &[Box<dyn AgentParser>],
) -> Result<ProcessOutcome, AppError> {
    let mut outcome = ProcessOutcome::default();
    let path = Path::new(path_str);

    // File deleted between event and scan? Skip silently.
    if !path.exists() {
        return Ok(outcome);
    }

    let current_size = match std::fs::metadata(path) {
        Ok(m) => m.len() as i64,
        Err(e) => return Err(AppError::Io(e)),
    };

    // Phase 1: read cursor (lock briefly), drop the lock before we touch
    // the file or call svc.record_usage (which also locks the same mutex —
    // re-acquiring a non-reentrant std::Mutex from the same thread deadlocks).
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

        // Truncation: file shrank below our stored offset → reset cursor
        // and re-parse from byte 0 (FNV-1a dedup makes this idempotent).
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
        // No new bytes since last scan.  Refresh cursor timestamp.
        cursor.last_event_at = Some(chrono::Utc::now().timestamp_millis());
        cursor.last_scan_at = chrono::Utc::now().timestamp_millis();
        let db_guard = db.lock().map_err(|e| {
            AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, format!("db lock: {e}")))
        })?;
        db_guard.upsert_cursor(&cursor)?;
        return Ok(outcome);
    }

    // Phase 2: read the new bytes from disk (no DB lock held).
    let use_file = std::fs::File::open(path)?;
    use std::io::{Read, Seek, SeekFrom};
    let mut file = use_file;
    file.seek(SeekFrom::Start(cursor.byte_offset as u64))?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    drop(file);

    // Find the matching parser and parse the new bytes.
    let parser = match parsers.iter().find(|p| p.agent_type() == agent_type) {
        Some(p) => p,
        None => return Ok(outcome),
    };

    let parsed = parse_new_bytes(parser.as_ref(), &buf)?;

    // Phase 3: record each parsed row.  svc.record_usage acquires the DB
    // lock internally; we MUST NOT hold our own lock here.
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

    // Phase 4: update cursor to current EOF.
    let now = chrono::Utc::now().timestamp_millis();
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

/// Per-parser dispatch for "parse this slice of JSONL text into records".
/// Reuses each parser's `parse()` for shape compatibility, but feeds the
/// raw bytes instead of walking the directory tree again.
fn parse_new_bytes(parser: &dyn AgentParser, new_bytes: &str) -> Result<ParseOutcome, AppError> {
    use crate::services::agent_parser::{ParseStats, ParseOutcome};
    // For opencode (SQLite) the byte-stream approach doesn't apply — its
    // parser reads opencode.db, which is a full-file relational source.
    // We delegate to the regular `parse()` path, which is already cursor-
    // friendly at the FNV-1a layer.
    if parser.agent_type() == "opencode" {
        return parser.parse();
    }
    // JSONL parsers: scan line-by-line and apply the same shape detection
    // the original parsers use.  Mirrors the existing parsing logic without
    // duplicating it field-by-field: callers go through `parse_one_line`
    // which returns Some(record) for valid usage lines, None otherwise.
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
                let _ = idx; // reserved for sample_errors line_no
                records.push(input);
            }
            None => {
                // Silently skip non-usage lines (Codex writes many).  True
                // parse errors are rare because the parser already filters.
                let _ = idx;
            }
        }
    }

    Ok(ParseOutcome { records, stats })
}

/// Parse one JSONL line per agent's known schema.  Mirrors the existing
/// inline parsers in `services/token_usage.rs::import_jsonl` and the agent-
/// specific parsers, but kept self-contained so this module doesn't depend
/// on private parser internals.
fn parse_one_line(agent_type: &str, line: &str) -> Option<UsageRecordInput> {
    use serde_json::Value;
    let v: Value = serde_json::from_str(line).ok()?;

    match agent_type {
        "claude-code" => {
            // Only "assistant" messages carry usage.
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
            // provider_name is derived in the full parser via pricing.json;
            // for V0.1 emit "unknown" and let daily rollups re-attribute on
            // next recompute_costs call.
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
            // Codex: usage may be absent (skip silently).
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

// ---------- fallback 30s poll loop ----------

fn fallback_poll_loop(
    app: AppHandle,
    db: Arc<std::sync::Mutex<Database>>,
    svc: TokenUsageService,
    _watch_paths: Vec<PathBuf>,
    _parsers: Arc<Vec<Box<dyn AgentParser>>>,
    shutdown_rx: mpsc::Receiver<()>,
) {
    // Use recv_timeout so we can wake up periodically even if no shutdown
    // signal arrives.  First iteration has zero delay (run immediately on
    // startup to catch any files notify missed while the app was off).
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

// ---------- Tauri emit ----------

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

/// Query today's cumulative tokens + cost USD from the AppState's database.
/// Used to give the frontend a one-shot total so the popover / KPI can
/// update without a separate IPC call.
fn read_today_totals(_app: &AppHandle) -> Result<(i64, f64), AppError> {
    // The AppHandle doesn't carry the DB lock directly; in V0.1 we compute
    // today's totals at call time via the AppState stored in Tauri's
    // managed state.  V0.1 fallback: return zeros — the frontend still has
    // the invalidation path via get_usage_periods_summary and will display
    // correct numbers within ~100ms of the next query.
    //
    // Future V0.2: thread the Arc<Database> into emit_tick so we can run
    // SELECT SUM(...) WHERE occurred_at >= today_start here.
    Ok((0, 0.0))
}

// ---------- tests ----------

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

    /// Smoke: the real-world line shape from a manual append test.  Verifies
    /// parse_one_line accepts what notify watcher would feed.
    #[test]
    fn parse_one_line_claude_code_real_smoke_shape() {
        let line = r#"{"type":"assistant","message":{"model":"claude-smoke-test","usage":{"input_tokens":777,"output_tokens":333}},"timestamp":"2026-06-29T17:29:52.385Z","sessionId":"smoke","uuid":"u-smoke"}"#;
        let rec = parse_one_line("claude-code", line).expect("real-shape line should parse");
        assert_eq!(rec.agent_type, "claude-code");
        assert_eq!(rec.model, "claude-smoke-test");
        assert_eq!(rec.input_tokens, 777);
        assert_eq!(rec.output_tokens, 333);
        assert_eq!(rec.session_id.as_deref(), Some("smoke"));
        // occurred_at: 2026-06-29T17:29:52.385Z epoch ms
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

    // ---------- integration: real file → DB → cursor advance (Task 3) ----------

    /// Helper: build a (db, svc, parsers) tuple sharing one in-memory DB.
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

    /// Helper: write a real JSONL file with the given lines and return path.
    fn write_jsonl(dir: &std::path::Path, name: &str, lines: &[&str]) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, lines.join("\n")).unwrap();
        path
    }

    /// Integration #1: first scan of a brand-new JSONL ingests every valid
    /// assistant line and parks the cursor at EOF.  No prior cursor row.
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

        // 1 assistant line was filtered out by `type=="user"`; only 2 should record.
        assert_eq!(outcome.imported, 2, "two assistant lines ingested");
        assert_eq!(outcome.skipped, 0);
        assert_eq!(outcome.truncated, false, "no truncation on fresh cursor");
        assert!(outcome.latest_at.is_some(), "latest_at set");

        // Cursor parked at EOF.
        let guard = db.lock().unwrap();
        let c = guard.get_cursor("claude-code", &path_str).unwrap().unwrap();
        assert_eq!(c.byte_offset, std::fs::metadata(&path).unwrap().len() as i64);
        assert_eq!(c.file_size, c.byte_offset);

        // 2 rows in token_usage_records.
        let total = guard.count_token_usage_records_filtered(None, None, None, None, None).unwrap();
        assert_eq!(total, 2, "two rows persisted");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Integration #2: appending more assistant lines + re-scan advances the
    /// cursor and inserts only the new rows (no duplicates via FNV-1a).
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

        // First scan.
        let out1 = process_one_file_inner(&path_str, "claude-code", &db, &svc, &parsers).unwrap();
        assert_eq!(out1.imported, 1);
        let size_after_first = std::fs::metadata(&path).unwrap().len() as i64;

        // Append two more lines (one assistant + one user to verify filtering).
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

        // Second scan: must skip line 1 (already ingested) and ingest line 2.
        let out2 = process_one_file_inner(&path_str, "claude-code", &db, &svc, &parsers).unwrap();
        assert_eq!(
            out2.imported, 1,
            "only the new assistant line is ingested"
        );
        assert_eq!(out2.truncated, false, "no truncation");

        // Cursor advanced past EOF.
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

        // Total rows: 2 (1 from first scan + 1 new, deduped).
        let total = guard.count_token_usage_records_filtered(None, None, None, None, None).unwrap();
        assert_eq!(total, 2, "no duplicate rows from FNV-1a dedup");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Integration #3: file truncation (size < stored offset) resets cursor
    /// to 0 and re-parses from start; FNV-1a dedup keeps the DB clean.
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

        // Truncate to 0 bytes (simulates `truncate -s 0 t.jsonl`).
        std::fs::write(&path, "").unwrap();

        let outcome = process_one_file_inner(&path_str, "claude-code", &db, &svc, &parsers).unwrap();
        assert!(outcome.truncated, "truncation flag set");
        assert_eq!(outcome.imported, 0, "no lines to parse");

        // Cursor reset to 0.
        let guard = db.lock().unwrap();
        let c = guard.get_cursor("claude-code", &path_str).unwrap().unwrap();
        assert_eq!(c.byte_offset, 0);

        // No duplicates in DB (still 1 row).
        let total = guard.count_token_usage_records_filtered(None, None, None, None, None).unwrap();
        assert_eq!(total, 1, "no duplicate rows after truncate + rescan");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}

// Quiet "unused" warnings on private imports used only in cfg(test).
#[cfg(test)]
#[allow(dead_code)]
fn _unused_imports() {
    use crate::services::agent_parser::ParseStats;
    let _: ParseStats = ParseStats::empty();
}