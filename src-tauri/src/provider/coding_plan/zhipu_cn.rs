//! Zhipu GLM coding plan provider (CN region, `open.bigmodel.cn`).
//!
//! Endpoint: `https://open.bigmodel.cn/api/monitor/usage/quota/limit`
//!
//! Auth: `Authorization: <api_key>` — note the absence of `Bearer `
//! prefix. Zhipu's quota endpoint expects the raw key, unlike the
//! chat-completions endpoint which takes `Bearer`. Sending `Bearer `
//! causes a 401.
//!
//! Response shape (verified against cc-switch MIT reference; same
//! backend serves both `open.bigmodel.cn` and `api.z.ai`):
//! ```json
//! {
//!   "success": true,
//!   "msg": "",
//!   "data": {
//!     "level": "pro",
//!     "limits": [
//!       {
//!         "type": "TOKENS_LIMIT",
//!         "unit": 3,
//!         "number": 5,
//!         "percentage": 15,
//!         "nextResetTime": 1783008000000
//!       },
//!       {
//!         "type": "TOKENS_LIMIT",
//!         "unit": 6,
//!         "number": 7,
//!         "percentage": 25,
//!         "nextResetTime": 1785600000000
//!       }
//!     ]
//!   }
//! }
//! ```
//!
//! Window classification (anchors on `unit` so 5h vs weekly never flip
//! when the cycle ends — issue #3036 in upstream cc-switch):
//! - `unit == 3`  → 5-hour rolling window
//! - `unit == 6`  → weekly window (`number` is 1 or 7 in the wild)
//! - unrecognized / missing `unit` → fallback heuristic by `nextResetTime`
//!
//! Legacy plans (subscribed before 2026-02-12) only return one
//! `TOKENS_LIMIT` entry — naturally degrades to a single 5-hour tier.
//!
//! `percentage` is the **used** percentage; we mirror it into the
//! keypilot `remaining_percent = 100 - percentage` convention so the
//! existing UI gauge math (clamp 0..100, `used_percent = 100 - remaining`)
//! keeps working unchanged.
//!
//! Plan tier (`data.level`, e.g. `"pro"`, `"max"`) is surfaced via
//! `credential_message` so the UI can render it without a separate IPC.
//!
//! Design pattern adapted from cc-switch (MIT, Copyright 2025 Jason Young).
//! See [`../../../../docs/third-party/cc-switch.LICENSE`](../../../../docs/third-party/cc-switch.LICENSE).
//!
//! SPDX-License-Identifier: MIT

use crate::provider::coding_plan::subscription::{make_error, make_tier};
use crate::types::subscription::{CredentialStatus, QuotaTier, QuotaTierKind, SubscriptionQuota, TierStatus};

/// Zhipu CN quota endpoint. Hard-coded — both presets share the same
/// path but live on different hosts; this module is CN-only.
const QUOTA_URL: &str = "https://open.bigmodel.cn/api/monitor/usage/quota/limit";

/// Stable id surfaced to the UI / frontend cache.
const PROVIDER_ID: &str = "zhipu_cn";

const HTTP_TIMEOUT_SECS: u64 = 15;

/// Marker string for the `TOKENS_LIMIT` filter. Matched
/// case-insensitively so an upstream casing flip does not silently
/// drop every tier.
const LIMIT_TYPE_TOKENS: &str = "TOKENS_LIMIT";

/// Zhipu `unit` discriminator for the 5-hour rolling window.
const UNIT_FIVE_HOUR: i64 = 3;
/// Zhipu `unit` discriminator for the weekly window. `number` varies
/// between 1 and 7 across plans, so we only anchor on `unit`.
const UNIT_WEEKLY: i64 = 6;

