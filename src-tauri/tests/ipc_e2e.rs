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
use keypilot_lib::commands::token_usage::{
    record_usage, list_usage_records, get_usage_summary, get_usage_periods_summary,
    get_usage_periods_summary_by_state,
    import_usage, get_pricing,
    RecordUsageRequest, ListUsageRecordsRequest, UsageFilterIpc, UsageRecordInputIpc,
    TokenBreakdownIpc,
};
use keypilot_lib::services::token_usage::{TokenUsageService, deterministic_id};
use keypilot_lib::types::{
    Theme, QuotaSnapshot, Visibility,
    UsageFilter as RustUsageFilter,
    UsageRecordInput as RustUsageRecordInput,
};
use keypilot_lib::commands::token_usage::PeriodsSummaryResponse;
use serde::de::DeserializeOwned;
use tauri::State;

fn build_test_state() -> AppState {
    let db = Database::open_in_memory().expect("Failed to open in-memory DB");
    db.setup_schema().expect("Failed to setup schema");
    db.migrate().expect("Failed to migrate schema");
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
        snapshot: QuotaSnapshot::legacy(
            Some(100.0),
            25.0,
            Some(75.0),
            "USD",
            Some("green".to_string()),
            None,
        ),
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
        snapshot: QuotaSnapshot::legacy(
            None,
            0.0,
            None,
            "token",
            None,
            None,
        ),
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
            "Expected AppError with code field, got {:?}", err_val
        );
    }
}

// ============== Stage C — Token Usage IPC E2E ==============

#[test]
fn e2e_record_usage_happy_path() {
    let state = build_test_state();
    let req = RecordUsageRequest {
        req: UsageRecordInputIpc {
            occurred_at: "2026-06-26T10:00:00Z".into(),
            finished_at: None,
            latency_ms: Some(1200),
            provider: "openai".into(),
            model: "gpt-4o".into(),
            agent_type: Some("claude-code".into()),
            user_id: None,
            session_id: Some("sess-1".into()),
            observation_type: None,
            status: None,
            error_code: None,
            cache_hit: None,
            usage_details: TokenBreakdownIpc {
                input: Some(1000),
                output: Some(500),
                cache_read: Some(200),
                cache_creation: None,
                reasoning: None,
            },
            cost_details: None,
            pricing_version: None,
            messages: None,
            response: None,
            tags: None,
        },
    };
    let response = call_ok("record_usage", block_on(record_usage(as_state(&state), req)));
    assert!(!response.id.is_empty());
    assert_eq!(response.provider, "openai");
    assert_eq!(response.model, "gpt-4o");
    assert_eq!(response.agent_type, "claude");
    assert_eq!(response.total_tokens, 1700);
}

#[test]
fn e2e_list_usage_records_pagination() {
    let state = build_test_state();
    // Insert 3 records
    for i in 0..3u64 {
        let req = RecordUsageRequest {
            req: UsageRecordInputIpc {
                occurred_at: format!("2026-06-26T10:0{}:00Z", i),
                finished_at: None,
                latency_ms: None,
                provider: "openai".into(),
                model: "gpt-4o".into(),
                agent_type: Some("claude-code".into()),
                user_id: None,
                session_id: None,
                observation_type: None,
                status: None,
                error_code: None,
                cache_hit: None,
                usage_details: TokenBreakdownIpc {
                    input: Some(100),
                    output: Some(50),
                    cache_read: None,
                    cache_creation: None,
                    reasoning: None,
                },
                cost_details: None,
                pricing_version: None,
                messages: None,
                response: None,
                tags: None,
            },
        };
        let rec = call_ok("record_usage", block_on(record_usage(as_state(&state), req)));
        assert!(!rec.id.is_empty());
    }

    let list_req = ListUsageRecordsRequest {
        filter: UsageFilterIpc::default(),
        page: 1,
        per_page: 10,
    };
    let response = call_ok("list_usage_records", block_on(list_usage_records(as_state(&state), list_req)));
    assert_eq!(response.total, 3);
    assert_eq!(response.page, 1);
    assert_eq!(response.per_page, 10);
    assert_eq!(response.items.len(), 3);
}

