use serde::{Deserialize, Serialize};
use crate::error::AppError;

pub mod subscription;
pub use subscription::{CredentialStatus, QuotaTier, QuotaTierKind, SubscriptionQuota, TierStatus};

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












#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuotaSnapshot {
    
    
    pub total: Option<f64>,
    
    pub used: f64,
    
    pub remaining: Option<f64>,
    
    pub unit: String,
    
    pub level: Option<String>,
    
    pub reset_at: Option<i64>,

    
    
    #[serde(default)]
    pub windows: Vec<LimitWindow>,
    
    #[serde(default = "default_limit_status")]
    pub status: LimitStatus,
    
    #[serde(default = "default_limit_source")]
    pub source: LimitSource,
    
    #[serde(default)]
    pub source_detail: String,
    
    #[serde(default)]
    pub account_label: Option<String>,
    
    #[serde(default)]
    pub account_email: Option<String>,
    
    #[serde(default)]
    pub region: Option<String>,
    
    #[serde(default)]
    pub balance: Option<MoneyAmount>,
    
    #[serde(default)]
    pub used_amount: Option<MoneyAmount>,
    
    #[serde(default)]
    pub balance_usd: Option<f64>,
    
    #[serde(default)]
    pub used_usd: Option<f64>,
}

impl QuotaSnapshot {
    
    
    pub fn legacy(
        total: Option<f64>,
        used: f64,
        remaining: Option<f64>,
        unit: &str,
        level: Option<String>,
        reset_at: Option<i64>,
    ) -> Self {
        Self {
            total,
            used,
            remaining,
            unit: unit.to_string(),
            level,
            reset_at,
            ..Default::default()
        }
    }
}



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





