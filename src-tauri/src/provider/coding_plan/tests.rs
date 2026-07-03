//! Unit tests for the coding-plan quota framework.
//!
//! Network-level fetch() is intentionally not exercised here — the
//! integration test belongs to Lane B with a wiremock / mock server.

use super::*;
use crate::provider::coding_plan::minimax::parse_minimax_tiers;
use crate::provider::coding_plan::subscription::{
    extract_reset_ms, make_error, make_success, make_tier, parse_f64,
};
use crate::types::subscription::{CredentialStatus, QuotaTierKind, TierStatus};

// ── detect_provider ─────────────────────────────────────────

#[test]
fn detect_provider_minimax_cn() {
    assert_eq!(
        detect_provider("https://api.minimaxi.com/v1"),
        Some(CodingPlanProvider::MiniMaxCn)
    );
}

#[test]
fn detect_provider_minimax_en() {
    assert_eq!(
        detect_provider("https://api.minimax.io/v1"),
        Some(CodingPlanProvider::MiniMaxEn)
    );
}

#[test]
fn detect_provider_minimax_case_insensitive() {
    // base_url might be uppercased by user / env. detect_provider must lowercase.
    assert_eq!(
        detect_provider("HTTPS://API.MINIMAXI.COM/V1"),
        Some(CodingPlanProvider::MiniMaxCn)
    );
}

#[test]
fn detect_provider_kimi() {
    assert_eq!(
        detect_provider("https://api.kimi.com/coding/v1"),
        Some(CodingPlanProvider::Kimi)
    );
}

#[test]
fn detect_provider_kimi_preset_default() {
    assert_eq!(
        detect_provider("https://api.moonshot.cn/v1"),
        Some(CodingPlanProvider::Kimi)
    );
}

#[test]
fn detect_provider_kimi_anthropic_preset_default() {
    assert_eq!(
        detect_provider("https://api.moonshot.cn/anthropic"),
        Some(CodingPlanProvider::Kimi)
    );
}

#[test]
fn detect_provider_zhipu_cn() {
    assert_eq!(
        detect_provider("https://open.bigmodel.cn/api/paas/v4"),
        Some(CodingPlanProvider::ZhipuCn)
    );
}

#[test]
fn detect_provider_zhipu_en() {
    assert_eq!(
        detect_provider("https://api.z.ai/api/paas/v4"),
        Some(CodingPlanProvider::ZhipuEn)
    );
}

#[test]
fn detect_provider_volcengine() {
    assert_eq!(
        detect_provider("https://ark.cn-beijing.volces.com/api/coding/v3"),
        Some(CodingPlanProvider::Volcengine)
    );
}

#[test]
fn detect_provider_volcengine_preset_default() {
    assert_eq!(
        detect_provider("https://ark.cn-beijing.volces.com/api/v3"),
        Some(CodingPlanProvider::Volcengine)
    );
}

#[test]
fn detect_provider_volcengine_anthropic_preset_default() {
    assert_eq!(
        detect_provider("https://ark.cn-beijing.volces.com/api/coding"),
        Some(CodingPlanProvider::Volcengine)
    );
}

#[test]
fn detect_provider_zhipu_anthropic_preset_default() {
    assert_eq!(
        detect_provider("https://open.bigmodel.cn/api/anthropic"),
        Some(CodingPlanProvider::ZhipuCn)
    );
}

#[test]
fn detect_provider_zenmux() {
    assert_eq!(
        detect_provider("https://zenmux.example.com/api/v1"),
        Some(CodingPlanProvider::ZenMux)
    );
}

#[test]
fn detect_provider_unknown() {
    assert_eq!(detect_provider("https://example.com"), None);
    assert_eq!(detect_provider(""), None);
    assert_eq!(detect_provider("not-a-url"), None);
}

#[test]
fn detect_provider_order_minimax_cn_beats_io_substring() {
    // Regression guard: `api.minimax.io` contains the substring `minimax`,
    // but the dispatch order must keep CN and EN distinct. Adding a future
    // generic `minimax` match above would silently break this — the test
    // catches it.
    assert_eq!(
        detect_provider("https://api.minimaxi.com/v1"),
        Some(CodingPlanProvider::MiniMaxCn),
        "CN must not collapse into EN"
    );
    assert_eq!(
        detect_provider("https://api.minimax.io/v1"),
        Some(CodingPlanProvider::MiniMaxEn),
        "EN must not collapse into CN"
    );
}

