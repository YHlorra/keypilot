// src-tauri/tests/ipc_e2e.rs
// Phase 2 Lane C — IPC E2E integration tests.
// @see openspec/changes/v0.1-general-credentials/design.md §7 for command signatures
//
// Integration tests live in `tests/` and are a separate crate from the library.
// These tests exercise command function bodies directly via AppState,
// bypassing the Tauri webview/IPC layer entirely.
// This avoids linking against WebView2Loader.dll (which causes STATUS_ENTRYPOINT_NOT_FOUND
// on systems where MSVC toolchain and UCRT versions don't match exactly).

use keypilot_lib::database::Database;
use keypilot_lib::store::AppState;
use keypilot_lib::services::provider::{
    AddProviderRequest, AddProviderFieldRequest, UpdateProviderRequest, UpdateProviderFieldRequest,
};
use keypilot_lib::services::category::AddCategoryRequest;
use keypilot_lib::commands::provider::{
    list_providers, get_provider, add_provider, update_provider, delete_provider,
    list_categories, add_category, delete_category, test_connection, get_theme, set_theme,
};
use keypilot_lib::commands::quota::{fetch_quota, set_manual_quota, SetManualQuotaRequest};
use keypilot_lib::types::{Theme, QuotaSnapshot, Visibility};
use serde::de::DeserializeOwned;
use tauri::State;

fn build_test_state() -> AppState {
    let db = Database::open_in_memory().expect("Failed to open in-memory DB");
    db.setup_schema().expect("Failed to setup schema");
    db.seed_preset_providers().expect("Failed to seed presets");
    AppState::new(db)
}

// Run an async future to completion on a dedicated single-threaded tokio runtime.
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime")
        .block_on(future)
}

// Call a command returning Result<T, AppError> where T: DeserializeOwned,
// asserting success and returning the deserialized value.
fn call_ok<T: DeserializeOwned>(label: &str, value: Result<T, keypilot_lib::error::AppError>) -> T {
    match value {
        Ok(v) => v,
        Err(e) => panic!("{} failed: {:?}", label, e),
    }
}

fn call_unit(label: &str, value: Result<(), keypilot_lib::error::AppError>) {
    if let Err(e) = value {
        panic!("{} failed: {:?}", label, e);
    }
}

// Construct Tauri State wrapper from a reference without a Tauri App.
// State<'r, T>(&'r T) has identical layout to &'r T.
fn as_state<'r>(r: &'r AppState) -> State<'r, AppState> {
    unsafe { std::mem::transmute(r) }
}

#[test]
fn e2e_list_providers() {
    let state = build_test_state();
    let response = call_ok("list_providers", block_on(list_providers(as_state(&state))));
    // 5 presets seeded
    assert!(response.len() >= 5, "Expected >= 5 preset providers, got {}", response.len());
}

#[test]
fn e2e_get_provider() {
    let state = build_test_state();
    let response = call_ok("get_provider", block_on(get_provider(as_state(&state), 1)));
    assert_eq!(response.id, 1, "Expected provider with id=1");
}

#[test]
fn e2e_add_provider() {
    let state = build_test_state();
    let req = AddProviderRequest {
        name: "Test Provider".to_string(),
        preset: None,
        category_id: 1,
        pinned: None,
        notes: None,
        icon: None,
        icon_color: None,
        fields: Some(vec![]),
    };
    let response = call_ok("add_provider", block_on(add_provider(as_state(&state), req)));
    assert!(response.id >= 6, "Expected new provider id >= 6, got {}", response.id);
    assert_eq!(response.name, "Test Provider");
}

#[test]
fn e2e_update_provider() {
    let state = build_test_state();
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
    let response = call_ok("update_provider", block_on(update_provider(as_state(&state), req)));
    assert_eq!(response.name, "Updated OpenAI");
    assert_eq!(response.id, 1);
}

#[test]
fn e2e_delete_provider() {
    let state = build_test_state();
    // First add a provider to delete
    let add_req = AddProviderRequest {
        name: "To Delete".to_string(),
        preset: None,
        category_id: 1,
        pinned: None,
        notes: None,
        icon: None,
        icon_color: None,
        fields: Some(vec![]),
    };
    let new_provider = call_ok("add_provider", block_on(add_provider(as_state(&state), add_req)));
    let new_id = new_provider.id;

    // Now delete it
    call_unit("delete_provider", block_on(delete_provider(as_state(&state), new_id)));
}

#[test]
fn e2e_list_categories() {
    let state = build_test_state();
    let response = call_ok("list_categories", block_on(list_categories(as_state(&state))));
    assert!(response.len() >= 1, "Expected >= 1 category (default), got {}", response.len());
}

#[test]
fn e2e_add_category() {
    let state = build_test_state();
    let req = AddCategoryRequest {
        name: "Test Category".to_string(),
    };
    let response = call_ok("add_category", block_on(add_category(as_state(&state), req)));
    assert!(response.id > 0);
    assert_eq!(response.name, "Test Category");
}