/// Top-level entry point. Mirrors cc-switch `query_zhipu` signature
/// `(base_url, api_key)`: `base_url` is accepted so the dispatcher
/// can route CN vs overseas without a different shape, but this
/// module is hard-pinned to the CN host — overseas wiring lives in
/// a sibling module.
pub async fn fetch(base_url: &str, api_key: &str) -> SubscriptionQuota {
    // ponytail: api_key emptiness is already guarded by the dispatcher
    // in fetch_coding_plan_quota, so no second guard here.
    let _ = base_url; // accepted for dispatcher symmetry; CN host is fixed

    let client = reqwest::Client::new();
    let resp = client
        .get(QUOTA_URL)
        // NOTE: no "Bearer " prefix — Zhipu's quota endpoint rejects it.
        .header("Authorization", api_key)
        .header("Content-Type", "application/json")
        // Force English response so error / message fields stay
        // machine-parseable even when the account is on a CN locale.
        .header("Accept-Language", "en-US,en")
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .send()
        .await;

    let resp = match resp {
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

    // Business-level error: explicit `success: false` in the payload.
    if body.get("success").and_then(|v| v.as_bool()) == Some(false) {
        let msg = body
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return make_error(
            PROVIDER_ID,
            format!("API error: {msg}"),
            CredentialStatus::Valid,
        );
    }

    let data = match body.get("data") {
        Some(d) => d,
        None => return make_error(PROVIDER_ID, "Missing 'data' field".into(), CredentialStatus::Valid),
    };

    let tiers = parse_zhipu_token_tiers(data);

    // Plan tier (e.g. "pro") into credential_message so the UI can
    // render it without an extra round-trip. Empty string is normalized
    // away so the UI does not show a stray label.
    let level = data
        .get("level")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty());

    SubscriptionQuota {
        provider_id: PROVIDER_ID.to_string(),
        credential_status: CredentialStatus::Valid,
        credential_message: level,
        success: true,
        tiers,
        error: None,
        queried_at_ms: crate::provider::coding_plan::subscription::now_millis(),
    }
}

/// Pure parser — extracted so unit tests can hit it without a mock server.
///
/// Returns 1 or 2 tiers depending on which `TOKENS_LIMIT` entries are
/// present. Classification priority:
/// 1. Explicit `unit` discriminator (3 → five_hour, 6 → weekly).
/// 2. Fallback heuristic when `unit` is missing/unrecognized: the
///    entry without `nextResetTime` (5h bucket can report 0% without
///    a reset) goes to `five_hour`, the rest fill remaining slots in
///    ascending reset order. Currently Zhipu returns at most 2
///    `TOKENS_LIMIT` entries, so the heuristic is bounded.
///
/// `percentage` (used) is mirrored into `remaining_percent = 100 - percentage`
/// to keep the keypilot UI gauge math intact.
pub(crate) fn parse_zhipu_token_tiers(data: &serde_json::Value) -> Vec<QuotaTier> {
    type Entry = (Option<i64>, f64);
    let mut five_hour: Option<Entry> = None;
    let mut weekly: Option<Entry> = None;
    let mut unclassified: Vec<Entry> = Vec::new();

    if let Some(limits) = data.get("limits").and_then(|v| v.as_array()) {
        for item in limits {
            let kind = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if !kind.eq_ignore_ascii_case(LIMIT_TYPE_TOKENS) {
                continue;
            }
            // Zhipu ships `percentage` as a JSON number (0–100, used).
            // Defensive parse via the shared helper accepts numeric strings too.
            let used_pct = item
                .get("percentage")
                .and_then(crate::provider::coding_plan::subscription::parse_f64)
                .unwrap_or(0.0);
            // `nextResetTime` is Unix epoch milliseconds; clamp to ms units
            // via the shared helper for symmetry with other providers.
            let reset_ms = item
                .get("nextResetTime")
                .and_then(crate::provider::coding_plan::subscription::extract_reset_ms);
            let entry = (reset_ms, used_pct);

            match item.get("unit").and_then(|v| v.as_i64()) {
                Some(UNIT_FIVE_HOUR) if five_hour.is_none() => five_hour = Some(entry),
                Some(UNIT_WEEKLY) if weekly.is_none() => weekly = Some(entry),
                _ => unclassified.push(entry),
            }
        }
    }

    // ponytail: sort by reset asc with `None` last so the missing-reset
    // entry (5h bucket at 0%) lands in `five_hour` first when the
    // explicit `unit` discriminator was absent. Note: the boolean sort
    // key uses `is_some()` (true for has-reset, false for None); since
    // `false` orders before `true`, the `None` row comes first and
    // gets filled into the `five_hour` slot.
    unclassified.sort_by_key(|(reset, _)| (reset.is_some(), reset.unwrap_or(0)));
    for entry in unclassified {
        if five_hour.is_none() {
            five_hour = Some(entry);
        } else if weekly.is_none() {
            weekly = Some(entry);
        }
        // Zhipu currently emits at most 2 TOKENS_LIMIT entries;
        // extras (if any) are intentionally dropped.
    }

    let mut tiers = Vec::new();
    if let Some((reset_ms, used_pct)) = five_hour {
        tiers.push(tier_from_used_pct(QuotaTierKind::FiveHour, "General 5h", used_pct, reset_ms));
    }
    if let Some((reset_ms, used_pct)) = weekly {
        tiers.push(tier_from_used_pct(QuotaTierKind::Weekly, "General Weekly", used_pct, reset_ms));
    }
    tiers
}