// ── helpers::parse_f64 ──────────────────────────────────────

#[test]
fn parse_f64_accepts_number_and_string() {
    assert_eq!(parse_f64(&serde_json::json!(85.5)), Some(85.5));
    assert_eq!(parse_f64(&serde_json::json!("85.5")), Some(85.5));
    assert_eq!(parse_f64(&serde_json::json!("0")), Some(0.0));
    assert_eq!(parse_f64(&serde_json::json!("-1")), Some(-1.0));
    assert_eq!(parse_f64(&serde_json::json!("abc")), None);
    assert_eq!(parse_f64(&serde_json::json!(null)), None);
    assert_eq!(parse_f64(&serde_json::json!(true)), None);
    assert_eq!(parse_f64(&serde_json::json!([])), None);
}

#[test]
fn parse_f64_handles_integer() {
    assert_eq!(parse_f64(&serde_json::json!(85)), Some(85.0));
    assert_eq!(parse_f64(&serde_json::json!(-1)), Some(-1.0));
}

#[test]
fn parse_f64_empty_string_returns_none() {
    // ponytail: empty-string → parse::<f64>() fails → None.
    // No explicit branch needed; relying on Option::None propagation.
    assert_eq!(parse_f64(&serde_json::json!("")), None);
}

// ── helpers::extract_reset_ms ────────────────────────────────

#[test]
fn extract_reset_ms_handles_ms() {
    // 1.782e12 is past 2001 in ms — clearly ms not seconds.
    let v = serde_json::json!(1_782_993_600_000_i64);
    assert_eq!(extract_reset_ms(&v), Some(1_782_993_600_000));
}

#[test]
fn extract_reset_ms_handles_seconds() {
    // 1.782e9 is ~year 2026 in seconds — should multiply by 1000.
    let v = serde_json::json!(1_782_993_600_i64);
    assert_eq!(extract_reset_ms(&v), Some(1_782_993_600_000));
}

#[test]
fn extract_reset_ms_handles_iso8601_string() {
    let v = serde_json::json!("2026-07-01T00:00:00Z");
    let out = extract_reset_ms(&v).expect("ISO 8601 must parse");
    // 2026-07-01T00:00:00Z = 1782864000 seconds = 1782864000000 ms
    // (epoch calc: 2026-01-01 = 1767225600s + 181 days × 86400s).
    assert_eq!(out, 1_782_864_000_000);
}

#[test]
fn extract_reset_ms_handles_negative() {
    // Volcengine session-no-active-window sentinel; matches cc-switch behavior.
    assert_eq!(extract_reset_ms(&serde_json::json!(-1_i64)), None);
    assert_eq!(extract_reset_ms(&serde_json::json!(0_i64)), None);
}

#[test]
fn extract_reset_ms_handles_unparseable_string() {
    assert_eq!(extract_reset_ms(&serde_json::json!("not-a-date")), None);
}

#[test]
fn extract_reset_ms_handles_null_and_bool() {
    assert_eq!(extract_reset_ms(&serde_json::json!(null)), None);
    assert_eq!(extract_reset_ms(&serde_json::json!(true)), None);
}

// ── helpers::make_tier ──────────────────────────────────────

#[test]
fn make_tier_clamps_remaining_percent() {
    let tier = make_tier(
        QuotaTierKind::FiveHour,
        "test",
        Some(150.0),
        None,
        TierStatus::Active,
    );
    assert_eq!(tier.remaining_percent, Some(100.0));
    assert_eq!(tier.used_percent, Some(0.0));
}

#[test]
fn make_tier_clamps_negative_remaining_percent() {
    let tier = make_tier(
        QuotaTierKind::Weekly,
        "test",
        Some(-10.0),
        None,
        TierStatus::Active,
    );
    assert_eq!(tier.remaining_percent, Some(0.0));
    assert_eq!(tier.used_percent, Some(100.0));
}

#[test]
fn make_tier_none_remaining_keeps_both_none() {
    let tier = make_tier(
        QuotaTierKind::Monthly,
        "test",
        None,
        None,
        TierStatus::Unknown,
    );
    assert_eq!(tier.remaining_percent, None);
    assert_eq!(tier.used_percent, None);
}

