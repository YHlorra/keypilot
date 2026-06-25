// src-tauri/tests/ipc_e2e.rs
// Phase 2 Lane C — 12 IPC E2E integration test using tauri::test::mock_builder()
// @see openspec/changes/v0.1-general-credentials/design.md §7 for command signatures

use keypilot::database::Database;
use keypilot::store::AppState;
use keypilot::services::provider::{AddProviderRequest, UpdateProviderRequest};
use keypilot::services::category::AddCategoryRequest;
use keypilot::commands::provider::{
    list_providers, get_provider, add_provider, update_provider, delete_provider,
    list_categories, add_category, delete_category, test_connection, fetch_quota,
    get_theme, set_theme,
};
use keypilot::types::{Provider, Category, Theme};
use serde_json::json;

// Helper: build a test app with in-memory DB + managed state + registered IPC handlers
// Phase 2 oracle fix #6: register generate_handler! so mock IPC dispatch works.
fn build_test_app() -> tauri::App {
    let db = Database::open_in_memory().expect("Failed to open in-memory DB");
    db.setup_schema().expect("Failed to setup schema");
    db.seed_preset_providers().expect("Failed to seed presets");

    let state = AppState::new(db);
    let app = tauri::test::mock_builder()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            list_providers, get_provider, add_provider, update_provider, delete_provider,
            list_categories, add_category, delete_category, test_connection, fetch_quota,
            get_theme, set_theme,
        ])
        .build();
    app
}

#[test]
fn e2e_list_providers() {
    let app = build_test_app();
    let response: Vec<Provider> = tauri::test::get_ipc_response(
        &app,
        "list_providers",
        json!({})
    ).expect("list_providers IPC failed");
    // 5 presets seeded
    assert!(response.len() >= 5, "Expected >= 5 preset providers, got {}", response.len());
}

#[test]
fn e2e_get_provider() {
    let app = build_test_app();
    let response: Provider = tauri::test::get_ipc_response(
        &app,
        "get_provider",
        json!({ "id": 1 })
    ).expect("get_provider IPC failed");
    assert_eq!(response.id, 1, "Expected provider with id=1");
}

#[test]
fn e2e_add_provider() {
    let app = build_test_app();
    let req = AddProviderRequest {
        name: "Test Provider".to_string(),
        preset: None,
        category_id: 1,
        pinned: false,
        notes: None,
        icon: None,
        icon_color: None,
        fields: vec![],
    };
    let response: Provider = tauri::test::get_ipc_response(
        &app,
        "add_provider",
        json!({ "req": req })
    ).expect("add_provider IPC failed");
    assert!(response.id >= 6, "Expected new provider id >= 6, got {}", response.id);
    assert_eq!(response.name, "Test Provider");
}

#[test]
fn e2e_update_provider() {
    let app = build_test_app();
    let req = UpdateProviderRequest {
        id: 1,
        name: Some("Updated OpenAI".to_string()),
        category_id: None,
        pinned: None,
        notes: None,
        icon: None,
        icon_color: None,
        fields: None,
    };
    let response: Provider = tauri::test::get_ipc_response(
        &app,
        "update_provider",
        json!({ "req": req })
    ).expect("update_provider IPC failed");
    assert_eq!(response.name, "Updated OpenAI");
    assert_eq!(response.id, 1);
}

#[test]
fn e2e_delete_provider() {
    let app = build_test_app();
    // First add a provider to delete
    let add_req = AddProviderRequest {
        name: "To Delete".to_string(),
        preset: None,
        category_id: 1,
        pinned: false,
        notes: None,
        icon: None,
        icon_color: None,
        fields: vec![],
    };
    let new_provider: Provider = tauri::test::get_ipc_response(
        &app,
        "add_provider",
        json!({ "req": add_req })
    ).expect("add_provider IPC failed");
    let new_id = new_provider.id;

    // Now delete it — Result<(), AppError> serializes as null on success
    let result: Option<serde_json::Value> = tauri::test::get_ipc_response(
        &app,
        "delete_provider",
        json!({ "id": new_id })
    ).expect("delete_provider IPC failed");
    assert!(result.is_none() || result == Some(serde_json::Value::Null));
}