#[test]
fn e2e_get_usage_summary_aggregates() {
    let state = build_test_state();
    let req = RecordUsageRequest {
        req: UsageRecordInputIpc {
            occurred_at: "2026-06-26T10:00:00Z".into(),
            finished_at: None,
            latency_ms: None,
            provider: "openai".into(),
            model: "gpt-4o".into(),
            agent_type: Some("claude-code".into()),
            user_id: None,
            session_id: None,
            observation_type: None,
            status: None,
            error_code: None,
            cache_hit: None,
            usage_details: TokenBreakdownIpc {
                input: Some(1000),
                output: Some(500),
                cache_read: None,
                cache_creation: None,
                reasoning: None,
            },
            cost_details: None,
            pricing_version: None,
            messages: None,
            response: None,
            tags: None,
        },
    };
    let rec = call_ok("record_usage", block_on(record_usage(as_state(&state), req)));
    assert!(!rec.id.is_empty());

    let summary = call_ok("get_usage_summary", block_on(get_usage_summary(as_state(&state), UsageFilterIpc::default())));
    assert_eq!(summary.total_requests, 1);
    assert_eq!(summary.total_tokens, 1500);
    assert_eq!(summary.agent_pairs.len(), 1);
    assert_eq!(summary.agent_pairs[0].agent_type, "claude");
    assert_eq!(summary.agent_pairs[0].model, "gpt-4o");
    assert_eq!(summary.agent_pairs[0].provider, "openai");
}

#[test]
fn e2e_get_usage_periods_summary() {
    let state = build_test_state();

    // 注入 1 条"今天"的测试数据(直接构造 TokenUsageService 调用底层
    // record_usage,绕过 IPC ISO 字符串解析,直接用 epoch 毫秒)
    let now_ms = chrono::Local::now().timestamp_millis();
    let input = RustUsageRecordInput {
        agent_type: "claude-code".into(),
        model: "gpt-4o".into(),
        provider_name: "openai".into(),
        occurred_at: now_ms,
        session_id: Some("s1".into()),
        request_id: Some("r1".into()),
        input_tokens: 100,
        output_tokens: 50,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        reasoning_tokens: 0,
        usage_details: None,
    };
    let svc = TokenUsageService::new(state.db.clone(), state.pricing.clone());
    let id = deterministic_id(&input);
    svc.record_usage(&id, input).expect("record_usage should succeed");

    // 通过 by_state 入口调用(避免 transmute State 的复杂度)
    let filter = UsageFilterIpc::default();
    let summary: PeriodsSummaryResponse = call_ok(
        "get_usage_periods_summary",
        block_on(get_usage_periods_summary_by_state(&state, filter)),
    );

    // today 应有 1 条
    assert_eq!(summary.periods.today.total_requests, 1,
        "today period should have 1 request");
    // month 应有 1 条(今天也在本月范围内)
    assert_eq!(summary.periods.month.total_requests, 1,
        "month period should have 1 request");
    // all_time 应有 1 条
    assert_eq!(summary.periods.all_time.total_requests, 1,
        "all_time period should have 1 request");

    // period_windows 应有 today + month
    assert!(!summary.period_windows.today.key.is_empty(),
        "today window key should not be empty");
    assert!(!summary.period_windows.month.key.is_empty(),
        "month window key should not be empty");
    // ends_at 是 ISO 8601 字符串(包含 "T" 分隔符)
    assert!(summary.period_windows.today.ends_at.contains("T"),
        "today ends_at should be ISO 8601 with 'T' separator, got: {}",
        summary.period_windows.today.ends_at);
    assert!(summary.period_windows.month.ends_at.contains("T"),
        "month ends_at should be ISO 8601 with 'T' separator, got: {}",
        summary.period_windows.month.ends_at);

    // client_models 应有 "claude"("claude-code" 被 normalize_agent_type 规范化为 "claude")
    assert!(!summary.client_models.is_empty(),
        "client_models should not be empty");
    assert!(summary.client_models.contains_key("claude"),
        "client_models should contain 'claude' (normalized from 'claude-code'), got: {:?}",
        summary.client_models.keys().collect::<Vec<_>>());

    // limits 应为 None(无 quota_cache 数据)
    assert!(summary.limits.is_none(),
        "limits should be None when quota_cache is empty");
}

#[test]
fn e2e_get_usage_periods_summary_via_ipc_command() {
    // 同样场景但通过 #[tauri::command] 入口(as_state transmute)验证
    // generate_handler! 注册路径可用。
    let state = build_test_state();

    let now_ms = chrono::Local::now().timestamp_millis();
    let input = RustUsageRecordInput {
        agent_type: "claude-code".into(),
        model: "gpt-4o".into(),
        provider_name: "openai".into(),
        occurred_at: now_ms,
        session_id: Some("s2".into()),
        request_id: Some("r2".into()),
        input_tokens: 200,
        output_tokens: 100,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        reasoning_tokens: 0,
        usage_details: None,
    };
    let svc = TokenUsageService::new(state.db.clone(), state.pricing.clone());
    let id = deterministic_id(&input);
    svc.record_usage(&id, input).expect("record_usage should succeed");

    let filter = UsageFilterIpc::default();
    let summary = call_ok(
        "get_usage_periods_summary (IPC)",
        block_on(get_usage_periods_summary(as_state(&state), filter)),
    );
    assert_eq!(summary.periods.today.total_requests, 1);
    assert_eq!(summary.periods.month.total_requests, 1);
    assert_eq!(summary.periods.all_time.total_requests, 1);
    assert!(summary.client_models.contains_key("claude"));
    assert!(summary.limits.is_none());
}

