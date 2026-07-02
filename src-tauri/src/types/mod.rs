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

/// QuotaSnapshot (Stage 4 + 7): per-preset shape reconciled to common denominator.
/// @see openspec/changes/v0.1-general-credentials/spec.md REQ-QUOTA-001~006
/// @see openspec/changes/v0.1-general-credentials/spec.md REQ-QUOTA-DISPLAY-001
/// Per-preset shapes:
/// - 3 LLM (OpenAI/DeepSeek): { total, used, remaining, unit='USD'|'token', level?, reset_at? }
/// - GitHub: { total, used, remaining, unit='req', level?, reset_at? }
/// - Anthropic: AppError::ProviderQuotaUnsupported (no QuotaSnapshot returned)
///
/// 2026-06-29 升级:新增 11 个字段对齐 token-monitor normalizeLimitProvider 输出
/// (windows / status / source / source_detail / account_* / region / balance / used_amount /
/// balance_usd / used_usd)。旧字段保留向后兼容;旧 JSON 反序列化时新字段走 serde 默认值。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuotaSnapshot {
    // === 旧字段(保留,向后兼容) ===
    /// Total quota limit (None for providers with no quota, only used).
    pub total: Option<f64>,
    /// Amount used (always present).
    pub used: f64,
    /// Amount remaining (computed if total+used known).
    pub remaining: Option<f64>,
    /// Display unit: "USD" | "CNY" | "req" | "GB" | "token" | etc.
    pub unit: String,
    /// UI color hint based on % remaining: "green" | "amber" | "red" | "ruby".
    pub level: Option<String>,
    /// Unix epoch seconds when quota resets.
    pub reset_at: Option<i64>,

    // === 新字段(对齐 token-monitor normalizeLimitProvider 输出) ===
    /// 多窗口进度数据(主入口;旧字段 balance/used 是 windows[0] 的快照)
    #[serde(default)]
    pub windows: Vec<LimitWindow>,
    /// 状态机
    #[serde(default = "default_limit_status")]
    pub status: LimitStatus,
    /// 数据源
    #[serde(default = "default_limit_source")]
    pub source: LimitSource,
    /// 源详情("app"/"cli"/"managed"/"unknown")
    #[serde(default)]
    pub source_detail: String,
    /// 账号标签
    #[serde(default)]
    pub account_label: Option<String>,
    /// 账号邮箱
    #[serde(default)]
    pub account_email: Option<String>,
    /// 区域
    #[serde(default)]
    pub region: Option<String>,
    /// 余额(原始货币)
    #[serde(default)]
    pub balance: Option<MoneyAmount>,
    /// 已用(原始货币)
    #[serde(default)]
    pub used_amount: Option<MoneyAmount>,
    /// 余额(换算 USD)
    #[serde(default)]
    pub balance_usd: Option<f64>,
    /// 已用(换算 USD)
    #[serde(default)]
    pub used_usd: Option<f64>,
}

impl QuotaSnapshot {
    /// 工厂方法:返回旧字段都填好但 windows=[] / status=Ok / source=Manual 的实例。
    /// 供 5 个 provider 临时兼容(它们仍填旧字段,新字段走默认值)。
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

/// Per-dimension USD cost breakdown for a single usage record.
/// Returned by `PricingService::calculate_token_usage_cost` and consumed
/// by `TokenUsageService::record_usage` to populate the 5 cost fields on
/// `TokenUsageRecord` plus the `cost_details` JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageCostBreakdown {
    pub input_cost: f64,
    pub output_cost: f64,
    pub cache_read_cost: f64,
    pub cache_creation_cost: f64,
    pub reasoning_cost: f64,
    pub total_cost: f64,
    pub currency: String,
    /// Set to `Some(model)` when the model is not in pricing.json.
    /// All numeric fields are 0 in that case.
    pub pricing_missing_for: Option<String>,
}

// --- TokenUsageService (Stage B) DTOs ---

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

// --- PeriodsSummary DTOs (token-monitor-alignment Part A #1) ---

/// 单个周期的窗口元数据(对齐 token-monitor periodWindows)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodWindow {
    /// 周期 key:today 是 "YYYY-MM-DD",month 是 "YYYY-MM"
    pub key: String,
    /// 周期结束时刻(ISO 8601 含时区,如 "2026-06-30T00:00:00+08:00")
    pub ends_at: String,
}

/// 一对周期窗口(today + month),对齐 token-monitor periodWindows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodWindowsPair {
    pub today: PeriodWindow,
    pub month: PeriodWindow,
}

/// 三周期同结构聚合(对齐 token-monitor PeriodSummary)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodsTriplet {
    pub today: UsageSummary,
    pub month: UsageSummary,
    pub all_time: UsageSummary,
}

/// 货币金额(对齐 token-monitor MoneyAmount):数值 + 货币代码
/// 2026-06-29 升级:新增 today_spend / month_spend / month_since_tracking
/// 三个可选字段,用于 DeepSeek 余额历史追踪(对齐 token-monitor todaySpend/monthSpend)。
/// 旧 JSON 反序列化时新字段走 serde 默认值(None),向后兼容。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MoneyAmount {
    pub amount: f64,
    pub currency: String,
    /// 今日已消耗(对齐 token-monitor todaySpend)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub today_spend: Option<f64>,
    /// 本月已消耗(对齐 token-monitor monthSpend)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month_spend: Option<f64>,
    /// 本月才开始追踪(月数据不完整)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month_since_tracking: Option<bool>,
}