#[test]
fn e2e_delete_category() {
    let state = build_test_state();
    let add_req = AddCategoryRequest {
        name: "To Delete Category".to_string(),
    };
    let new_cat = call_ok("add_category", block_on(add_category(as_state(&state), add_req)));
    let new_cat_id = new_cat.id;

    // Delete it, migrate providers to category 1
    let req = keypilot_lib::services::category::DeleteCategoryRequest {
        id: new_cat_id,
        migrate_to: 1,
    };
    call_unit("delete_category", block_on(delete_category(as_state(&state), req)));
}

#[test]
fn e2e_get_theme() {
    let state = build_test_state();
    let response = call_ok("get_theme", block_on(get_theme(as_state(&state))));
    // Theme is always one of dark/light/auto
    assert!(matches!(response, Theme::Dark | Theme::Light | Theme::Auto));
}

#[test]
fn e2e_set_theme() {
    let state = build_test_state();
    // Set theme to dark
    call_unit("set_theme", block_on(set_theme(as_state(&state), Theme::Dark)));

    // Verify theme was actually set
    let theme = call_ok("get_theme", block_on(get_theme(as_state(&state))));
    assert!(matches!(theme, Theme::Dark));
}

// ============== V0.1 rev2 — manual quota + atomic field replace ==============

#[test]
fn e2e_set_manual_quota_persists_and_fetch_returns_it() {
    let state = build_test_state();
    let req = SetManualQuotaRequest {
        id: 1, // OpenAI preset
        snapshot: QuotaSnapshot {
            total: Some(100.0),
            used: 25.0,
            remaining: Some(75.0),
            unit: "USD".to_string(),
            level: Some("green".to_string()),
            reset_at: None,
        },
    };

    call_unit("set_manual_quota", block_on(set_manual_quota(as_state(&state), req)));

    // fetch_quota must return the manual snapshot (source='manual' bypasses TTL).
    let quota = call_ok("fetch_quota", block_on(fetch_quota(as_state(&state), 1)));
    assert_eq!(quota.unit, "USD");
    assert_eq!(quota.used, 25.0);
    assert_eq!(quota.remaining, Some(75.0));
    assert_eq!(quota.total, Some(100.0));
    assert_eq!(quota.level, Some("green".to_string()));
}

#[test]
fn e2e_set_manual_quota_provider_not_found() {
    let state = build_test_state();
    let req = SetManualQuotaRequest {
        id: 99999, // not seeded
        snapshot: QuotaSnapshot {
            total: None,
            used: 0.0,
            remaining: None,
            unit: "token".to_string(),
            level: None,
            reset_at: None,
        },
    };
    let result = block_on(set_manual_quota(as_state(&state), req));
    assert!(result.is_err(), "Expected error for non-existent provider id");
    let err = result.unwrap_err();
    let err_val = serde_json::to_value(&err).unwrap();
    assert_eq!(
        err_val["code"], "PROVIDER_NOT_FOUND",
        "Expected PROVIDER_NOT_FOUND error code, got {:?}", err_val
    );
}

#[test]
fn e2e_update_provider_fields_replace_atomic() {
    let state = build_test_state();
    // Add a provider with 2 initial fields
    let add_req = AddProviderRequest {
        name: "Atomic Test".to_string(),
        preset: Some("openai".to_string()),
        category_id: 1,
        pinned: None,
        notes: None,
        icon: None,
        icon_color: None,
        fields: Some(vec![
            AddProviderFieldRequest {
                key: "field1".to_string(),
                value: "value1".to_string(),
                visibility: Visibility::Visible,
                sort_index: 0,
            },
            AddProviderFieldRequest {
                key: "field2".to_string(),
                value: "value2".to_string(),
                visibility: Visibility::Visible,
                sort_index: 1,
            },
        ]),
    };
    let new_provider = call_ok("add_provider", block_on(add_provider(as_state(&state), add_req)));
    let new_id = new_provider.id;
    assert_eq!(new_provider.fields.len(), 2, "Initial fields should be 2");

    // Replace fields with a single new one
    let update_req = UpdateProviderRequest {
        id: new_id,
        name: None,
        category_id: None,
        pinned: None,
        notes: None,
        icon: None,
        icon_color: None,
        fields: Some(vec![UpdateProviderFieldRequest {
            key: "new_field".to_string(),
            value: "new_value".to_string(),
            visibility: Visibility::Visible,
            sort_index: 0,
        }]),
    };
    let response = call_ok("update_provider", block_on(update_provider(as_state(&state), update_req)));

    // Atomicity verification: fields were replaced (not wiped, not duplicated)
    assert_eq!(
        response.fields.len(),
        1,
        "Field replace should leave exactly 1 field, got {}",
        response.fields.len()
    );
    assert_eq!(response.fields[0].key, "new_field");
    assert_eq!(response.fields[0].value, "new_value");
}

// Ensure the test_connection path doesn't panic. Mock environment cannot reach external
// APIs, so the call is expected to fail — we only assert the error is well-formed.
#[test]
fn e2e_test_connection_does_not_panic() {
    let state = build_test_state();
    let result = block_on(test_connection(as_state(&state), 1));
    // Either Ok (unexpected) or Err with a structured AppError. The important
    // thing is that the call does not panic and the error code is well-formed.
    if let Err(e) = result {
        let err_val = serde_json::to_value(&e).unwrap();
        assert!(
            err_val.get("code").is_some(),
            "Expected AppError with code field, got {:?}",
            err_val
        );
    }
}