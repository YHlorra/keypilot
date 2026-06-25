use serde::{Deserialize, Serialize};
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Visible,
    Masked,
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Visible => "visible",
            Self::Masked => "masked",
        }
    }

    pub fn parse(s: &str) -> Result<Self, AppError> {
        match s {
            "visible" => Ok(Self::Visible),
            "masked" => Ok(Self::Masked),
            _ => Err(AppError::InvalidVisibility(s.into())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderField {
    pub id: i64,
    pub provider_id: i64,
    pub key: String,
    pub value: String,
    pub visibility: Visibility,
    pub sort_index: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: i64,
    pub name: String,
    pub preset: Option<String>,
    pub is_preset: bool,
    pub category_id: i64,
    pub pinned: bool,
    pub notes: Option<String>,
    pub icon: Option<String>,
    pub icon_color: Option<String>,
    pub sort_index: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub fields: Vec<ProviderField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub is_default: bool,
    pub sort_index: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    Auto,
}

impl Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::Auto => "auto",
        }
    }

    pub fn parse(s: &str) -> Result<Self, AppError> {
        match s {
            "dark" => Ok(Self::Dark),
            "light" => Ok(Self::Light),
            "auto" => Ok(Self::Auto),
            _ => Err(AppError::InvalidTheme(s.into())),
        }
    }
}

/// QuotaSnapshot (Stage 4 + 7): per-preset shape reconciled to common denominator.
/// @see openspec/changes/v0.1-general-credentials/spec.md REQ-QUOTA-001~006
/// @see openspec/changes/v0.1-general-credentials/spec.md REQ-QUOTA-DISPLAY-001
/// Per-preset shapes:
/// - 3 LLM (OpenAI/DeepSeek): { total, used, remaining, unit='USD'|'token', level?, reset_at? }
/// - GitHub: { total, used, remaining, unit='req', level?, reset_at? }
/// - PostgreSQL: { total=None, used, unit='GB', level? }  (no remaining, no reset_at)
/// - Anthropic: AppError::ProviderQuotaUnsupported (no QuotaSnapshot returned)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaSnapshot {
    /// Total quota limit (None for PostgreSQL which has no quota, only used).
    pub total: Option<f64>,
    /// Amount used (always present).
    pub used: f64,
    /// Amount remaining (computed if total+used known; None for PostgreSQL).
    pub remaining: Option<f64>,
    /// Display unit: "USD" | "CNY" | "req" | "GB" | "token" | etc.
    pub unit: String,
    /// UI color hint based on % remaining: "green" | "amber" | "red" | "ruby".
    pub level: Option<String>,
    /// Unix epoch seconds when quota resets (None for PostgreSQL).
    pub reset_at: Option<i64>,
}

// --- Token Usage types (Stage A) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageRecord {
    pub id: String,
    pub agent_type: String,
    pub model: String,
    pub provider_name: String,
    pub occurred_at: i64,
    pub recorded_at: i64,
    pub session_id: Option<String>,
    pub request_id: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub cache_read_input_tokens: i64,
    pub cache_creation_input_tokens: i64,
    pub reasoning_tokens: i64,
    pub prompt_cost: f64,
    pub completion_cost: f64,
    pub cache_read_cost: f64,
    pub cache_creation_cost: f64,
    pub reasoning_cost: f64,
    pub total_cost: f64,
    pub currency: String,
    pub pricing_version: Option<String>,
    pub usage_details: Option<String>,
    pub cost_details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyAgentModelUsage {
    pub date: String,
    pub agent_type: String,
    pub model: String,
    pub provider: String,
    pub request_count: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub total_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyModelUsage {
    pub date: String,
    pub model: String,
    pub provider: String,
    pub request_count: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub total_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingEntry {
    pub model: String,
    pub provider: String,
    pub input_price_per_1m: Option<f64>,
    pub output_price_per_1m: Option<f64>,
    pub cache_read_price_per_1m: Option<f64>,
    pub cache_creation_price_per_1m: Option<f64>,
    pub reasoning_price_per_1m: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCounts {
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_creation: i64,
    pub reasoning: i64,
}