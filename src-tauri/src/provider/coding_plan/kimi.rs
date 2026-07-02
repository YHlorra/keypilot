//! Kimi For Coding provider.
//!
//! Endpoint: `https://api.kimi.com/coding/v1/usages`
//! Auth: `Authorization: Bearer <api_key>`
//!
//! Response shape (from cc-switch MIT reference impl, 2026-07):
//! ```json
//! {
//!   "limits": [
//!     {
//!       "detail": {
//!         "limit": <number>,
//!         "remaining": <number>,
//!         "resetTime": <ISO 8601 string | epoch number>
//!       }
//!     }
//!   ],
//!   "usage": {
//!     "limit": <number>,
//!     "remaining": <number>,
//!     "resetTime": <ISO 8601 string | epoch number>
//!   }
//! }
//! ```
//!
//! Per response we emit:
//! - 5-hour tier from the first `limits[].detail` entry (Kimi ships one).
//! - Weekly tier from top-level `usage`.
//!
//! Tier computation: `used_percent = (limit - remaining) / limit * 100`,
//! `remaining_percent = 100 - used_percent`. `limit` missing or `<= 0`
//! skips the tier rather than emit a misleading reading.
//!
//! Design pattern adapted from cc-switch (MIT, Copyright 2025 Jason Young).
//! See [`../../../../docs/third-party/cc-switch.LICENSE`].
//!
//! SPDX-License-Identifier: MIT

use crate::provider::coding_plan::subscription::{
    extract_reset_ms, make_error, make_success, make_tier, parse_f64,
};
use crate::types::subscription::{
    CredentialStatus, QuotaTier, QuotaTierKind, SubscriptionQuota, TierStatus,
};

/// Kimi quota endpoint. Hard-coded — `base_url` only signals `api.kimi.com/coding`
/// for detection; Kimi ships a single global endpoint.
const USAGES_URL: &str = "https://api.kimi.com/coding/v1/usages";

const PROVIDER_ID: &str = "kimi";
const HTTP_TIMEOUT_SECS: u64 = 15;

/// Top-level entry point. Mirrors the cc-switch `query_kimi` signature:
/// `(base_url, api_key)` — `base_url` is unused after detection.
pub async fn fetch(_base_url: &str, api_key: &str) -> SubscriptionQuota {
    let client = reqwest::Client::new();

    let resp = client
        .get(USAGES_URL)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            return make_error(PROVIDER_ID, format!("Network: {e}"), CredentialStatus::Valid);
        }
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
        let body = resp.text().await.unwrap_or_default();
        return make_error(
            PROVIDER_ID,
            format!("HTTP {status}: {body}"),
            CredentialStatus::Valid,
        );
    }

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            return make_error(PROVIDER_ID, format!("Parse: {e}"), CredentialStatus::Valid);
        }
    };

    let tiers = parse_kimi_tiers(&body);
    make_success(PROVIDER_ID, tiers)
}

/// Pure parser for Kimi `/v1/usages` response. Extracted so unit tests can
/// hit it without a mock server.
pub(crate) fn parse_kimi_tiers(body: &serde_json::Value) -> Vec<QuotaTier> {
    let mut tiers = Vec::new();

    // 5-hour tier from the first `limits[].detail` entry (Kimi ships one).
    // ponytail: take the first entry rather than iterating all — duplicate
    // tier names would confuse the UI and Kimi documents a single bucket.
    if let Some(detail) = body
        .get("limits")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("detail"))
    {
        if let Some(tier) = parse_limit_tier(detail, QuotaTierKind::FiveHour, "Kimi 5h") {
            tiers.push(tier);
        }
    }

    // Weekly tier from top-level `usage`.
    if let Some(usage) = body.get("usage") {
        if let Some(tier) = parse_limit_tier(usage, QuotaTierKind::Weekly, "Kimi Weekly") {
            tiers.push(tier);
        }
    }

    tiers
}

