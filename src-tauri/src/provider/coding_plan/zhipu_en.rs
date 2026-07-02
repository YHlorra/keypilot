// SPDX-License-Identifier: MIT
//
// Zhipu GLM overseas (api.z.ai) coding plan provider.
//
// Original design pattern adapted from cc-switch (MIT, Copyright 2025 Jason
// Young), which ships a single `query_zhipu` covering both `open.bigmodel.cn`
// and `api.z.ai` because the two backends share the same quota path and JSON
// shape. See [`../../../../docs/third-party/cc-switch.LICENSE`](../../../../docs/third-party/cc-switch.LICENSE).

//! Zhipu GLM overseas (`api.z.ai`) coding plan provider.
//!
//! Endpoint: `https://api.z.ai/api/monitor/usage/quota/limit`.
//!
//! Auth: `Authorization: <api_key>` — **no `Bearer` prefix**, matching the
//! upstream Zhipu convention observed by cc-switch (`bigmodel.cn` and `z.ai`
//! both reject the standard `Bearer` scheme; the raw key is the credential).
//!
//! Headers (mirrors cc-switch `query_zhipu`):
//! - `Authorization: <api_key>`
//! - `Content-Type: application/json`
//! - `Accept-Language: en-US,en`
//!
//! Response shape (verified field-by-field against cc-switch parser):
//! ```json
//! {
//!   "success": true,
//!   "msg": "success",
//!   "data": {
//!     "level": "pro",
//!     "limits": [
//!       {
//!         "type": "TOKENS_LIMIT",
//!         "unit": 3,
//!         "number": 5,
//!         "percentage": 15.0,
//!         "nextResetTime": 1783008000000
//!       },
//!       {
//!         "type": "TOKENS_LIMIT",
//!         "unit": 6,
//!         "number": 7,
//!         "percentage": 50.0,
//!         "nextResetTime": 1783267200000
//!       }
//!     ]
//!   }
//! }
//! ```
//!
//! **Percentage semantics**: upstream `percentage` is the **used** percentage
//! (matches cc-switch `QuotaTier.utilization`). keypilot's [`QuotaTier`] stores
//! `remaining_percent`, so we invert as `100 - percentage`. We do NOT clamp
//! the upstream value — `percentage < 0` (over-return) and `> 100` (over-use)
//! pass through unchanged so the UI can display a truthful gauge. The
//! `make_tier` helper handles the final `[0, 100]` clamp on `remaining_percent`.
//!
//! Tier classification (`unit` field, anchored on value only — `number` is
//! not part of the contract; cc-switch observed both `unit:6 number:7` and
//! `unit:6 number:1` for the weekly window on `z.ai`):
//! - `unit: 3` ⇒ FiveHour
//! - `unit: 6` ⇒ Weekly
//! - unknown `unit` ⇒ fall back to reset-time heuristic: entries with no
//!   `nextResetTime` (5h bucket can be reset-less at 0%) fill the FiveHour
//!   slot first, then the remaining slots fill in ascending reset order.
//!
//! Old plans (pre-2026-02-12) emit a single `TOKENS_LIMIT` entry and degrade
//! to a FiveHour-only result — the unclassified fallback naturally handles
//! this.
//!
//! `data.level` (e.g. `"pro"` / `"max"`) is surfaced via
//! [`SubscriptionQuota::credential_message`] so the UI can show "GLM Coding
//! Plan — pro" without a second round-trip.

use crate::provider::coding_plan::subscription::{make_error, make_tier};
use crate::types::subscription::{CredentialStatus, QuotaTierKind, SubscriptionQuota, TierStatus};

/// Overseas quota endpoint. Hard-coded because `base_url` from the dispatcher
/// is the data plane (e.g. `https://api.z.ai/api/coding/paas/v4`) and the
/// quota path lives on the bare host with no path prefix in common.
const QUOTA_URL: &str = "https://api.z.ai/api/monitor/usage/quota/limit";

const HTTP_TIMEOUT_SECS: u64 = 15;

/// `unit` enum mirror — local to this module so the parser stays self-
/// contained. Values anchored against cc-switch's `classify_zhipu_window`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZhipuWindow {
    /// 5-hour rolling window (`unit: 3`).
    FiveHour,
    /// Weekly window (`unit: 6`).
    Weekly,
}