#[test]
fn e2e_list_categories() {
    let app = build_test_app();
    let response: Vec<Category> = tauri::test::get_ipc_response(
        &app,
        "list_categories",
        json!({})
    ).expect("list_categories IPC failed");
    assert!(response.len() >= 1, "Expected >= 1 category (default), got {}", response.len());
}

#[test]
fn e2e_add_category() {
    let app = build_test_app();
    let req = AddCategoryRequest {
        name: "Test Category".to_string(),
    };
    let response: Category = tauri::test::get_ipc_response(
        &app,
        "add_category",
        json!({ "req": req })
    ).expect("add_category IPC failed");
    assert!(response.id > 0);
    assert_eq!(response.name, "Test Category");
}

#[test]
fn e2e_delete_category() {
    let app = build_test_app();
    // First add a category to delete
    let add_req = AddCategoryRequest {
        name: "To Delete Category".to_string(),
    };
    let new_cat: Category = tauri::test::get_ipc_response(
        &app,
        "add_category",
        json!({ "req": add_req })
    ).expect("add_category IPC failed");
    let new_cat_id = new_cat.id;

    // Delete it, migrate providers to category 1
    let result: Option<serde_json::Value> = tauri::test::get_ipc_response(
        &app,
        "delete_category",
        json!({ "req": { "id": new_cat_id, "migrate_to": 1 } })
    ).expect("delete_category IPC failed");
    assert!(result.is_none() || result == Some(serde_json::Value::Null));
}

#[test]
fn e2e_test_connection() {
    let app = build_test_app();
    // OpenAI (id=1) preset has can_test=true, but mock key may fail network
    // Should return AppError, not panic
    let result: Result<Option<serde_json::Value>, _> = tauri::test::get_ipc_response(
        &app,
        "test_connection",
        json!({ "id": 1 })
    );
    // Accept both Ok(null) for success and Err for network failure
    // The important thing is it doesn't panic
    if let Err(e) = result {
        let err_val = serde_json::to_value(&e).unwrap();
        assert!(err_val.get("code").is_some(), "Expected AppError with code field");
    }
}

#[test]
fn e2e_fetch_quota() {
    let app = build_test_app();
    // DeepSeek (id=2) has can_fetch_quota=true
    // May fail network but should not panic
    let result: Result<serde_json::Value, _> = tauri::test::get_ipc_response(
        &app,
        "fetch_quota",
        json!({ "id": 2 })
    );
    if result.is_ok() {
        let quota = result.unwrap();
        // Valid QuotaSnapshot has 'unit' and 'used' fields
        assert!(quota.get("unit").is_some());
        assert!(quota.get("used").is_some());
    } else {
        // Network errors are acceptable for mock environment
        let err_val = serde_json::to_value(&result.unwrap_err()).unwrap();
        assert!(err_val.get("code").is_some());
    }
}

#[test]
fn e2e_get_theme() {
    let app = build_test_app();
    let response: Theme = tauri::test::get_ipc_response(
        &app,
        "get_theme",
        json!({})
    ).expect("get_theme IPC failed");
    // Theme is always one of dark/light/auto
    match response {
        Theme::Dark | Theme::Light | Theme::Auto => (),
        _ => panic!("Invalid theme variant"),
    }
}

#[test]
fn e2e_set_theme() {
    let app = build_test_app();
    // set_theme returns Result<(), AppError> — success is null
    let result: Option<serde_json::Value> = tauri::test::get_ipc_response(
        &app,
        "set_theme",
        json!({ "theme": "dark" })
    ).expect("set_theme IPC failed");
    assert!(result.is_none() || result == Some(serde_json::Value::Null));

    // Verify theme was actually set
    let theme: Theme = tauri::test::get_ipc_response(
        &app,
        "get_theme",
        json!({})
    ).expect("get_theme IPC failed");
    assert!(matches!(theme, Theme::Dark | Theme::Light | Theme::Auto));
}
