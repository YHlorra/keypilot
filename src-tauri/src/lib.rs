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
    
    tauri::Builder::default()
        .setup(|app| {
            
            let app_dir = app.path().app_data_dir()?;

            std::fs::create_dir_all(&app_dir).map_err(AppError::Io)?;

            let db_path = app_dir.join("keypilot.db");
            let db = Database::open(&db_path)?;
            db.setup_schema()?;
            db.migrate()?;
            
            
            if let Err(e) = db.purge_expired_quota_cache(7 * 86400) {
                eprintln!("quota_cache purge failed: {}", e);
            }
            
            
            match db.delete_preset_providers() {
                Ok(n) if n > 0 => eprintln!("Removed {} preset provider(s) from previous startup", n),
                Ok(_) => {},
                Err(e) => eprintln!("Failed to clean preset providers: {}", e),
            }

            let state = AppState::new(db);
            
            
            let db_for_import = state.db.clone();
            let pricing_for_import = state.pricing.clone();
            app.manage(state);

            
            
            let app_handle = app.app_handle().clone();

            
            
            
            
            
            
            
            
            
            
            
            
            tauri::async_runtime::spawn_blocking(move || {
                let svc = TokenUsageService::new(db_for_import.clone(), pricing_for_import.clone());
                let summary = auto_import::scan_and_import_if_empty(&svc);
                let json = serde_json::to_string(&summary).unwrap_or_default();
                if let Err(e) = db_for_import.lock().unwrap().set_meta("last_auto_import", &json) {
                    eprintln!("Failed to store last_auto_import meta: {}", e);
                }

                
                
                
                let parsers =
                    services::agent_parser::default_parsers(pricing_for_import.clone());
                let _importer = IncrementalImporter::start(
                    app_handle,
                    db_for_import.clone(),
                    pricing_for_import,
                    parsers,
                );
                
                
                Box::leak(Box::new(_importer));
            });

            
            let _tray = tray::init_tray(app.handle())?;

            
            let (label, title) = ("main".to_string(), "KeyPilot");

            let builder = WebviewWindowBuilder::new(
                app,
                label,
                WebviewUrl::App("index.html".into()),
            )
            .title(title)
            .inner_size(1200.0, 760.0)
            .resizable(true);

            builder.build()?;

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
    commands::quota::fetch_coding_plan_quota,
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
    
    commands::action::list_actions,
    commands::action::execute_action,
])
        .run(tauri::generate_context!())
        .expect("failed to run tauri app");
}