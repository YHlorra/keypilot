//! MiniMax coding plan provider.
//!
//! Endpoint: `https://www.minimaxi.com/v1/token_plan/remains`
//! (Note: token-plan endpoint uses `www.minimaxi.com`, not the data-plane
//! `api.minimaxi.com` / `api.minimax.io` — keypilot's `base_url` is the
//! data plane; we hard-code the token-plan host because MiniMax's quota
//! surface is single-region.)
//!
//! Auth: `Authorization: Bearer <api_key>`.
//!
//! Response (verified against real API key 2026-06):
//! ```json
//! {
//!   "model_remains": [{
//!     "model_name": "general",
//!     "start_time": 1782993600000,
//!     "end_time": 1783008000000,
//!     "remains_time": 11887961,
//!     "current_interval_total_count": 0,
//!     "current_interval_usage_count": 0,
//!     "current_interval_status": 1,
//!     "current_interval_remaining_percent": 85,
//!     "weekly_start_time": 1782662400000,
//!     "weekly_end_time": 1783267200000,
//!     "weekly_remains_time": 271087961,
//!     "current_weekly_total_count": 0,
//!     "current_weekly_usage_count": 0,
//!     "current_weekly_status": 3,
//!     "current_weekly_remaining_percent": 100
//!   }],
//!   "base_resp": { "status_code": 0, "status_msg": "success" }
//! }
//! ```
//!
//! Per `model_remains[]` we emit:
//! - 5-hour tier (always; `current_interval_status == 1` ⇒ Active)
//! - Weekly tier ONLY when `current_weekly_status == 1`; plans without
//!   weekly cap return `status == 3` and a fake `remaining_percent == 100`,
//!   which would otherwise display a misleading "100% remaining" gauge.
//!
//! Design pattern adapted from cc-switch (MIT, Copyright 2025 Jason Young).
//! See [`../../../../docs/third-party/cc-switch.LICENSE`](../../../../docs/third-party/cc-switch.LICENSE).

use crate::provider::coding_plan::subscription::{make_error, make_success, make_tier, parse_f64};
use crate::types::subscription::{CredentialStatus, QuotaTierKind, SubscriptionQuota, TierStatus};

/// MiniMax quota endpoint. Same host for both regions — auth / API key is
/// what distinguishes CN vs overseas accounts. Keep verbatim per MiniMax docs.
const TOKEN_PLAN_URL: &str = "https://www.minimaxi.com/v1/token_plan/remains";

const HTTP_TIMEOUT_SECS: u64 = 15;

/// Top-level entry point. Mirrors the cc-switch `query_minimax` signature:
/// `(base_url, api_key)` — `base_url` is only used to disambiguate region
/// for diagnostics; the quota host is fixed.
pub async fn fetch(base_url: &str, api_key: &str) -> SubscriptionQuota {
    // We accept `base_url` so the dispatcher can route CN vs overseas
    // without a different signature, but the actual quota endpoint is the
    // same. Surfacing the region in the provider_id helps log diagnosis.
    let provider_id = if base_url.contains("api.minimax.io") {
        "minimax_en"
    } else {
        "minimax_cn"
    };

    let client = reqwest::Client::new();
    let resp = client
        .get(TOKEN_PLAN_URL)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => return make_error(provider_id, format!("Network: {e}"), CredentialStatus::Valid),
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return make_error(
            provider_id,
            format!("Auth failed: HTTP {status}"),
            CredentialStatus::Invalid,
        );
    }
    if !status.is_success() {
        return make_error(provider_id, format!("HTTP {status}"), CredentialStatus::Valid);
    }

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => return make_error(provider_id, format!("Parse: {e}"), CredentialStatus::Valid),
    };

    // Business-level error: base_resp.status_code != 0
    if let Some(code) = body.get("base_resp").and_then(|b| b.get("status_code")).and_then(|c| c.as_i64()) {
        if code != 0 {
            let msg = body
                .get("base_resp")
                .and_then(|b| b.get("status_msg"))
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_string();
            return make_error(
                provider_id,
                format!("base_resp error {code}: {msg}"),
                CredentialStatus::Valid,
            );
        }
    }

    let tiers = parse_minimax_tiers(&body);
    make_success(provider_id, tiers)
}

/// Pure parser extracted so unit tests can hit it without a mock server.
///
/// Returns 1 tier (5-hour) when the response has no `model_remains` array,
/// or the `general` model entry, or the weekly status is not Active.
/// Returns 2 tiers when `general` + weekly-active is present.
///
/// `body` is the raw JSON value from the upstream endpoint.
pub(crate) fn parse_minimax_tiers(body: &serde_json::Value) -> Vec<crate::types::subscription::QuotaTier> {
    let mut tiers = Vec::new();

    let arr = match body.get("model_remains").and_then(|m| m.as_array()) {
        Some(a) => a,
        None => return tiers,
    };

    // ponytail: only the `general` model_name is the coding-plan tier.
    // Other names (e.g. `video`) refer to non-coding subscriptions and
    // would mislead the user if surfaced here. Mirrors cc-switch's filter.
    let Some(entry) = arr.iter().find(|item| {
        item.get("model_name")
            .and_then(|v| v.as_str())
            .map(|s| s == "general")
            .unwrap_or(false)
    }) else {
        return tiers;
    };

    // 5-hour bucket — always present if the response was successful.
    let interval_pct = entry
        .get("current_interval_remaining_percent")
        .and_then(parse_f64);
    let interval_status = entry
        .get("current_interval_status")
        .and_then(|v| v.as_i64())
        .map(|n| match n {
            1 => TierStatus::Active,
            _ => TierStatus::Inactive,
        })
        .unwrap_or(TierStatus::Unknown);
    let interval_end_ms = entry.get("end_time").and_then(|v| v.as_i64());

    tiers.push(make_tier(
        QuotaTierKind::FiveHour,
        "General 5h",
        interval_pct,
        interval_end_ms,
        interval_status,
    ));

    // Weekly bucket — only when status == 1 (active). Plans without a
    // weekly cap return status == 3 with a misleading 100% reading.
    let weekly_status_raw = entry.get("current_weekly_status").and_then(|v| v.as_i64());
    if weekly_status_raw == Some(1) {
        let weekly_pct = entry
            .get("current_weekly_remaining_percent")
            .and_then(parse_f64);
        let weekly_status = TierStatus::Active;
        let weekly_end_ms = entry.get("weekly_end_time").and_then(|v| v.as_i64());
        tiers.push(make_tier(
            QuotaTierKind::Weekly,
            "General Weekly",
            weekly_pct,
            weekly_end_ms,
            weekly_status,
        ));
    }

    tiers
}