#[test]
fn e2e_import_usage_jsonl_dedup() {
    let state = build_test_state();
    let jsonl = r#"{"agent":"claude-code","model":"gpt-4o","timestamp":1700000000000,"usage":{"input_tokens":100,"output_tokens":50}}
{"agent":"claude-code","model":"gpt-4o","timestamp":1700000000000,"usage":{"input_tokens":100,"output_tokens":50}}
"#;
    let result = call_ok("import_usage", block_on(import_usage(as_state(&state), jsonl.into(), "jsonl".into(), None)));
    assert_eq!(result.imported, 1);
    assert_eq!(result.skipped, 1);
    assert!(result.errors.is_empty());
}

#[test]
fn e2e_get_pricing_returns_entries() {
    let state = build_test_state();
    let entries = call_ok("get_pricing", block_on(get_pricing(as_state(&state))));
    assert!(!entries.is_empty(), "Expected pricing entries");
    let gpt4o = entries.iter().find(|e| e.model == "gpt-4o");
    assert!(gpt4o.is_some(), "Expected gpt-4o in pricing table");
    let entry = gpt4o.unwrap();
    assert!(entry.input_cost_per_token > 0.0);
    assert!(entry.output_cost_per_token > 0.0);
}

// === Action Registry e2e tests (Stage 10) ===

use keypilot_lib::actions::{self, ActionDef};
use keypilot_lib::commands::action::{list_actions, execute_action, ExecuteActionRequest};
use serde_json::json;

#[test]
fn e2e_list_actions_returns_all_registered() {
    let actions: Vec<ActionDef> = list_actions();
    // 6 provider + 3 category + 1 quota + 3 system + 5 token_usage = 18 actions
    assert!(actions.len() >= 18, "Expected >= 18 actions, got {}", actions.len());

    // Verify each action has the required fields
    for a in &actions {
        assert!(!a.id.is_empty());
        assert!(!a.name.is_empty());
        assert!(!a.description.is_empty());
        assert!(!a.category.is_empty());
        assert!(a.input_schema.is_object());
        assert!(a.output_schema.is_object() || a.output_schema.is_array());
    }

    // Spot-check specific actions exist
    let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
    assert!(ids.contains(&"provider.list"));
    assert!(ids.contains(&"provider.add"));
    assert!(ids.contains(&"category.list"));
    assert!(ids.contains(&"quota.fetch"));
    assert!(ids.contains(&"system.get_theme"));
    assert!(ids.contains(&"token_usage.record"));
    assert!(ids.contains(&"token_usage.summary"));
    assert!(ids.contains(&"token_usage.pricing"));
}

#[test]
fn e2e_execute_action_category_list() {
    let state = build_test_state();
    let req = ExecuteActionRequest {
        action_id: "category.list".into(),
        params: Some(json!({})),
    };
    let result = call_ok("execute_action category.list", block_on(execute_action(as_state(&state), req)));
    // Default category seed: 1 category
    let arr = result.as_array().expect("Expected array result");
    assert!(!arr.is_empty(), "Expected at least 1 category");
}

#[test]
fn e2e_execute_action_provider_list() {
    let state = build_test_state();
    let req = ExecuteActionRequest {
        action_id: "provider.list".into(),
        params: Some(json!({})),
    };
    let result = call_ok("execute_action provider.list", block_on(execute_action(as_state(&state), req)));
    let arr = result.as_array().expect("Expected array result");
    assert!(arr.len() >= 5, "Expected >= 5 preset providers, got {}", arr.len());
}

#[test]
fn e2e_execute_action_unknown_returns_error() {
    let state = build_test_state();
    let req = ExecuteActionRequest {
        action_id: "nonexistent.action".into(),
        params: Some(json!({})),
    };
    let result = block_on(execute_action(as_state(&state), req));
    assert!(result.is_err(), "Expected error for unknown action");
    let err = result.unwrap_err();
    let msg = format!("{:?}", err);
    assert!(msg.contains("ActionNotFound"), "Expected ActionNotFound variant, got: {}", msg);
    assert!(msg.contains("nonexistent.action"), "Expected action id in error, got: {}", msg);
}