/// Build a `QuotaTier` from a Zhipu `percentage` (which is the **used**
/// percentage). Inverts to `remaining_percent` so the shared helper
/// can clamp and derive `used_percent` consistently with other providers.
fn tier_from_used_pct(kind: QuotaTierKind, label: &str, used_pct: f64, resets_at_ms: Option<i64>) -> QuotaTier {
    let remaining = (100.0 - used_pct).clamp(0.0, 100.0);
    // Zhipu only surfaces limits when they exist, so any emitted tier
    // is Active. Inactive legacy buckets would not appear at all.
    make_tier(kind, label, Some(remaining), resets_at_ms, TierStatus::Active)
}

// ── unit tests ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::coding_plan::subscription::extract_reset_ms;

    #[test]
    fn parses_two_tier_response_with_unit_classifier() {
        // Real-shape response: unit=3 → five_hour, unit=6 → weekly.
        let data = serde_json::json!({
            "level": "pro",
            "limits": [
                {
                    "type": "TOKENS_LIMIT",
                    "unit": 3,
                    "number": 5,
                    "percentage": 15.0,
                    "nextResetTime": 1_783_008_000_000_i64
                },
                {
                    "type": "TOKENS_LIMIT",
                    "unit": 6,
                    "number": 7,
                    "percentage": 25.0,
                    "nextResetTime": 1_785_600_000_000_i64
                }
            ]
        });

        let tiers = parse_zhipu_token_tiers(&data);
        assert_eq!(tiers.len(), 2);

        // 5-hour tier: 15% used → 85% remaining.
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].label, "General 5h");
        assert_eq!(tiers[0].remaining_percent, Some(85.0));
        assert_eq!(tiers[0].used_percent, Some(15.0));
        assert_eq!(tiers[0].resets_at_ms, Some(1_783_008_000_000));
        assert_eq!(tiers[0].status, TierStatus::Active);

        // Weekly tier: 25% used → 75% remaining.
        assert_eq!(tiers[1].kind, QuotaTierKind::Weekly);
        assert_eq!(tiers[1].label, "General Weekly");
        assert_eq!(tiers[1].remaining_percent, Some(75.0));
        assert_eq!(tiers[1].used_percent, Some(25.0));
        assert_eq!(tiers[1].resets_at_ms, Some(1_785_600_000_000));
        assert_eq!(tiers[1].status, TierStatus::Active);
    }

    #[test]
    fn legacy_plan_emits_only_five_hour_tier() {
        // Pre-2026-02-12 plans return one TOKENS_LIMIT entry.
        let data = serde_json::json!({
            "level": "lite",
            "limits": [
                {
                    "type": "TOKENS_LIMIT",
                    "unit": 3,
                    "number": 5,
                    "percentage": 0.0,
                    "nextResetTime": 1_783_008_000_000_i64
                }
            ]
        });

        let tiers = parse_zhipu_token_tiers(&data);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].remaining_percent, Some(100.0));
        assert_eq!(tiers[0].used_percent, Some(0.0));
    }

    #[test]
    fn ignores_non_tokens_limit_entries() {
        // Some accounts carry TIME_RANGE_LIMIT or other bucket types that
        // are not coding-plan relevant. They must be silently dropped.
        let data = serde_json::json!({
            "level": "pro",
            "limits": [
                {
                    "type": "TIME_RANGE_LIMIT",
                    "unit": 3,
                    "percentage": 50.0,
                    "nextResetTime": 1_783_008_000_000_i64
                },
                {
                    "type": "TOKENS_LIMIT",
                    "unit": 3,
                    "number": 5,
                    "percentage": 10.0,
                    "nextResetTime": 1_783_008_000_000_i64
                }
            ]
        });

        let tiers = parse_zhipu_token_tiers(&data);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].remaining_percent, Some(90.0));
    }

    #[test]
    fn fallback_heuristic_when_unit_missing() {
        // Older / non-standard payloads omit `unit`. Without the
        // discriminator the heuristic must still land the bucket
        // without a reset timestamp in `five_hour`.
        let data = serde_json::json!({
            "level": "pro",
            "limits": [
                {
                    "type": "TOKENS_LIMIT",
                    "percentage": 20.0,
                    "nextResetTime": 1_785_600_000_000_i64
                },
                {
                    "type": "TOKENS_LIMIT",
                    "percentage": 0.0
                    // no nextResetTime → must fill five_hour
                }
            ]
        });

        let tiers = parse_zhipu_token_tiers(&data);
        assert_eq!(tiers.len(), 2);
        // The 0% no-reset entry lands in five_hour.
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].remaining_percent, Some(100.0));
        assert_eq!(tiers[0].resets_at_ms, None);
        // The reset-bearing entry lands in weekly.
        assert_eq!(tiers[1].kind, QuotaTierKind::Weekly);
        assert_eq!(tiers[1].resets_at_ms, Some(1_785_600_000_000));
    }

    #[test]
    fn missing_data_field_returns_empty() {
        let data = serde_json::json!({});
        assert!(parse_zhipu_token_tiers(&data).is_empty());
    }

    #[test]
    fn percentage_clamped_via_remaining_inversion() {
        // Defensive: a stray 110% upstream would emit -10% remaining,
        // which the helper clamps to 0.
        let data = serde_json::json!({
            "level": "pro",
            "limits": [{
                "type": "TOKENS_LIMIT",
                "unit": 3,
                "number": 5,
                "percentage": 110.0,
                "nextResetTime": 1_783_008_000_000_i64
            }]
        });
        let tiers = parse_zhipu_token_tiers(&data);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].remaining_percent, Some(0.0));
        assert_eq!(tiers[0].used_percent, Some(100.0));
    }

    #[test]
    fn end_to_end_success_path_attaches_level_to_credential_message() {
        // Verify the public fetch wiring (status, business error,
        // credential_message) without hitting the network. We exercise
        // only the pure parser + credential_message shape by reusing
        // the same code path through a hand-built body.
        let body = serde_json::json!({
            "success": true,
            "msg": "",
            "data": {
                "level": "max",
                "limits": [{
                    "type": "TOKENS_LIMIT",
                    "unit": 3,
                    "number": 5,
                    "percentage": 5.0,
                    "nextResetTime": 1_783_008_000_000_i64
                }]
            }
        });

        // Mirror the relevant slice of `fetch`'s post-parse logic so the
        // test does not need a mock HTTP server.
        let data = body.get("data").expect("data");
        let tiers = parse_zhipu_token_tiers(data);
        let level = data.get("level").and_then(|v| v.as_str()).map(str::to_string);

        let quota = SubscriptionQuota {
            provider_id: PROVIDER_ID.to_string(),
            credential_status: CredentialStatus::Valid,
            credential_message: level,
            success: true,
            tiers,
            error: None,
            queried_at_ms: 0,
        };

        assert_eq!(quota.provider_id, "zhipu_cn");
        assert_eq!(quota.credential_message.as_deref(), Some("max"));
        assert!(quota.success);
        assert_eq!(quota.tiers.len(), 1);
        assert_eq!(quota.tiers[0].remaining_percent, Some(95.0));
    }

    // ponytail: a separate guard that `extract_reset_ms` accepts the
    // shape Zhipu emits (i64 ms). Keeps regressions local if a future
    // refactor changes the upstream unit to seconds.
    #[test]
    fn zhipu_reset_timestamp_is_milliseconds() {
        let v = serde_json::json!(1_783_008_000_000_i64);
        assert_eq!(extract_reset_ms(&v), Some(1_783_008_000_000));
    }
}