impl MoneyAmount {
    /// 工厂方法:只填 amount + currency,新字段为 None
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

/// 单个 provider 的 limits 信息(对齐 token-monitor LimitProvider)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitProvider {
    /// provider 名称(对齐 quota_cache.provider_name)
    pub provider: String,
    /// 多窗口进度数据(可能为空数组,表示 NotConfigured 等)
    #[serde(default)]
    pub windows: Vec<LimitWindow>,
    /// 状态机
    #[serde(default = "default_limit_status")]
    pub status: LimitStatus,
    /// 数据源
    #[serde(default = "default_limit_source")]
    pub source: LimitSource,
    /// 源详情(如 "app"/"cli"/"managed"/"unknown")
    #[serde(default)]
    pub source_detail: String,
    /// 账号标签(可选)
    #[serde(default)]
    pub account_label: Option<String>,
    /// 账号邮箱(可选)
    #[serde(default)]
    pub account_email: Option<String>,
    /// 区域(可选)
    #[serde(default)]
    pub region: Option<String>,
    /// 余额(原始货币)
    #[serde(default)]
    pub balance: Option<MoneyAmount>,
    /// 已用(原始货币)
    #[serde(default)]
    pub used_amount: Option<MoneyAmount>,
    /// 余额(换算 USD)
    #[serde(default)]
    pub balance_usd: Option<f64>,
    /// 已用(换算 USD)
    #[serde(default)]
    pub used_usd: Option<f64>,
}

/// 全部 provider 的 limits 聚合(对齐 token-monitor LimitsSummary)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsSummary {
    pub providers: Vec<LimitProvider>,
    pub updated_at: i64,
}

/// 五元结构(对齐 token-monitor usage.js 主数据契约)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodsSummary {
    pub periods: PeriodsTriplet,
    pub period_windows: PeriodWindowsPair,
    /// client × model 二维聚合:agent_type → model → total_tokens
    #[serde(default)]
    pub client_models: std::collections::BTreeMap<String, std::collections::BTreeMap<String, i64>>,
    /// quota_cache 聚合的 limits;空时为 None
    #[serde(default)]
    pub limits: Option<LimitsSummary>,
}

// --- LimitWindow / LimitStatus / LimitSource (token-monitor-alignment Part B #6A) ---

/// 配额窗口类型(对齐 token-monitor LimitWindow.kind)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LimitWindowKind {
    /// 会话窗口(短时,如 GitHub hourly / opencode 5h session)
    Session,
    /// 周窗口(如 opencode 7d weekly)
    Weekly,
    /// 计费窗口(月度/订阅周期)
    Billing,
}

/// 配额状态机(对齐 token-monitor LimitStatus 8 种)
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LimitStatus {
    /// 正常,有数据
    #[default]
    Ok,
    /// 已禁用
    Disabled,
    /// 未配置(如 Anthropic 无 API,需要用户手动设置)
    NotConfigured,
    /// 401 未授权(API key 错误)
    Unauthorized,
    /// 429 限流
    RateLimited,
    /// 数据源限流(如 RPC 限流但 web 可用)
    SourceRateLimited,
    /// 数据源不可用(网络错误)
    Unavailable,
    /// 解析错误
    Error,
}

/// 配额数据源(对齐 token-monitor LimitSource)
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LimitSource {
    /// OAuth token(如 grok.com bearer)
    #[default]
    Oauth,
    /// CLI 配置文件(如 opencode.db)
    Cli,
    /// Web 抓取(如 grok.com gRPC-web)
    Web,
    /// RPC(如 Codex agent stdio)
    Rpc,
    /// 本地数据库
    Local,
    /// 官方 API
    Api,
    /// 用户手动设置
    Manual,
}

/// 单个配额窗口的进度数据(对齐 token-monitor LimitWindow)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitWindow {
    /// 窗口类型
    pub kind: LimitWindowKind,
    /// 显示标签(如 "Session (5h)" / "Weekly" / "Monthly" / "Hourly")
    pub label: String,
    /// 已用
    pub used: f64,
    /// 上限(None 表示无上限)
    pub limit: Option<f64>,
    /// 剩余
    pub remaining: Option<f64>,
    /// 已用百分比(0-100)
    pub used_percent: Option<f64>,
    /// 剩余百分比(0-100)
    pub remaining_percent: Option<f64>,
    /// 重置时间(ISO 8601)
    pub resets_at: Option<String>,
    /// 窗口时长(分钟)
    pub window_minutes: Option<i64>,
    /// 重置描述(如 "Resets in 2h 30m")
    pub reset_description: String,
    /// 是否显示进度条(某些状态不应显示)
    pub show_meter: bool,
}

/// 默认 status 用于 serde 反序列化旧 JSON
fn default_limit_status() -> LimitStatus {
    LimitStatus::Ok
}

/// 默认 source 用于 serde 反序列化旧 JSON
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
        // 旧 JSON(只有 6 个旧字段)反序列化时,新字段应走 serde 默认值
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
        // source defaults to Oauth (#[default] variant), not Manual
        assert_eq!(snap.source, LimitSource::Oauth);
    }
}