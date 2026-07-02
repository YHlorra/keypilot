// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Jason Young (cc-switch original author). Adapted for keypilot.
// See ../../../../docs/third-party/cc-switch.LICENSE for the verbatim license copy.

//! ZenMux coding plan provider.
//!
//! Endpoint: ZenMux exposes its quota surface at the same host the user
//! configured as their `base_url` (the request goes to `base_url` directly,
//! not to a separate quota host). The dispatcher in [`super::detect_provider`]
//! routes any host substring matching `zenmux` (lowercased) to this module,
//! so users on custom proxies / region-specific mirrors can still hit it.
//!
//! Auth: `Authorization: Bearer <api_key>`.
//!
//! Response (per cc-switch `query_zenmux` reference, MIT upstream):
//! ```json
//! {
//!   "success": true,
//!   "message": "ok",
//!   "data": {
//!     "quota_5_hour": {
//!       "usage_percentage": 0.25,
//!       "resets_at": "2026-07-02T12:00:00Z",
//!       "used_value_usd": 1.25,
//!       "max_value_usd": 5.0
//!     },
//!     "quota_7_day": {
//!       "usage_percentage": 0.5,
//!       "resets_at": "2026-07-09T00:00:00Z",
//!       "used_value_usd": 5.0,
//!       "max_value_usd": 10.0
//!     },
//!     "plan": { "tier": "Pro" },
//!     "account_status": "active"
//!   }
//! }
//! ```
//!
//! Field semantics (verified from cc-switch reference):
//! - `usage_percentage` is a 0.0–1.0 **fraction** (NOT 0–100), multiplied by
//!   100 here to land in the standard 0–100 percent scale.
//! - `used_value_usd` / `max_value_usd` are absolute USD amounts — ZenMux is
//!   one of the few coding-plan providers that reports absolute values, so
//!   we populate `used` / `limit` directly.
//! - `resets_at` is ISO 8601 string; converted to epoch ms via
//!   [`extract_reset_ms`] so downstream code never has to branch on
//!   string-vs-int.
//! - `plan.tier` + `account_status` are surfaced via `credential_message`
//!   as `"<tier> (<account_status>)"` so the UI can show the active plan.
//!
//! `quota_5_hour` maps to [`QuotaTierKind::FiveHour`]; `quota_7_day` maps to
//! [`QuotaTierKind::Weekly`] (the upstream has no monthly bucket). Both are
//! emitted independently — ZenMux plans carry both, unlike MiniMax where
//! weekly is optional.
//!
//! Design pattern adapted from cc-switch (MIT, Copyright 2025 Jason Young).
//! See [`../../../../docs/third-party/cc-switch.LICENSE`](../../../../docs/third-party/cc-switch.LICENSE).

use crate::provider::coding_plan::subscription::{extract_reset_ms, make_error, make_success, parse_f64};
use crate::types::subscription::{CredentialStatus, QuotaTier, QuotaTierKind, SubscriptionQuota, TierStatus};

const HTTP_TIMEOUT_SECS: u64 = 15;

/// ZenMux provider id surfaced to the UI / logs.
const PROVIDER_ID: &str = "zenmux";