#[test]
fn make_tier_used_equals_hundred_minus_remaining() {
    let tier = make_tier(
        QuotaTierKind::FiveHour,
        "test",
        Some(85.0),
        None,
        TierStatus::Active,
    );
    assert_eq!(tier.remaining_percent, Some(85.0));
    assert_eq!(tier.used_percent, Some(15.0));
}

// ── helpers::make_error / make_success ──────────────────────

#[test]
fn make_error_default_status_is_valid() {
    let err = make_error("test", "boom".into(), CredentialStatus::Valid);
    assert!(!err.success);
    assert_eq!(err.error, Some("boom".into()));
    assert_eq!(err.credential_status, CredentialStatus::Valid);
    assert_eq!(err.provider_id, "test");
    assert!(err.tiers.is_empty());
    // queried_at_ms must be > 0 (any reasonable epoch).
    assert!(err.queried_at_ms > 0);
}

#[test]
fn make_error_propagates_credential_status() {
    let err = make_error("test", "boom".into(), CredentialStatus::Invalid);
    assert_eq!(err.credential_status, CredentialStatus::Invalid);
}

#[test]
fn make_success_default_status_is_valid() {
    let ok = make_success("minimax", vec![]);
    assert!(ok.success);
    assert_eq!(ok.error, None);
    assert_eq!(ok.credential_status, CredentialStatus::Valid);
    assert_eq!(ok.provider_id, "minimax");
}

// ── minimax::parse_minimax_tiers ────────────────────────────

#[test]
fn minimax_parses_real_response_with_active_weekly() {
    // Real response shape with active weekly (status == 1) → 2 tiers.
    let body = serde_json::json!({
        "model_remains": [{
            "model_name": "general",
            "start_time": 1_782_993_600_000_i64,
            "end_time": 1_783_008_000_000_i64,
            "remains_time": 11_887_961_i64,
            "current_interval_total_count": 0,
            "current_interval_usage_count": 0,
            "current_interval_status": 1,
            "current_interval_remaining_percent": 85,
            "weekly_start_time": 1_782_662_400_000_i64,
            "weekly_end_time": 1_783_267_200_000_i64,
            "weekly_remains_time": 271_087_961_i64,
            "current_weekly_total_count": 0,
            "current_weekly_usage_count": 0,
            "current_weekly_status": 1,
            "current_weekly_remaining_percent": 75
        }],
        "base_resp": { "status_code": 0, "status_msg": "success" }
    });
    let tiers = parse_minimax_tiers(&body);
    assert_eq!(tiers.len(), 2);

    // 5-hour tier
    assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
    assert_eq!(tiers[0].label, "General 5h");
    assert_eq!(tiers[0].remaining_percent, Some(85.0));
    assert_eq!(tiers[0].used_percent, Some(15.0));
    assert_eq!(tiers[0].resets_at_ms, Some(1_783_008_000_000));
    assert_eq!(tiers[0].status, TierStatus::Active);

    // Weekly tier (status == 1 ⇒ Active)
    assert_eq!(tiers[1].kind, QuotaTierKind::Weekly);
    assert_eq!(tiers[1].label, "General Weekly");
    assert_eq!(tiers[1].remaining_percent, Some(75.0));
    assert_eq!(tiers[1].used_percent, Some(25.0));
    assert_eq!(tiers[1].status, TierStatus::Active);
}

#[test]
fn minimax_skips_weekly_when_status_is_3() {
    // Real response with weekly status == 3 (no weekly cap on this plan).
    // Should emit ONLY the 5-hour tier, ignoring the misleading 100%.
    let body = serde_json::json!({
        "model_remains": [{
            "model_name": "general",
            "current_interval_status": 1,
            "current_interval_remaining_percent": 85,
            "current_weekly_status": 3,
            "current_weekly_remaining_percent": 100
        }],
        "base_resp": { "status_code": 0, "status_msg": "success" }
    });
    let tiers = parse_minimax_tiers(&body);
    assert_eq!(tiers.len(), 1, "Weekly tier must be skipped when status != 1");
    assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
}

#[test]
fn minimax_skips_non_general_model_entries() {
    // Non-general model entries (e.g. `video`) must be filtered out.
    let body = serde_json::json!({
        "model_remains": [
            {
                "model_name": "video",
                "current_interval_status": 1,
                "current_interval_remaining_percent": 50
            },
            {
                "model_name": "general",
                "current_interval_status": 1,
                "current_interval_remaining_percent": 85,
                "current_weekly_status": 3,
                "current_weekly_remaining_percent": 100
            }
        ],
        "base_resp": { "status_code": 0, "status_msg": "success" }
    });
    let tiers = parse_minimax_tiers(&body);
    assert_eq!(tiers.len(), 1, "Only `general` model is coding-plan relevant");
    assert_eq!(tiers[0].remaining_percent, Some(85.0));
}

