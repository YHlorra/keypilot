// src-tauri/tests/ipc_e2e.rs
// Integration tests for Stage A: Token Usage Data Foundation
// Tests Database operations (migrate, insert, query) and PricingService directly.

use keypilot::database::Database;
use keypilot::services::pricing::PricingService;
use keypilot::types::{TokenUsageRecord, TokenCounts, PricingEntry};

// ---------------------------------------------------------------------------
// TokenUsageRecord: DB round-trip + daily rollups
// ---------------------------------------------------------------------------

#[test]
fn test_token_usage_insert_and_query() {
    let db = Database::open_in_memory().expect("Failed to open in-memory DB");
    db.setup_schema().expect("Failed to setup schema");
    db.migrate().expect("Failed to migrate schema");
    db.seed_preset_providers().expect("Failed to seed presets");

    let record = TokenUsageRecord {
        id: "test-1".to_string(),
        agent_type: "coder".to_string(),
        model: "gpt-4o".to_string(),
        provider_name: "OpenAI".to_string(),
        occurred_at: 1700000000,
        recorded_at: 1700000001,
        session_id: Some("sess-1".to_string()),
        request_id: Some("req-1".to_string()),
        input_tokens: 100,
        output_tokens: 50,
        total_tokens: 150,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        reasoning_tokens: 0,
        prompt_cost: 0.0015,
        completion_cost: 0.00025,
        cache_read_cost: 0.0,
        cache_creation_cost: 0.0,
        reasoning_cost: 0.0,
        total_cost: 0.00175,
        currency: "USD".to_string(),
        pricing_version: Some("1".to_string()),
        usage_details: None,
        cost_details: None,
    };
    db.insert_token_usage(&record).expect("Insert failed");

    let results = db.list_token_usage_records(0, 10).expect("Query failed");
    assert_eq!(results.len(), 1, "Expected 1 record");
    assert_eq!(results[0].id, "test-1");
    assert_eq!(results[0].model, "gpt-4o");
    assert_eq!(results[0].input_tokens, 100);
    assert_eq!(results[0].total_cost, 0.00175);

    // daily_agent_model_usage rollup (2023-11-14 = unix 1700000000)
    let daily = db.get_daily_usage_summary("2023-11-14").expect("Daily query failed");
    assert_eq!(daily.len(), 1);
    assert_eq!(daily[0].agent_type, "coder");
    assert_eq!(daily[0].request_count, 1);
    assert_eq!(daily[0].total_tokens, 150);

    // daily_model_usage rollup
    let model_daily = db.get_model_usage_summary("2023-11-14").expect("Model daily query failed");
    assert_eq!(model_daily.len(), 1);
    assert_eq!(model_daily[0].model, "gpt-4o");
    assert_eq!(model_daily[0].request_count, 1);
}

#[test]
fn test_daily_rollups_increment() {
    let db = Database::open_in_memory().expect("Failed to open in-memory DB");
    db.setup_schema().expect("Failed to setup schema");
    db.migrate().expect("Failed to migrate schema");

    let record1 = TokenUsageRecord {
        id: "inc-1".to_string(),
        agent_type: "coder".to_string(),
        model: "gpt-4o".to_string(),
        provider_name: "OpenAI".to_string(),
        occurred_at: 1700000000,
        recorded_at: 1700000001,
        session_id: None,
        request_id: None,
        input_tokens: 100,
        output_tokens: 50,
        total_tokens: 150,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        reasoning_tokens: 0,
        prompt_cost: 0.0015,
        completion_cost: 0.00025,
        cache_read_cost: 0.0,
        cache_creation_cost: 0.0,
        reasoning_cost: 0.0,
        total_cost: 0.00175,
        currency: "USD".to_string(),
        pricing_version: None,
        usage_details: None,
        cost_details: None,
    };
    let record2 = TokenUsageRecord {
        id: "inc-2".to_string(),
        agent_type: "coder".to_string(),
        model: "gpt-4o".to_string(),
        provider_name: "OpenAI".to_string(),
        occurred_at: 1700000060,
        recorded_at: 1700000061,
        session_id: None,
        request_id: None,
        input_tokens: 200,
        output_tokens: 80,
        total_tokens: 280,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        reasoning_tokens: 0,
        prompt_cost: 0.003,
        completion_cost: 0.0004,
        cache_read_cost: 0.0,
        cache_creation_cost: 0.0,
        reasoning_cost: 0.0,
        total_cost: 0.0034,
        currency: "USD".to_string(),
        pricing_version: None,
        usage_details: None,
        cost_details: None,
    };

    db.insert_token_usage(&record1).expect("Insert 1 failed");
    db.insert_token_usage(&record2).expect("Insert 2 failed");

    let daily = db.get_daily_usage_summary("2023-11-14").expect("Daily query failed");
    assert_eq!(daily.len(), 1, "Expected 1 agent-model group");
    assert_eq!(daily[0].request_count, 2);
    assert_eq!(daily[0].input_tokens, 300);
    assert_eq!(daily[0].output_tokens, 130);
    assert_eq!(daily[0].total_tokens, 430);

    let model_daily = db.get_model_usage_summary("2023-11-14").expect("Model daily query failed");
    assert_eq!(model_daily.len(), 1, "Expected 1 model group");
    assert_eq!(model_daily[0].total_tokens, 430);
}

// ---------------------------------------------------------------------------
// PricingService: lookup, version, cost calculation
// ---------------------------------------------------------------------------

#[test]
fn test_pricing_service_lookup_and_cost() {
    let svc = PricingService::new();

    let entry = svc.lookup("gpt-4o").expect("Expected gpt-4o in pricing.json");
    assert_eq!(entry.provider, "OpenAI");
    assert!(entry.input_price_per_1m.is_some());
    assert!(entry.output_price_per_1m.is_some());

    let counts = TokenCounts {
        input: 1_000_000,
        output: 500_000,
        cache_read: 0,
        cache_creation: 0,
        reasoning: 0,
    };
    // gpt-4o: input=$2.50/M, output=$10.00/M
    // cost = 2.50 + (10.00 * 0.5) = 7.50
    let cost = svc.calculate_cost(&entry, &counts);
    assert!((cost - 7.50).abs() < 0.001, "Expected ~7.50, got {}", cost);
}

#[test]
fn test_pricing_service_unknown_model() {
    let svc = PricingService::new();
    assert!(svc.lookup("nonexistent-model-xyz").is_none());
}

#[test]
fn test_pricing_service_version() {
    let svc = PricingService::new();
    let ver = svc.version();
    assert!(!ver.is_empty(), "Version should not be empty");
}

#[test]
fn test_pricing_service_partial_rates() {
    let svc = PricingService::new();
    // Model with only input_price_per_1m should only charge input
    let entry = PricingEntry {
        model: "test-only-input".to_string(),
        provider: "Test".to_string(),
        input_price_per_1m: Some(5.0),
        output_price_per_1m: None,
        cache_read_price_per_1m: None,
        cache_creation_price_per_1m: None,
        reasoning_price_per_1m: None,
    };
    let counts = TokenCounts { input: 1_000_000, output: 500_000, cache_read: 0, cache_creation: 0, reasoning: 0 };
    let cost = svc.calculate_cost(&entry, &counts);
    assert_eq!(cost, 5.0, "Only input should be charged");
}
