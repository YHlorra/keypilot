use super::*;
use crate::provider::coding_plan::minimax::parse_minimax_tiers;
use crate::provider::coding_plan::subscription::{
    extract_reset_ms, make_error, make_success, make_tier, parse_f64,
};
use crate::types::subscription::{CredentialStatus, QuotaTierKind, TierStatus};

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
    assert_eq!(parse_f64(&serde_json::json!("")), None);
}

#[test]
fn extract_reset_ms_handles_ms() {
    let v = serde_json::json!(1_782_993_600_000_i64);
    assert_eq!(extract_reset_ms(&v), Some(1_782_993_600_000));
}

#[test]
fn extract_reset_ms_handles_seconds() {
    let v = serde_json::json!(1_782_993_600_i64);
    assert_eq!(extract_reset_ms(&v), Some(1_782_993_600_000));
}

#[test]
fn extract_reset_ms_handles_iso8601_string() {
    let v = serde_json::json!("2026-07-01T00:00:00Z");
    let out = extract_reset_ms(&v).expect("ISO 8601 must parse");
    assert_eq!(out, 1_782_864_000_000);
}

#[test]
fn extract_reset_ms_handles_negative() {
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

#[test]
fn make_error_default_status_is_valid() {
    let err = make_error("test", "boom".into(), CredentialStatus::Valid);
    assert!(!err.success);
    assert_eq!(err.error, Some("boom".into()));
    assert_eq!(err.credential_status, CredentialStatus::Valid);
    assert_eq!(err.provider_id, "test");
    assert!(err.tiers.is_empty());
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

#[test]
fn minimax_parses_real_response_with_active_weekly() {
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

    assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
    assert_eq!(tiers[0].label, "General 5h");
    assert_eq!(tiers[0].remaining_percent, Some(85.0));
    assert_eq!(tiers[0].used_percent, Some(15.0));
    assert_eq!(tiers[0].resets_at_ms, Some(1_783_008_000_000));
    assert_eq!(tiers[0].status, TierStatus::Active);

    assert_eq!(tiers[1].kind, QuotaTierKind::Weekly);
    assert_eq!(tiers[1].label, "General Weekly");
    assert_eq!(tiers[1].remaining_percent, Some(75.0));
    assert_eq!(tiers[1].used_percent, Some(25.0));
    assert_eq!(tiers[1].status, TierStatus::Active);
}

#[test]
fn minimax_skips_weekly_when_status_is_3() {
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