#[test]
fn e2e_execute_action_missing_required_field_returns_error() {
    let state = build_test_state();
    // provider.get requires "id" field
    let req = ExecuteActionRequest {
        action_id: "provider.get".into(),
        params: Some(json!({})),
    };
    let result = block_on(execute_action(as_state(&state), req));
    assert!(result.is_err(), "Expected error for missing required field");
    let err = result.unwrap_err();
    let json = serde_json::to_value(&err).expect("Failed to serialize error");
    assert_eq!(
        json.get("code").and_then(|v| v.as_str()),
        Some("ACTION_VALIDATION"),
        "Expected ACTION_VALIDATION code, got: {}",
        json
    );
}

#[test]
fn e2e_execute_action_wrong_field_type_returns_error() {
    let state = build_test_state();
    // provider.get expects "id" as integer; pass as string
    let req = ExecuteActionRequest {
        action_id: "provider.get".into(),
        params: Some(json!({ "id": "not-an-int" })),
    };
    let result = block_on(execute_action(as_state(&state), req));
    assert!(result.is_err(), "Expected error for wrong field type");
    let err = result.unwrap_err();
    let json = serde_json::to_value(&err).expect("Failed to serialize error");
    // serde fails to deserialize "not-an-int" as i64 → SERDE code
    let code = json.get("code").and_then(|v| v.as_str());
    assert!(
        code == Some("SERDE") || code == Some("ACTION_VALIDATION"),
        "Expected SERDE or ACTION_VALIDATION code, got: {}",
        json
    );
}

#[test]
fn e2e_execute_action_non_object_params_returns_error() {
    let state = build_test_state();
    // params must be a JSON object (or null), not a string/number/array
    let req = ExecuteActionRequest {
        action_id: "provider.list".into(),
        params: Some(json!("not-an-object")),
    };
    let result = block_on(execute_action(as_state(&state), req));
    assert!(result.is_err(), "Expected error for non-object params");
    let err = result.unwrap_err();
    let json = serde_json::to_value(&err).expect("Failed to serialize error");
    assert_eq!(
        json.get("code").and_then(|v| v.as_str()),
        Some("ACTION_VALIDATION"),
        "Expected ACTION_VALIDATION code, got: {}",
        json
    );
}

#[test]
fn e2e_execute_action_unknown_returns_action_not_found() {
    let state = build_test_state();
    let req = ExecuteActionRequest {
        action_id: "nonexistent.action".into(),
        params: Some(json!({})),
    };
    let result = block_on(execute_action(as_state(&state), req));
    assert!(result.is_err(), "Expected error for unknown action");
    let err = result.unwrap_err();
    // Serialize the error to JSON to verify the code field is set correctly.
    let json = serde_json::to_value(&err).expect("Failed to serialize error");
    assert_eq!(
        json.get("code").and_then(|v| v.as_str()),
        Some("ACTION_NOT_FOUND"),
        "Expected ACTION_NOT_FOUND code, got: {}",
        json
    );
}

#[test]
fn e2e_execute_action_token_usage_pricing() {
    let state = build_test_state();
    let req = ExecuteActionRequest {
        action_id: "token_usage.pricing".into(),
        params: Some(json!({})),
    };
    let result = call_ok("execute_action token_usage.pricing", block_on(execute_action(as_state(&state), req)));
    let arr = result.as_array().expect("Expected array result");
    assert!(!arr.is_empty(), "Expected pricing entries");
}

#[test]
fn e2e_execute_action_token_usage_record() {
    let state = build_test_state();
    let req = ExecuteActionRequest {
        action_id: "token_usage.record".into(),
        params: Some(json!({
            "occurred_at": "2024-01-01T00:00:00Z",
            "provider": "openai",
            "model": "gpt-4o",
            "agent_type": "claude-code",
            "usage_details": {
                "input": 100,
                "output": 50
            }
        })),
    };
    let result = call_ok("execute_action token_usage.record", block_on(execute_action(as_state(&state), req)));
    assert!(result.get("id").is_some(), "Expected id in result");
    assert_eq!(result.get("provider").and_then(|v| v.as_str()), Some("openai"));
    assert_eq!(result.get("model").and_then(|v| v.as_str()), Some("gpt-4o"));
}

#[test]
fn e2e_dispatch_consistency_with_all_actions() {
    // all_actions() from the module and list_actions() IPC should return the same set.
    let from_module = actions::all_actions();
    let from_ipc = list_actions();
    assert_eq!(from_module.len(), from_ipc.len());
    let mut module_ids: Vec<String> = from_module.iter().map(|a| a.id.clone()).collect();
    let mut ipc_ids: Vec<String> = from_ipc.iter().map(|a| a.id.clone()).collect();
    module_ids.sort();
    ipc_ids.sort();
    assert_eq!(module_ids, ipc_ids);
}