#[test]
fn minimax_handles_missing_model_remains() {
    let body = serde_json::json!({
        "base_resp": { "status_code": 0, "status_msg": "success" }
    });
    assert!(parse_minimax_tiers(&body).is_empty());
}

#[test]
fn minimax_handles_missing_general_entry() {
    // Only `video` entries; no `general` ⇒ empty result.
    let body = serde_json::json!({
        "model_remains": [{
            "model_name": "video",
            "current_interval_remaining_percent": 50
        }],
        "base_resp": { "status_code": 0, "status_msg": "success" }
    });
    assert!(parse_minimax_tiers(&body).is_empty());
}

#[test]
fn minimax_string_percent_fallback() {
    // Defensive: some MiniMax proxies serialize percentage as string.
    let body = serde_json::json!({
        "model_remains": [{
            "model_name": "general",
            "current_interval_status": 1,
            "current_interval_remaining_percent": "85",
            "current_weekly_status": 1,
            "current_weekly_remaining_percent": "75"
        }],
        "base_resp": { "status_code": 0, "status_msg": "success" }
    });
    let tiers = parse_minimax_tiers(&body);
    assert_eq!(tiers.len(), 2);
    assert_eq!(tiers[0].remaining_percent, Some(85.0));
    assert_eq!(tiers[1].remaining_percent, Some(75.0));
}

#[test]
fn minimax_inactive_5h_status_maps_to_inactive() {
    // 5-hour status == 2 (or anything else) ⇒ Inactive, but still emitted
    // so the UI can explain why the gauge is empty / hidden.
    let body = serde_json::json!({
        "model_remains": [{
            "model_name": "general",
            "current_interval_status": 2,
            "current_interval_remaining_percent": 0,
            "current_weekly_status": 1,
            "current_weekly_remaining_percent": 100
        }],
        "base_resp": { "status_code": 0, "status_msg": "success" }
    });
    let tiers = parse_minimax_tiers(&body);
    assert_eq!(tiers.len(), 2);
    assert_eq!(tiers[0].status, TierStatus::Inactive);
    assert_eq!(tiers[1].status, TierStatus::Active);
}

// ── fetch_coding_plan_quota (dispatcher) ────────────────────

#[tokio::test]
async fn dispatcher_rejects_empty_api_key() {
    let q = fetch_coding_plan_quota("https://api.minimaxi.com/v1", "").await;
    assert!(!q.success);
    assert_eq!(q.credential_status, CredentialStatus::Invalid);
    assert!(q.error.is_some());
}

#[tokio::test]
async fn dispatcher_returns_unknown_provider_for_unrecognized_host() {
    let q = fetch_coding_plan_quota("https://example.com", "sk-test").await;
    assert!(!q.success);
    assert_eq!(q.credential_status, CredentialStatus::Unknown);
    assert_eq!(q.provider_id, "unknown");
    assert!(q.tiers.is_empty());
}

#[tokio::test]
async fn dispatcher_dispatches_kimi_to_real_provider_not_stub() {
    // Lane C: Kimi is now wired with a real implementation. The dispatcher
    // must NOT emit a "not yet implemented" placeholder — instead it routes
    // to `kimi::fetch`, which then attempts the real HTTP call. The wire
    // outcome depends on the test environment: a sealed network returns a
    // transport error (Valid status, success=false) while a real endpoint
    // returns HTTP 401 for our fake key (Invalid status, success=false).
    // Either is fine — the only thing we assert is that the new arm fired.
    let q = fetch_coding_plan_quota("https://api.kimi.com/coding/v1", "sk-test").await;
    assert_eq!(
        q.provider_id, "kimi",
        "Lane C: Kimi must dispatch to kimi::fetch, not return a stub"
    );
    let err = q.error.as_deref().unwrap_or("");
    assert!(
        !err.contains("not yet implemented"),
        "stub placeholder must be gone after Lane C wiring (got: {err:?})"
    );
    assert!(!q.success, "no successful Kimi fetch without a real key");
}
