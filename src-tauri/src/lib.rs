pub mod actions;
pub mod database;
pub mod error;
pub mod store;
pub mod timeutil;
pub mod tray;
pub mod types;
pub mod provider;
pub mod services;
pub mod commands;

use database::Database;
use error::AppError;
use services::auto_import;
use services::incremental_import::IncrementalImporter;
use services::token_usage::TokenUsageService;
use store::AppState;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

pub fn run() {
    // Build Tauri app — startup chain runs inside .setup() where app.path() is accessible
    tauri::Builder::default()
        .setup(|app| {
            // Stage 1: app data dir via Tauri 2 API
            let app_dir = app.path().app_data_dir().map_err(|e: tauri::Error| {
                AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;

            std::fs::create_dir_all(&app_dir).map_err(AppError::Io)?;

            let db_path = app_dir.join("keypilot.db");
            let db = Database::open(&db_path)?;
            db.setup_schema()?;
            db.migrate()?;
            // Purge stale auto-fetched quota_cache rows (older than 7 days).
            // Manual entries are preserved. Failure is non-fatal — app still starts.
            if let Err(e) = db.purge_expired_quota_cache(7 * 86400) {
                eprintln!("quota_cache purge failed: {}", e);
            }
            // One-time cleanup: remove seeded preset providers so app starts empty per user feedback (2026-06-26).
            // Safe to run on every startup -- only deletes is_preset=1 rows.
            match db.delete_preset_providers() {
                Ok(n) if n > 0 => eprintln!("Removed {} preset provider(s) from previous startup", n),
                Ok(_) => {},
                Err(e) => eprintln!("Failed to clean preset providers: {}", e),
            }

            let state = AppState::new(db);
            // Clone Arcs before moving state into app.manage — needed for
            // auto-import which runs before the window opens.
            let db_for_import = state.db.clone();
            let pricing_for_import = state.pricing.clone();
            app.manage(state);

            // Clone AppHandle BEFORE entering the closure — `app` is `&mut tauri::App`
            // and can't escape into the spawn_blocking 'static closure.
            let app_handle = app.app_handle().clone();

            // Run auto-import across all available agent parsers (opencode.db,
            // claude-code jsonl files, etc.) in a BACKGROUND thread so webview
            // creation is not blocked by a potentially long JSONL scan (Claude
            // Code projects can be hundreds of MB).  This populates
            // token_usage_records from existing agent data so the heatmap and
            // KPI cards are non-empty on first launch.
            //
            // 2026-06-29 (Bug #3 fix): replaced one-shot `scan_and_import_if_empty`
            // (which became a no-op once the DB had > 100 rows) with
            // `IncrementalImporter` — a file-watching, per-file-cursor scanner
            // that emits `token_usage_tick` events as Claude Code / Codex append
            // new lines to their JSONL session logs.
            tauri::async_runtime::spawn_blocking(move || {
                let svc = TokenUsageService::new(db_for_import.clone(), pricing_for_import.clone());
                let summary = auto_import::scan_and_import_if_empty(&svc);
                let json = serde_json::to_string(&summary).unwrap_or_default();
                if let Err(e) = db_for_import.lock().unwrap().set_meta("last_auto_import", &json) {
                    eprintln!("Failed to store last_auto_import meta: {}", e);
                }

                // Spawn the file watcher + 30s fallback poll loop.  Owns its
                // own threads; dropping the IncrementalImporter shuts them
                // down.  V0.1 keeps it alive for the process lifetime.
                let parsers =
                    services::agent_parser::default_parsers(pricing_for_import.clone());
                let _importer = IncrementalImporter::start(
                    app_handle,
                    db_for_import.clone(),
                    pricing_for_import,
                    parsers,
                );
                // Intentional leak: importer holds debouncer + thread alive
                // for the process lifetime.  V0.1 has no shutdown signal.
                Box::leak(Box::new(_importer));
            });

            // Stage 5: Initialize system tray
            let _tray = tray::init_tray(app.handle())?;

            // Create main window
            let unique_id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let (label, title) = (format!("keypilot-{}", unique_id), "KeyPilot");

            let builder = WebviewWindowBuilder::new(
                app,
                label,
                WebviewUrl::App("index.html".into()),
            )
            .title(title)
            .inner_size(1200.0, 760.0)
            .resizable(true);

            builder.build().map_err(|e| {
                AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
    commands::provider::list_providers,
    commands::provider::get_provider,
    commands::provider::add_provider,
    commands::provider::update_provider,
    commands::provider::delete_provider,
    commands::provider::list_categories,
    commands::provider::add_category,
    commands::provider::delete_category,
    commands::provider::test_connection,
    commands::quota::fetch_quota,
    commands::quota::set_manual_quota,
    commands::provider::get_theme,
    commands::provider::set_theme,
    commands::tray::pin_provider,
    commands::tray::unpin_provider,
    commands::tray::quit_app,
    commands::token_usage::record_usage,
    commands::token_usage::list_usage_records,
    commands::token_usage::get_usage_summary,
    commands::token_usage::get_usage_periods_summary,
    commands::token_usage::import_usage,
    commands::token_usage::import_opencode_db,
    commands::token_usage::get_last_auto_import,
    commands::token_usage::get_pricing,
    commands::token_usage::recompute_costs,
    commands::token_usage::force_rescan_all,
    // Action Registry (Stage 10)
    commands::action::list_actions,
    commands::action::execute_action,
])
        .run(tauri::generate_context!())
        .expect("failed to run tauri app");
}