/// Top-level entry point. Mirrors the cc-switch `query_zenmux` signature:
/// `(base_url, api_key)` — `base_url` is the actual quota endpoint host
/// (no separate quota URL like MiniMax has).
pub async fn fetch(base_url: &str, api_key: &str) -> SubscriptionQuota {
    let client = reqwest::Client::new();

    let resp = match client
        .get(base_url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return make_error(PROVIDER_ID, format!("Network: {e}"), CredentialStatus::Valid),
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return make_error(
            PROVIDER_ID,
            format!("Auth failed: HTTP {status}"),
            CredentialStatus::Invalid,
        );
    }
    if !status.is_success() {
        return make_error(PROVIDER_ID, format!("HTTP {status}"), CredentialStatus::Valid);
    }

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => return make_error(PROVIDER_ID, format!("Parse: {e}"), CredentialStatus::Valid),
    };

    // Business-level error envelope: `success` field, not a 4xx HTTP code.
    // Anything other than `true` is treated as an upstream failure.
    if body.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let msg = body
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return make_error(PROVIDER_ID, format!("API error: {msg}"), CredentialStatus::Valid);
    }

    let Some(data) = body.get("data") else {
        return make_error(
            PROVIDER_ID,
            "Missing 'data' field in response".to_string(),
            CredentialStatus::Valid,
        );
    };

    let tiers = parse_tiers(data);

    // Plan tier + account status → credential_message, mirroring cc-switch's
    // `"{tier} ({account_status})"` format so the UI can render it directly.
    let plan_tier = data
        .get("plan")
        .and_then(|p| p.get("tier"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let account_status = data
        .get("account_status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let credential_message = if plan_tier.is_empty() {
        None
    } else if account_status.is_empty() {
        Some(plan_tier.to_string())
    } else {
        Some(format!("{plan_tier} ({account_status})"))
    };

    let mut quota = make_success(PROVIDER_ID, tiers);
    quota.credential_message = credential_message;
    quota
}

/// Pure parser for the ZenMux `data` object. Extracted so unit tests can
/// cover the JSON-shape contract without a mock HTTP server.
///
/// `data` is the value of the top-level `data` key in the ZenMux response.
///
/// Behavior:
/// - Emits a 5-hour tier iff `quota_5_hour` is present and parseable.
/// - Emits a weekly tier iff `quota_7_day` is present and parseable.
/// - Missing fields / null / unparseable numerics fall back to defaults
///   rather than emitting a half-populated tier that could mislead the UI.
/// - Percentage is reported by upstream as a 0.0–1.0 fraction; we flip it
///   to `remaining_percent = (1 - fraction) * 100` so [`crate::provider::coding_plan::subscription::make_tier`]
///   can derive `used_percent` via its standard `100 - remaining` rule.
pub(crate) fn parse_tiers(data: &serde_json::Value) -> Vec<QuotaTier> {
    let mut tiers = Vec::new();

    if let Some(q5h) = data.get("quota_5_hour") {
        if let Some(tier) = parse_window(q5h, QuotaTierKind::FiveHour, "5h") {
            tiers.push(tier);
        }
    }

    if let Some(q7d) = data.get("quota_7_day") {
        if let Some(tier) = parse_window(q7d, QuotaTierKind::Weekly, "7d") {
            tiers.push(tier);
        }
    }

    tiers
}

/// Parse one ZenMux window (`quota_5_hour` or `quota_7_day`) into a
/// [`QuotaTier`]. Returns `None` only if the upstream object is the wrong
/// shape (not an object); per-field fallbacks inside are non-fatal so a
/// partially-populated response still renders.
///
/// `kind` discriminates the bucket; `label_prefix` is just a tag for the
/// `label` field (UI-facing, "5h" / "7d"). USD `used` / `limit` are
/// populated directly — ZenMux is one of the few providers reporting
/// absolute values, so we don't drop them through `make_tier` (which would
/// force them to None).
fn parse_window(window: &serde_json::Value, kind: QuotaTierKind, label_prefix: &str) -> Option<QuotaTier> {
    let obj = window.as_object()?;

    // usage_percentage is a 0-1 fraction in upstream → flip to remaining 0-100.
    let used_fraction = obj.get("usage_percentage").and_then(parse_f64).unwrap_or(0.0);
    let used_percent = (used_fraction * 100.0).clamp(0.0, 100.0);
    let remaining_percent = (100.0 - used_percent).clamp(0.0, 100.0);

    let resets_at_ms = obj.get("resets_at").and_then(extract_reset_ms);

    let used = obj.get("used_value_usd").and_then(parse_f64);
    let limit = obj.get("max_value_usd").and_then(parse_f64);

    Some(QuotaTier {
        kind,
        label: format!("ZenMux {label_prefix}"),
        used,
        limit,
        used_percent: Some(used_percent),
        remaining_percent: Some(remaining_percent),
        resets_at_ms,
        reset_description: String::new(),
        // ZenMux doesn't expose an activation flag — if the window object
        // is present at all, treat it as Active. Inactive plans simply
        // omit the field.
        status: TierStatus::Active,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tiers_both_windows_with_usd_and_iso_reset() {
        // Canonical ZenMux response: both windows present, ISO reset,
        // USD amounts populated, plan tier + account_status surfaced.
        let data = serde_json::json!({
            "quota_5_hour": {
                "usage_percentage": 0.25,
                "resets_at": "2026-07-02T12:00:00Z",
                "used_value_usd": 1.25,
                "max_value_usd": 5.0
            },
            "quota_7_day": {
                "usage_percentage": 0.5,
                "resets_at": "2026-07-09T00:00:00Z",
                "used_value_usd": 5.0,
                "max_value_usd": 10.0
            },
            "plan": { "tier": "Pro" },
            "account_status": "active"
        });

        let tiers = parse_tiers(&data);
        assert_eq!(tiers.len(), 2);

        // 5-hour: 25% used → 75% remaining
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].label, "ZenMux 5h");
        assert_eq!(tiers[0].used_percent, Some(25.0));
        assert_eq!(tiers[0].remaining_percent, Some(75.0));
        assert_eq!(tiers[0].used, Some(1.25));
        assert_eq!(tiers[0].limit, Some(5.0));
        assert_eq!(tiers[0].status, TierStatus::Active);
        assert_eq!(
            tiers[0].resets_at_ms,
            Some(1_782_993_600_000), // 2026-07-02T12:00:00Z epoch ms
        );

        // 7-day: 50% used → 50% remaining
        assert_eq!(tiers[1].kind, QuotaTierKind::Weekly);
        assert_eq!(tiers[1].label, "ZenMux 7d");
        assert_eq!(tiers[1].used_percent, Some(50.0));
        assert_eq!(tiers[1].remaining_percent, Some(50.0));
        assert_eq!(tiers[1].used, Some(5.0));
        assert_eq!(tiers[1].limit, Some(10.0));
    }

    #[test]
    fn parse_tiers_handles_missing_7_day_window() {
        // ZenMux always sends 5h; 7d is optional per cc-switch. Missing
        // window must not crash — emit only the 5h tier.
        let data = serde_json::json!({
            "quota_5_hour": {
                "usage_percentage": 0.1,
                "resets_at": "2026-07-02T12:00:00Z"
            }
        });

        let tiers = parse_tiers(&data);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].used_percent, Some(10.0));
        assert_eq!(tiers[0].remaining_percent, Some(90.0));
        // No USD fields supplied → None, not zero.
        assert_eq!(tiers[0].used, None);
        assert_eq!(tiers[0].limit, None);
    }
}