/// Parse a `{ limit, remaining, resetTime }` object into a `QuotaTier`.
/// Returns `None` if `limit` is missing or `<= 0` (skips tier rather than
/// emit a misleading 100% reading — cc-switch's `unwrap_or(1.0)` fallback
/// would silently mark every missing-field response as fully consumed).
fn parse_limit_tier(obj: &serde_json::Value, kind: QuotaTierKind, label: &str) -> Option<QuotaTier> {
    let limit = obj.get("limit").and_then(parse_f64).filter(|&n| n > 0.0)?;
    let remaining = obj
        .get("remaining")
        .and_then(parse_f64)
        .unwrap_or(0.0)
        .clamp(0.0, limit);
    let used_percent = (limit - remaining) / limit * 100.0;
    let remaining_percent = 100.0 - used_percent;
    let resets_at_ms = obj.get("resetTime").and_then(extract_reset_ms);

    Some(make_tier(
        kind,
        label,
        Some(remaining_percent),
        resets_at_ms,
        TierStatus::Active,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // 2026-07-01T00:00:00Z = 1_782_864_000_000 ms
    // 2026-07-07T00:00:00Z = 1_783_382_400_000 ms (6 days later)

    #[test]
    fn kimi_parses_real_response_with_5h_and_weekly() {
        // Real Kimi shape: `limits[].detail` for 5h, top-level `usage` for weekly.
        let body = json!({
            "limits": [
                {
                    "detail": {
                        "limit": 100,
                        "remaining": 50,
                        "resetTime": "2026-07-01T00:00:00Z"
                    }
                }
            ],
            "usage": {
                "limit": 500,
                "remaining": 250,
                "resetTime": "2026-07-07T00:00:00Z"
            }
        });
        let tiers = parse_kimi_tiers(&body);
        assert_eq!(tiers.len(), 2);

        // 5-hour tier: 50/100 remaining → 50% remaining, 50% used.
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].label, "Kimi 5h");
        assert_eq!(tiers[0].remaining_percent, Some(50.0));
        assert_eq!(tiers[0].used_percent, Some(50.0));
        assert_eq!(tiers[0].resets_at_ms, Some(1_782_864_000_000));
        assert_eq!(tiers[0].status, TierStatus::Active);

        // Weekly tier: 250/500 remaining → 50% remaining, 50% used.
        assert_eq!(tiers[1].kind, QuotaTierKind::Weekly);
        assert_eq!(tiers[1].label, "Kimi Weekly");
        assert_eq!(tiers[1].remaining_percent, Some(50.0));
        assert_eq!(tiers[1].used_percent, Some(50.0));
        assert_eq!(tiers[1].resets_at_ms, Some(1_783_382_400_000));
        assert_eq!(tiers[1].status, TierStatus::Active);
    }

    #[test]
    fn kimi_missing_usage_emits_only_5h_tier() {
        // Some Kimi responses may omit the weekly `usage` block; we still
        // emit the 5-hour tier rather than fail.
        let body = json!({
            "limits": [
                {
                    "detail": {
                        "limit": 100,
                        "remaining": 25,
                        "resetTime": "2026-07-01T00:00:00Z"
                    }
                }
            ]
        });
        let tiers = parse_kimi_tiers(&body);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].remaining_percent, Some(25.0));
        assert_eq!(tiers[0].used_percent, Some(75.0));
    }

    #[test]
    fn kimi_zero_limit_skips_tier() {
        // ponytail: limit <= 0 (degenerate / unset) skips the tier rather
        // than emitting a misleading 100% reading via cc-switch's
        // `unwrap_or(1.0)` fallback.
        let body = json!({
            "limits": [
                {
                    "detail": {
                        "limit": 0,
                        "remaining": 0,
                        "resetTime": "2026-07-01T00:00:00Z"
                    }
                }
            ],
            "usage": {
                "limit": 100,
                "remaining": 80,
                "resetTime": "2026-07-07T00:00:00Z"
            }
        });
        let tiers = parse_kimi_tiers(&body);
        assert_eq!(tiers.len(), 1, "5h tier should be skipped on limit=0");
        assert_eq!(tiers[0].kind, QuotaTierKind::Weekly);
        assert_eq!(tiers[0].remaining_percent, Some(80.0));
    }

    #[test]
    fn kimi_empty_body_returns_empty() {
        let body = json!({});
        assert!(parse_kimi_tiers(&body).is_empty());
    }

    #[test]
    fn kimi_string_numeric_fallback() {
        // Defensive: if a proxy serializes limit/remaining as strings,
        // parse_f64 should still extract them.
        let body = json!({
            "limits": [
                {
                    "detail": {
                        "limit": "100",
                        "remaining": "30",
                        "resetTime": "2026-07-01T00:00:00Z"
                    }
                }
            ],
            "usage": {
                "limit": "1000",
                "remaining": "750",
                "resetTime": "2026-07-07T00:00:00Z"
            }
        });
        let tiers = parse_kimi_tiers(&body);
        assert_eq!(tiers.len(), 2);
        assert_eq!(tiers[0].remaining_percent, Some(30.0));
        assert_eq!(tiers[0].used_percent, Some(70.0));
        assert_eq!(tiers[1].remaining_percent, Some(75.0));
        assert_eq!(tiers[1].used_percent, Some(25.0));
    }

    #[test]
    fn kimi_remaining_exceeds_limit_clamps_to_zero_used() {
        // Defensive: upstream may report `remaining > limit` due to
        // rounding / race; we clamp to 0% used rather than negative.
        let body = json!({
            "limits": [
                {
                    "detail": {
                        "limit": 100,
                        "remaining": 150,
                        "resetTime": "2026-07-01T00:00:00Z"
                    }
                }
            ]
        });
        let tiers = parse_kimi_tiers(&body);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].remaining_percent, Some(100.0));
        assert_eq!(tiers[0].used_percent, Some(0.0));
    }
}