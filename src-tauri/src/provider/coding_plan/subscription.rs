//! Shared helpers for coding plan providers.
//!
//! Pure functions only — no I/O. Per-provider modules wire these into their
//! specific JSON shapes.
//!
//! Design inspired by cc-switch (MIT, Copyright 2025 Jason Young). See
//! [`../../../../docs/third-party/cc-switch.LICENSE`](../../../../docs/third-party/cc-switch.LICENSE).

use crate::types::subscription::{CredentialStatus, QuotaTier, QuotaTierKind, SubscriptionQuota, TierStatus};

/// Current Unix epoch milliseconds. Mirrors `timeutil::now_millis()` but
/// stays local to this module so unit tests don't have to depend on the
/// shared clock.
pub fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Parse a JSON value as `f64`, accepting both JSON number and numeric string.
///
/// Returns `None` for null, non-numeric strings, booleans, arrays, and objects.
/// Whitespace around string digits is not trimmed (callers pass raw JSON).
pub fn parse_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}

/// Extract a reset timestamp from JSON, supporting ISO 8601 strings and
/// Unix epoch integers (seconds or milliseconds).
///
/// Returns the timestamp as Unix epoch milliseconds, normalized so that
/// downstream code can compare / diff without second-vs-ms accidents.
/// Values `<= 0` return `None` (matches cc-switch behavior for the
/// Volcengine session-no-active-window `-1` sentinel).
pub fn extract_reset_ms(value: &serde_json::Value) -> Option<i64> {
    if let Some(s) = value.as_str() {
        // Try ISO 8601 first; if parse fails, fall through to numeric path.
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            return Some(dt.timestamp_millis());
        }
        // Fall through — some providers embed a numeric string.
        if let Ok(n) = s.parse::<i64>() {
            return normalize_epoch(n);
        }
        return None;
    }
    if let Some(n) = value.as_i64() {
        return normalize_epoch(n);
    }
    None
}

/// Convert raw seconds/milliseconds to canonical milliseconds.
fn normalize_epoch(n: i64) -> Option<i64> {
    if n <= 0 {
        return None;
    }
    // ponytail: 1e12 is the same heuristic cc-switch uses (seconds < 1e12,
    // milliseconds >= 1e12). Good enough; leap-second edge cases are not
    // worth a date-time library round-trip here.
    Some(if n < 1_000_000_000_000 { n * 1000 } else { n })
}

/// Build a [`QuotaTier`] from raw values.
///
/// `remaining_percent` is clamped to `[0, 100]`. `used_percent` is derived
/// as `100 - remaining_percent` (matching cc-switch's MiniMax parsing).
/// `used` and `limit` are left `None` because most coding-plan providers
/// do not expose absolute values; providers that do (ZenMux USD) populate
/// them directly via the `QuotaTier` constructor instead of this helper.
pub fn make_tier(
    kind: QuotaTierKind,
    label: &str,
    remaining_percent: Option<f64>,
    resets_at_ms: Option<i64>,
    status: TierStatus,
) -> QuotaTier {
    let remaining_percent_norm = remaining_percent.map(|p| p.clamp(0.0, 100.0));
    let used_percent = remaining_percent_norm.map(|p| 100.0 - p);
    QuotaTier {
        kind,
        label: label.to_string(),
        used: None,
        limit: None,
        used_percent,
        remaining_percent: remaining_percent_norm,
        resets_at_ms,
        reset_description: String::new(),
        status,
    }
}

/// Build a failure [`SubscriptionQuota`] with the given provider id, message,
/// and credential status. Used by per-provider modules when they need to
/// return early (network error, parse error, HTTP non-2xx).
///
/// `credential_status` defaults to `Valid` (transport-layer error, key
/// itself is fine). Callers that detect 401/403 should pass `Invalid`.
pub fn make_error(
    provider_id: &str,
    msg: String,
    credential_status: CredentialStatus,
) -> SubscriptionQuota {
    SubscriptionQuota {
        provider_id: provider_id.to_string(),
        credential_status,
        credential_message: None,
        success: false,
        tiers: vec![],
        error: Some(msg),
        queried_at_ms: now_millis(),
    }
}

/// Build a success [`SubscriptionQuota`] with the given provider id and
/// tier list. Helper for symmetry with [`make_error`] and to centralize the
/// default `credential_status = Valid` / `queried_at_ms = now()` defaults.
pub fn make_success(provider_id: &str, tiers: Vec<QuotaTier>) -> SubscriptionQuota {
    SubscriptionQuota {
        provider_id: provider_id.to_string(),
        credential_status: CredentialStatus::Valid,
        credential_message: None,
        success: true,
        tiers,
        error: None,
        queried_at_ms: now_millis(),
    }
}