/// Top-level entry point. Mirrors the cc-switch `query_zhipu` signature:
/// `(base_url, api_key)`. `base_url` is currently unused for routing because
/// the dispatcher guarantees this module is only invoked when `base_url`
/// contains `api.z.ai` (see [`detect_provider`]); we accept the parameter for
/// signature symmetry with sibling providers so Lane C can wire up a single
/// dispatch arm without per-provider adapters.
///
/// [`detect_provider`]: crate::provider::coding_plan::detect_provider
pub async fn fetch(base_url: &str, api_key: &str) -> SubscriptionQuota {
    let provider_id = "zhipu_en";
    // ponytail: `base_url` reserved for symmetry; suppress unused-variable lint
    // without pulling a dependency on the `let _ = ...;` boilerplate pattern.
    let _ = base_url;

    let client = reqwest::Client::new();
    let resp = client
        .get(QUOTA_URL)
        .header("Authorization", api_key)
        .header("Content-Type", "application/json")
        .header("Accept-Language", "en-US,en")
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

    // Business-level error: success == false ⇒ upstream rejected the request
    // despite a 2xx transport reply. Surface `msg` so the UI can show the
    // exact reason (e.g. quota-locked vs. region-mismatch).
    if body.get("success").and_then(|v| v.as_bool()) == Some(false) {
        let msg = body
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return make_error(
            provider_id,
            format!("API error: {msg}"),
            CredentialStatus::Valid,
        );
    }

    let data = match body.get("data") {
        Some(d) => d,
        None => {
            return make_error(
                provider_id,
                "Missing 'data' field in response".to_string(),
                CredentialStatus::Valid,
            );
        }
    };

    let tiers = parse_zhipu_en_tiers(data);

    let level = data
        .get("level")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    SubscriptionQuota {
        provider_id: provider_id.to_string(),
        credential_status: CredentialStatus::Valid,
        credential_message: level,
        success: true,
        tiers,
        error: None,
        queried_at_ms: crate::provider::coding_plan::subscription::now_millis(),
    }
}

/// Pure parser extracted so unit tests can hit it without a mock server.
///
/// Mirrors cc-switch `parse_zhipu_token_tiers` semantics, adapted to
/// keypilot's `QuotaTier` shape (`remaining_percent = 100 - upstream_percentage`).
pub(crate) fn parse_zhipu_en_tiers(data: &serde_json::Value) -> Vec<crate::types::subscription::QuotaTier> {
    type Entry = (Option<i64>, Option<f64>);

    let mut five_hour: Option<Entry> = None;
    let mut weekly: Option<Entry> = None;
    let mut unclassified: Vec<Entry> = Vec::new();

    let limits = match data.get("limits").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };

    for limit_item in limits {
        // ponytail: case-insensitive `type` match mirrors cc-switch; the
        // upstream has not actually shipped a non-`TOKENS_LIMIT` value but
        // the defensive read costs nothing.
        let limit_type = limit_item
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !limit_type.eq_ignore_ascii_case("TOKENS_LIMIT") {
            continue;
        }

        let used_pct = limit_item.get("percentage").and_then(|v| v.as_f64());
        let reset_ms = limit_item.get("nextResetTime").and_then(|v| v.as_i64());
        let entry = (reset_ms, used_pct);

        match classify_zhipu_window(limit_item) {
            Some(ZhipuWindow::FiveHour) if five_hour.is_none() => five_hour = Some(entry),
            Some(ZhipuWindow::Weekly) if weekly.is_none() => weekly = Some(entry),
            _ => unclassified.push(entry),
        }
    }

    // ponytail: fallback heuristic — entries the `unit` field could not
    // classify get sorted by reset time and fill whichever slot is still
    // empty. Entries with no `nextResetTime` (5h bucket at 0% usage can be
    // reset-less) are pushed to the front so they preferentially fill the
    // FiveHour slot.
    unclassified.sort_by_key(|(reset, _)| (reset.is_none(), reset.unwrap_or(i64::MIN)));
    for entry in unclassified {
        if five_hour.is_none() {
            five_hour = Some(entry);
        } else if weekly.is_none() {
            weekly = Some(entry);
        }
        // Zhipu currently emits at most two TOKENS_LIMIT entries; anything
        // beyond that is dropped on purpose (matches cc-switch).
    }

    let mut tiers = Vec::new();
    if let Some((reset_ms, used_pct)) = five_hour {
        tiers.push(make_tier(
            QuotaTierKind::FiveHour,
            "GLM 5h",
            used_pct.map(|p| 100.0 - p),
            reset_ms,
            TierStatus::Active,
        ));
    }
    if let Some((reset_ms, used_pct)) = weekly {
        tiers.push(make_tier(
            QuotaTierKind::Weekly,
            "GLM Weekly",
            used_pct.map(|p| 100.0 - p),
            reset_ms,
            TierStatus::Active,
        ));
    }
    tiers
}

