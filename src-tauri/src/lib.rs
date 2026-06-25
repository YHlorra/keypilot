pub mod database;
pub mod error;
pub mod store;
pub mod tray;
pub mod types;
pub mod provider;
pub mod services;
pub mod commands;

use database::Database;
use error::AppError;
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
            db.seed_preset_providers()?;

            let state = AppState::new(db);
            app.manage(state);

            // Stage 5: Initialize system tray
            let _tray = tray::init_tray(app.handle())?;

            // Create main window
            let (label, title) = ("main", "KeyPilot");

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
    commands::provider::get_theme,
    commands::provider::set_theme,
    commands::tray::pin_provider,
    commands::tray::unpin_provider,
    commands::tray::quit_app,
])
        .run(tauri::generate_context!())
        .expect("failed to run tauri app");
}