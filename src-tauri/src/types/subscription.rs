//! Subscription quota model for coding plan providers (Kimi / GLM / MiniMax /
//! Volcengine / ZenMux).
//!
//! Distinct from [`crate::types::QuotaSnapshot`] (USD billing model). Coding
//! plans expose 5-hour / weekly / monthly **percentage + time window** data,
//! no USD amounts.
//!
//! Design inspired by cc-switch (MIT, Copyright 2025 Jason Young). See
//! [`../../docs/third-party/cc-switch.LICENSE`](../../docs/third-party/cc-switch.LICENSE)
//! for the verbatim license copy.

use serde::{Deserialize, Serialize};

/// Top-level result of a coding-plan quota fetch.
///
/// `success = true` ⇒ `tiers` is the source of truth; `error` is None.
/// `success = false` ⇒ `error` is set; UI may still surface
/// `credential_status` (e.g. `Invalid` for 401) so users can act on it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionQuota {
    /// Stable provider id (e.g. `"minimax"` / `"kimi"`). Lowercase, no spaces.
    pub provider_id: String,
    /// Whether the API key / OAuth token is still accepted by the upstream.
    pub credential_status: CredentialStatus,
    /// Optional human message accompanying `credential_status` (plan tier,
    /// expiry reason, etc.). Empty when not applicable.
    pub credential_message: Option<String>,
    /// `true` iff the upstream returned a parseable, successful response.
    pub success: bool,
    /// Quota windows in display order (5-hour first, weekly second, monthly
    /// third when present). Empty on failure.
    pub tiers: Vec<QuotaTier>,
    /// Top-level error message; surfaced in UI footer when `success = false`.
    pub error: Option<String>,
    /// Local Unix epoch milliseconds when the fetch completed. Useful for
    /// cache TTL math on the frontend.
    pub queried_at_ms: i64,
}

/// Coarse classification of credential validity.
///
/// Mirrors cc-switch's `CredentialStatus` (Valid / Expired / NotFound /
/// ParseError) collapsed into 4 cases relevant to keypilot's cached display:
/// - `Valid`: upstream accepted the key (2xx + parseable body).
/// - `Invalid`: HTTP 401 / 403 — user must rotate the key.
/// - `Expired`: upstream explicitly reported expiration / quota-locked state.
/// - `Unknown`: transport error or unparseable response — UI shows retry hint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CredentialStatus {
    Valid,
    Invalid,
    Expired,
    Unknown,
}

/// One quota window (5-hour, weekly, or monthly).
///
/// Mirrors `QuotaTier` in `cc-switch::services::subscription`; field names
/// aligned to keypilot's existing `LimitWindow` shape (`used_percent` /
/// `remaining_percent`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaTier {
    /// Window category.
    pub kind: QuotaTierKind,
    /// Human-readable label for UI (e.g. "5-hour", "Weekly", "Monthly").
    /// Provider-specific prefixes are allowed (e.g. "General 5h").
    pub label: String,
    /// Absolute usage if the upstream reports it (some coding plans only
    /// give percentage). `None` for percentage-only providers like MiniMax.
    pub used: Option<f64>,
    /// Absolute limit if reported; `None` when only percentage is available.
    pub limit: Option<f64>,
    /// Used percentage 0–100. Clamped at parse time (see helpers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_percent: Option<f64>,
    /// Remaining percentage 0–100. Clamped at parse time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_percent: Option<f64>,
    /// Reset time as Unix epoch milliseconds (preferred for cache math).
    /// Always stored as `i64` ms to match `timeutil::now_millis()`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resets_at_ms: Option<i64>,
    /// Free-form reset hint (e.g. "Resets in 2h 30m"). Empty when unknown.
    pub reset_description: String,
    /// Whether this tier is currently active. Inactive tiers (e.g. weekly
    /// limit disabled on a base plan) are still emitted so the UI can
    /// explain the absence.
    pub status: TierStatus,
}

/// Coding-plan window category.
///
/// Aligned to `LimitWindowKind` (Session / Weekly / Billing) when possible,
/// with `FiveHour` = Session renamed for clarity in coding-plan context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuotaTierKind {
    /// 5-hour rolling window (most coding plans).
    FiveHour,
    /// 7-day window (Kimi / MiniMax weekly tier).
    Weekly,
    /// Calendar-month window (Volcengine AFP / Coding Plan monthly tier).
    Monthly,
}

/// Activation flag for a `QuotaTier`.
///
/// Mirrors `current_interval_status` / `current_weekly_status` semantics in
/// the MiniMax API: `1` ⇒ active, anything else ⇒ inactive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TierStatus {
    Active,
    Inactive,
    Unknown,
}