/// Classify a single `limits[]` entry by its `unit` field.
///
/// `unit: 3` ⇒ FiveHour; `unit: 6` ⇒ Weekly. Anything else (missing, null,
/// unrecognised integer) returns `None` so the caller can run the
/// reset-time fallback heuristic.
fn classify_zhipu_window(item: &serde_json::Value) -> Option<ZhipuWindow> {
    match item.get("unit").and_then(|v| v.as_i64()) {
        Some(3) => Some(ZhipuWindow::FiveHour),
        Some(6) => Some(ZhipuWindow::Weekly),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::subscription::QuotaTierKind;

    // ── classify_zhipu_window ─────────────────────────────────

    #[test]
    fn classify_zhipu_window_known_units() {
        let v_five = serde_json::json!({ "unit": 3, "number": 5 });
        let v_week = serde_json::json!({ "unit": 6, "number": 7 });
        assert_eq!(classify_zhipu_window(&v_five), Some(ZhipuWindow::FiveHour));
        assert_eq!(classify_zhipu_window(&v_week), Some(ZhipuWindow::Weekly));
    }

    #[test]
    fn classify_zhipu_window_unknown_or_missing_returns_none() {
        assert_eq!(classify_zhipu_window(&serde_json::json!({})), None);
        assert_eq!(classify_zhipu_window(&serde_json::json!({ "unit": 9 })), None);
        assert_eq!(classify_zhipu_window(&serde_json::json!({ "unit": "3" })), None);
    }

    // ── parse_zhipu_en_tiers ──────────────────────────────────

    #[test]
    fn parse_zhipu_en_tiers_inverts_percentage_and_emits_both_windows() {
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
                    "percentage": 50.0,
                    "nextResetTime": 1_783_267_200_000_i64
                }
            ]
        });
        let tiers = parse_zhipu_en_tiers(&data);
        assert_eq!(tiers.len(), 2);

        // 5h: upstream `percentage: 15` ⇒ used 15% ⇒ remaining 85%
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].label, "GLM 5h");
        assert_eq!(tiers[0].remaining_percent, Some(85.0));
        assert_eq!(tiers[0].used_percent, Some(15.0));
        assert_eq!(tiers[0].resets_at_ms, Some(1_783_008_000_000));

        // Weekly: upstream `percentage: 50` ⇒ used 50% ⇒ remaining 50%
        assert_eq!(tiers[1].kind, QuotaTierKind::Weekly);
        assert_eq!(tiers[1].label, "GLM Weekly");
        assert_eq!(tiers[1].remaining_percent, Some(50.0));
        assert_eq!(tiers[1].used_percent, Some(50.0));
        assert_eq!(tiers[1].resets_at_ms, Some(1_783_267_200_000));
    }

    #[test]
    fn parse_zhipu_en_tiers_handles_old_plan_single_tier() {
        // Pre-2026-02-12 plans emit only one TOKENS_LIMIT entry → only the
        // 5h tier is produced; the fallback heuristic slots it into
        // FiveHour because the unclassified bucket's first entry always wins.
        let data = serde_json::json!({
            "level": "lite",
            "limits": [{
                "type": "TOKENS_LIMIT",
                "unit": 99, // unrecognised — falls through to heuristic
                "number": 1,
                "percentage": 25.0,
                "nextResetTime": 1_783_008_000_000_i64
            }]
        });
        let tiers = parse_zhipu_en_tiers(&data);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].remaining_percent, Some(75.0));
    }

    #[test]
    fn parse_zhipu_en_tiers_skips_non_tokens_limit_entries() {
        // Non-TOKENS_LIMIT entries (e.g. future billing meters) must be
        // ignored outright, not routed into the fallback heuristic.
        let data = serde_json::json!({
            "level": "pro",
            "limits": [
                { "type": "RPM_LIMIT", "unit": 3, "percentage": 99.0 },
                {
                    "type": "TOKENS_LIMIT",
                    "unit": 3,
                    "number": 5,
                    "percentage": 20.0,
                    "nextResetTime": 1_783_008_000_000_i64
                }
            ]
        });
        let tiers = parse_zhipu_en_tiers(&data);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].remaining_percent, Some(80.0));
    }

    #[test]
    fn parse_zhipu_en_tiers_empty_when_no_limits() {
        let data = serde_json::json!({ "level": "pro" });
        assert!(parse_zhipu_en_tiers(&data).is_empty());
    }
}