#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageCostBreakdown {
    pub input_cost: f64,
    pub output_cost: f64,
    pub cache_read_cost: f64,
    pub cache_creation_cost: f64,
    pub reasoning_cost: f64,
    pub total_cost: f64,
    pub currency: String,
    
    
    pub pricing_missing_for: Option<String>,
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecordInput {
    pub agent_type: String,
    pub model: String,
    pub provider_name: String,
    pub occurred_at: i64,
    pub session_id: Option<String>,
    pub request_id: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_input_tokens: i64,
    pub cache_creation_input_tokens: i64,
    pub reasoning_tokens: i64,
    pub usage_details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageFilter {
    pub date_from: Option<i64>,
    pub date_to: Option<i64>,
    pub agent_type: Option<String>,
    pub model: Option<String>,
    pub provider_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummaryAgentPair {
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
pub struct DailySeries {
    pub date: String,
    pub request_count: i64,
    pub total_tokens: i64,
    pub total_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    pub total_tokens: i64,
    pub total_cost: f64,
    pub total_requests: i64,
    pub agent_pairs: Vec<UsageSummaryAgentPair>,
    pub daily_series: Vec<DailySeries>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub imported: u32,
    pub skipped: u32,
    pub errors: Vec<ImportError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportError {
    pub line: u32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecomputeResult {
    pub recomputed: u32,
    pub dates_refreshed: u32,
}




#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodWindow {
    
    pub key: String,
    
    pub ends_at: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodWindowsPair {
    pub today: PeriodWindow,
    pub month: PeriodWindow,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodsTriplet {
    pub today: UsageSummary,
    pub month: UsageSummary,
    pub all_time: UsageSummary,
}





#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MoneyAmount {
    pub amount: f64,
    pub currency: String,
    
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub today_spend: Option<f64>,
    
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month_spend: Option<f64>,
    
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month_since_tracking: Option<bool>,
}

impl MoneyAmount {
    
    pub fn new(amount: f64, currency: &str) -> Self {
        Self {
            amount,
            currency: currency.to_string(),
            today_spend: None,
            month_spend: None,
            month_since_tracking: None,
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitProvider {
    
    pub provider: String,
    
    #[serde(default)]
    pub windows: Vec<LimitWindow>,
    
    #[serde(default = "default_limit_status")]
    pub status: LimitStatus,
    
    #[serde(default = "default_limit_source")]
    pub source: LimitSource,
    
    #[serde(default)]
    pub source_detail: String,
    
    #[serde(default)]
    pub account_label: Option<String>,
    
    #[serde(default)]
    pub account_email: Option<String>,
    
    #[serde(default)]
    pub region: Option<String>,
    
    #[serde(default)]
    pub balance: Option<MoneyAmount>,
    
    #[serde(default)]
    pub used_amount: Option<MoneyAmount>,
    
    #[serde(default)]
    pub balance_usd: Option<f64>,
    
    #[serde(default)]
    pub used_usd: Option<f64>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsSummary {
    pub providers: Vec<LimitProvider>,
    pub updated_at: i64,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodsSummary {
    pub periods: PeriodsTriplet,
    pub period_windows: PeriodWindowsPair,
    
    #[serde(default)]
    pub client_models: std::collections::BTreeMap<String, std::collections::BTreeMap<String, i64>>,
    
    #[serde(default)]
    pub limits: Option<LimitsSummary>,
}




#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LimitWindowKind {
    
    Session,
    
    Weekly,
    
    Billing,
}


#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LimitStatus {
    
    #[default]
    Ok,
    
    Disabled,
    
    NotConfigured,
    
    Unauthorized,
    
    RateLimited,
    
    SourceRateLimited,
    
    Unavailable,
    
    Error,
}


#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LimitSource {
    
    #[default]
    Oauth,
    
    Cli,
    
    Web,
    
    Rpc,
    
    Local,
    
    Api,
    
    Manual,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitWindow {
    
    pub kind: LimitWindowKind,
    
    pub label: String,
    
    pub used: f64,
    
    pub limit: Option<f64>,
    
    pub remaining: Option<f64>,
    
    pub used_percent: Option<f64>,
    
    pub remaining_percent: Option<f64>,
    
    pub resets_at: Option<String>,
    
    pub window_minutes: Option<i64>,
    
    pub reset_description: String,
    
    pub show_meter: bool,
}


fn default_limit_status() -> LimitStatus {
    LimitStatus::Ok
}


fn default_limit_source() -> LimitSource {
    LimitSource::Manual
}

#[cfg(test)]
mod tests_quota_snapshot {
    use super::*;

    #[test]
    fn quota_snapshot_serializes_with_new_fields() {
        let snap = QuotaSnapshot {
            total: Some(100.0),
            used: 30.0,
            remaining: Some(70.0),
            unit: "USD".into(),
            level: Some("green".into()),
            reset_at: Some(1718000000),
            windows: vec![LimitWindow {
                kind: LimitWindowKind::Billing,
                label: "Monthly".into(),
                used: 30.0,
                limit: Some(100.0),
                remaining: Some(70.0),
                used_percent: Some(30.0),
                remaining_percent: Some(70.0),
                resets_at: Some("2026-07-01T00:00:00Z".into()),
                window_minutes: Some(43200),
                reset_description: "Resets in 2 days".into(),
                show_meter: true,
            }],
            status: LimitStatus::Ok,
            source: LimitSource::Api,
            source_detail: "app".into(),
            account_label: Some("Personal".into()),
            account_email: None,
            region: Some("us-east".into()),
            balance: Some(MoneyAmount { amount: 70.0, currency: "USD".into(), ..Default::default() }),
            used_amount: Some(MoneyAmount { amount: 30.0, currency: "USD".into(), ..Default::default() }),
            balance_usd: Some(70.0),
            used_usd: Some(30.0),
        };
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("\"windows\""));
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"source\":\"api\""));
        assert!(json.contains("\"balance_usd\":70.0"));

        let back: QuotaSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.windows.len(), 1);
        assert_eq!(back.status, LimitStatus::Ok);
        assert_eq!(back.source, LimitSource::Api);
    }

    #[test]
    fn quota_snapshot_deserializes_legacy_json_without_new_fields() {
        
        let legacy_json = r#"{
            "total": 50.0,
            "used": 20.0,
            "remaining": 30.0,
            "unit": "USD",
            "level": "green",
            "reset_at": 1718000000
        }"#;
        let snap: QuotaSnapshot = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(snap.total, Some(50.0));
        assert_eq!(snap.used, 20.0);
        assert!(snap.windows.is_empty());
        assert_eq!(snap.status, LimitStatus::Ok);
        assert_eq!(snap.source, LimitSource::Manual);
        assert_eq!(snap.source_detail, "");
        assert!(snap.balance.is_none());
        assert!(snap.used_usd.is_none());
    }

    #[test]
    fn legacy_factory_returns_compatible_snapshot() {
        let snap = QuotaSnapshot::legacy(
            Some(100.0),
            30.0,
            Some(70.0),
            "USD",
            Some("green".into()),
            Some(1718000000),
        );
        assert_eq!(snap.total, Some(100.0));
        assert_eq!(snap.used, 30.0);
        assert!(snap.windows.is_empty());
        assert_eq!(snap.status, LimitStatus::Ok);
        
        assert_eq!(snap.source, LimitSource::Oauth);
    }
}