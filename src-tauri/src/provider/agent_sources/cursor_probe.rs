//! Cursor probe 余额查询 source。
//!
//! 读 `~/.cursor/auth.json` 拿 sessionToken,probe `https://cursor.com/api/usage`
//! 拿多窗口用量(reqs used/limit + autoPercent + apiPercent + onDemand credits)。
//!
//! 对齐 token-monitor `cursorProbe.js` + `limitCollector.js::fetchCursorLimits`。
//! V0.1 简化:跳过 team credits / team pool 窗口;不自动 refresh token。

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;

use crate::provider::adapter::QuotaError;
use crate::provider::agent_source::AgentBalanceSource;
use crate::types::{LimitSource, LimitStatus, LimitWindow, LimitWindowKind, QuotaSnapshot};

const USAGE_URL: &str = "https://cursor.com/api/usage";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(12);

pub struct CursorProbeSource;

impl CursorProbeSource {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CursorProbeSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentBalanceSource for CursorProbeSource {
    fn agent_type(&self) -> &'static str {
        "cursor"
    }

    fn display_name(&self) -> &'static str {
        "Cursor"
    }

    fn is_available(&self) -> bool {
        cursor_auth_path().exists()
    }

    async fn fetch_balance(&self) -> Result<QuotaSnapshot, QuotaError> {
        let auth = match read_cursor_auth() {
            Ok(a) => a,
            Err(QuotaError::Unsupported) => return Ok(not_configured_snapshot()),
            Err(e) => return Ok(unavailable_snapshot(format!("read auth: {:?}", e))),
        };

        let usage = match probe_usage(&auth.session_token).await {
            Ok(u) => u,
            Err(QuotaError::Network(msg)) if msg == "401" => {
                return Ok(unauthorized_snapshot())
            }
            Err(e) => return Ok(unavailable_snapshot(format!("probe: {:?}", e))),
        };

        let windows = build_windows(&usage);

        // 用 auth.membership_type 优先,否则用 usage.membership_type
        let membership = auth.membership_type.as_deref().unwrap_or("").to_string();
        if membership.is_empty() {
            if let Some(m) = &usage.membership_type {
                let label = cursor_membership_label(m);
                if !label.is_empty() {
                    return Ok(final_snapshot(windows, auth.email, Some(label)));
                }
            }
            return Ok(final_snapshot(windows, auth.email, None));
        }
        let label = cursor_membership_label(&membership);
        Ok(final_snapshot(
            windows,
            auth.email,
            if label.is_empty() { None } else { Some(label) },
        ))
    }
}

// === auth 文件读取 ===

/// Windows: `%USERPROFILE%\.cursor\auth.json`;Unix: `~/.cursor/auth.json`。
fn cursor_auth_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".cursor").join("auth.json")
}

struct CursorAuth {
    session_token: String,
    #[allow(dead_code)]
    user_id: Option<String>,
    email: Option<String>,
    membership_type: Option<String>,
}

/// 读 `~/.cursor/auth.json`。文件不存在 → `Unsupported`(映射到 NotConfigured)。
fn read_cursor_auth() -> Result<CursorAuth, QuotaError> {
    let path = cursor_auth_path();
    let text = std::fs::read_to_string(&path).map_err(|_| QuotaError::Unsupported)?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| QuotaError::Parse(e.to_string()))?;

    // 优先 workosCursorSessionToken,其次 sessionToken
    let session_token = v
        .get("workosCursorSessionToken")
        .or_else(|| v.get("sessionToken"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    if session_token.is_empty() {
        return Err(QuotaError::Unsupported);
    }

    let user_id = v.get("userId").and_then(|s| s.as_str()).map(|s| s.to_string());
    let email = v
        .get("email")
        .and_then(|s| s.as_str())
        .map(|s| s.to_lowercase());
    let membership_type = v
        .get("membershipType")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    Ok(CursorAuth {
        session_token,
        user_id,
        email,
        membership_type,
    })
}

// === membership 标签 ===

/// 对齐 token-monitor `formatCursorMembership`:`pro+` / `pro_plus` → "Pro+" 等。
fn cursor_membership_label(membership_type: &str) -> String {
    let lower = membership_type.trim().to_lowercase();
    match lower.as_str() {
        "pro+" | "pro_plus" | "pro-plus" | "proplus" => "Pro+".to_string(),
        "pro" => "Pro".to_string(),
        "business" => "Business".to_string(),
        "free" => "Free".to_string(),
        "team" | "teams" => "Team".to_string(),
        "" => String::new(),
        _ => {
            // capitalize first letter
            let mut chars = lower.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}

// === usage probe ===

#[derive(Default, Debug, Clone)]
struct CursorUsage {
    requests_used: Option<f64>,
    requests_limit: Option<f64>,
    auto_percent: Option<f64>,
    api_percent: Option<f64>,
    plan_used_usd: Option<f64>,
    plan_limit_usd: Option<f64>,
    on_demand_used_usd: Option<f64>,
    on_demand_limit_usd: Option<f64>,
    billing_cycle_end: Option<String>,
    membership_type: Option<String>,
}

/// `GET https://cursor.com/api/usage` 带 `Cookie: WorkosCursorSessionToken=<token>`。
/// 401/403 → `QuotaError::Network("401")`(映射到 Unauthorized);非 200 → `Network`。
async fn probe_usage(session_token: &str) -> Result<CursorUsage, QuotaError> {
    let client = reqwest::Client::new();
    let resp = client
        .get(USAGE_URL)
        .header(
            "Cookie",
            format!("WorkosCursorSessionToken={}", session_token),
        )
        .header("Accept", "*/*")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        )
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| QuotaError::Network(e.to_string()))?;

    let status = resp.status().as_u16();
    if status == 401 || status == 403 {
        return Err(QuotaError::Network("401".to_string()));
    }
    if !resp.status().is_success() {
        return Err(QuotaError::Network(format!("HTTP {}", resp.status())));
    }

    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| QuotaError::Parse(e.to_string()))?;
    Ok(parse_cursor_usage(&v))
}

/// 解析 Cursor usage JSON。cents → USD(对齐 token-monitor `centsToUsd = value/100`)。
fn parse_cursor_usage(v: &serde_json::Value) -> CursorUsage {
    let individual = v.get("individualUsage").unwrap_or(v);
    let plan = individual
        .get("plan")
        .unwrap_or(&serde_json::Value::Null);
    let on_demand = individual
        .get("onDemand")
        .unwrap_or(&serde_json::Value::Null);

    CursorUsage {
        requests_used: num(v, "requestsUsed"),
        requests_limit: num(v, "requestsLimit"),
        auto_percent: num(plan, "autoPercentUsed"),
        api_percent: num(plan, "apiPercentUsed"),
        plan_used_usd: cents_to_usd(num(plan, "used")),
        plan_limit_usd: cents_to_usd(num(plan, "limit")),
        on_demand_used_usd: cents_to_usd(num(on_demand, "used")),
        on_demand_limit_usd: cents_to_usd(num(on_demand, "limit")),
        billing_cycle_end: v
            .get("billingCycleEnd")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string()),
        membership_type: v
            .get("membershipType")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string()),
    }
}

fn num(v: &serde_json::Value, key: &str) -> Option<f64> {
    v.get(key).and_then(|p| p.as_f64())
}

fn cents_to_usd(cents: Option<f64>) -> Option<f64> {
    cents.map(|c| (c / 100.0).round())
}

// === 多窗口构造 ===

fn build_windows(usage: &CursorUsage) -> Vec<LimitWindow> {
    let mut windows = Vec::new();

    // Total 窗口(reqs used / limit)
    if usage.requests_used.is_some() || usage.requests_limit.is_some() {
        let used = usage.requests_used.unwrap_or(0.0);
        let limit = usage.requests_limit;
        let used_percent = limit.and_then(|l| {
            if l > 0.0 {
                Some((used / l) * 100.0)
            } else {
                None
            }
        });
        windows.push(LimitWindow {
            kind: LimitWindowKind::Billing,
            label: "Total".to_string(),
            used,
            limit,
            remaining: limit.map(|l| (l - used).max(0.0)),
            used_percent,
            remaining_percent: used_percent.map(|p| (100.0 - p).max(0.0)),
            resets_at: usage.billing_cycle_end.clone(),
            window_minutes: None,
            reset_description: String::new(),
            show_meter: true,
        });
    }

    // Plan 窗口(plan used/limit USD)
    if usage.plan_used_usd.is_some() || usage.plan_limit_usd.is_some() {
        let used = usage.plan_used_usd.unwrap_or(0.0);
        let limit = usage.plan_limit_usd;
        let used_percent = limit.and_then(|l| {
            if l > 0.0 {
                Some((used / l) * 100.0)
            } else {
                None
            }
        });
        windows.push(LimitWindow {
            kind: LimitWindowKind::Billing,
            label: "Plan".to_string(),
            used,
            limit,
            remaining: limit.map(|l| (l - used).max(0.0)),
            used_percent,
            remaining_percent: used_percent.map(|p| (100.0 - p).max(0.0)),
            resets_at: usage.billing_cycle_end.clone(),
            window_minutes: None,
            reset_description: String::new(),
            show_meter: true,
        });
    }

    // Auto 窗口(autoPercent)
    if let Some(auto_percent) = usage.auto_percent {
        windows.push(LimitWindow {
            kind: LimitWindowKind::Billing,
            label: "Auto".to_string(),
            used: 0.0,
            limit: None,
            remaining: None,
            used_percent: Some(auto_percent),
            remaining_percent: Some((100.0 - auto_percent).max(0.0)),
            resets_at: usage.billing_cycle_end.clone(),
            window_minutes: None,
            reset_description: String::new(),
            show_meter: true,
        });
    }

    // API 窗口(apiPercent)
    if let Some(api_percent) = usage.api_percent {
        windows.push(LimitWindow {
            kind: LimitWindowKind::Billing,
            label: "API".to_string(),
            used: 0.0,
            limit: None,
            remaining: None,
            used_percent: Some(api_percent),
            remaining_percent: Some((100.0 - api_percent).max(0.0)),
            resets_at: usage.billing_cycle_end.clone(),
            window_minutes: None,
            reset_description: String::new(),
            show_meter: true,
        });
    }

    // Credits 窗口(onDemand used/limit USD)
    if usage.on_demand_used_usd.is_some() || usage.on_demand_limit_usd.is_some() {
        let used = usage.on_demand_used_usd.unwrap_or(0.0);
        let limit = usage.on_demand_limit_usd;
        let used_percent = limit.and_then(|l| {
            if l > 0.0 {
                Some((used / l) * 100.0)
            } else {
                None
            }
        });
        windows.push(LimitWindow {
            kind: LimitWindowKind::Billing,
            label: "Credits".to_string(),
            used,
            limit,
            remaining: limit.map(|l| (l - used).max(0.0)),
            used_percent,
            remaining_percent: used_percent.map(|p| (100.0 - p).max(0.0)),
            resets_at: None,
            window_minutes: None,
            reset_description: String::new(),
            show_meter: true,
        });
    }

    windows
}

// === QuotaSnapshot 工厂 ===

fn final_snapshot(
    windows: Vec<LimitWindow>,
    email: Option<String>,
    label: Option<String>,
) -> QuotaSnapshot {
    QuotaSnapshot {
        total: None,
        used: 0.0,
        remaining: None,
        unit: String::new(),
        level: None,
        reset_at: None,
        windows,
        status: LimitStatus::Ok,
        source: LimitSource::Web,
        source_detail: "app".to_string(),
        account_label: label,
        account_email: email,
        region: None,
        balance: None,
        used_amount: None,
        balance_usd: None,
        used_usd: None,
    }
}

fn not_configured_snapshot() -> QuotaSnapshot {
    QuotaSnapshot {
        total: None,
        used: 0.0,
        remaining: None,
        unit: String::new(),
        level: None,
        reset_at: None,
        windows: Vec::new(),
        status: LimitStatus::NotConfigured,
        source: LimitSource::Web,
        source_detail: "unknown".to_string(),
        account_label: None,
        account_email: None,
        region: None,
        balance: None,
        used_amount: None,
        balance_usd: None,
        used_usd: None,
    }
}

fn unauthorized_snapshot() -> QuotaSnapshot {
    QuotaSnapshot {
        total: None,
        used: 0.0,
        remaining: None,
        unit: String::new(),
        level: None,
        reset_at: None,
        windows: Vec::new(),
        status: LimitStatus::Unauthorized,
        source: LimitSource::Web,
        source_detail: "token expired".to_string(),
        account_label: None,
        account_email: None,
        region: None,
        balance: None,
        used_amount: None,
        balance_usd: None,
        used_usd: None,
    }
}

fn unavailable_snapshot(detail: String) -> QuotaSnapshot {
    QuotaSnapshot {
        total: None,
        used: 0.0,
        remaining: None,
        unit: String::new(),
        level: None,
        reset_at: None,
        windows: Vec::new(),
        status: LimitStatus::Unavailable,
        source: LimitSource::Web,
        source_detail: detail,
        account_label: None,
        account_email: None,
        region: None,
        balance: None,
        used_amount: None,
        balance_usd: None,
        used_usd: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_auth_path_returns_home_cursor_auth_json() {
        let path = cursor_auth_path();
        let s = path.to_string_lossy();
        assert!(s.contains(".cursor"), "path should contain .cursor: {}", s);
        assert!(
            s.contains("auth.json"),
            "path should contain auth.json: {}",
            s
        );
    }

    #[test]
    fn cursor_membership_label_maps_aliases() {
        assert_eq!(cursor_membership_label("pro+"), "Pro+");
        assert_eq!(cursor_membership_label("pro_plus"), "Pro+");
        assert_eq!(cursor_membership_label("pro-plus"), "Pro+");
        assert_eq!(cursor_membership_label("pro"), "Pro");
        assert_eq!(cursor_membership_label("business"), "Business");
        assert_eq!(cursor_membership_label("free"), "Free");
        assert_eq!(cursor_membership_label("team"), "Team");
        assert_eq!(cursor_membership_label("teams"), "Team");
        assert_eq!(cursor_membership_label(""), "");
        assert_eq!(cursor_membership_label("enterprise"), "Enterprise");
    }

    #[test]
    fn parse_cursor_usage_handles_full_payload() {
        let v: serde_json::Value = serde_json::json!({
            "individualUsage": {
                "plan": {
                    "used": 5000,
                    "limit": 10000,
                    "autoPercentUsed": 30.0,
                    "apiPercentUsed": 50.0
                },
                "onDemand": {
                    "used": 2000,
                    "limit": 5000
                }
            },
            "billingCycleEnd": "2026-07-01T00:00:00Z",
            "membershipType": "pro",
            "requestsUsed": 100,
            "requestsLimit": 500
        });
        let usage = parse_cursor_usage(&v);
        assert_eq!(usage.requests_used, Some(100.0));
        assert_eq!(usage.requests_limit, Some(500.0));
        assert_eq!(usage.auto_percent, Some(30.0));
        assert_eq!(usage.api_percent, Some(50.0));
        assert_eq!(usage.plan_used_usd, Some(50.0)); // 5000 cents → 50 USD
        assert_eq!(usage.plan_limit_usd, Some(100.0));
        assert_eq!(usage.on_demand_used_usd, Some(20.0));
        assert_eq!(usage.on_demand_limit_usd, Some(50.0));
        assert_eq!(
            usage.billing_cycle_end.as_deref(),
            Some("2026-07-01T00:00:00Z")
        );
        assert_eq!(usage.membership_type.as_deref(), Some("pro"));
    }

    #[test]
    fn parse_cursor_usage_handles_minimal_payload() {
        let v: serde_json::Value = serde_json::json!({});
        let usage = parse_cursor_usage(&v);
        assert!(usage.requests_used.is_none());
        assert!(usage.auto_percent.is_none());
        assert!(usage.plan_used_usd.is_none());
        assert!(usage.billing_cycle_end.is_none());
    }

    #[test]
    fn read_cursor_auth_parses_session_token_and_email() {
        // 直接验证 JSON 解析逻辑(无法改 home_dir,所以测 parse 而非 read_cursor_auth)
        let json = r#"{
            "sessionToken": "user-uuid%3A%3Asession-token",
            "userId": "user-uuid",
            "email": "USER@EXAMPLE.COM",
            "membershipType": "pro"
        }"#;
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let session_token = v
            .get("sessionToken")
            .and_then(|s| s.as_str())
            .unwrap_or("");
        assert_eq!(session_token, "user-uuid%3A%3Asession-token");
        let email = v
            .get("email")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_lowercase();
        assert_eq!(email, "user@example.com");
        let membership = v
            .get("membershipType")
            .and_then(|s| s.as_str())
            .unwrap_or("");
        assert_eq!(cursor_membership_label(membership), "Pro");
    }

    #[test]
    fn read_cursor_auth_prefers_workos_cursor_session_token() {
        let json = r#"{
            "workosCursorSessionToken": "workos-token",
            "sessionToken": "fallback-token"
        }"#;
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let token = v
            .get("workosCursorSessionToken")
            .or_else(|| v.get("sessionToken"))
            .and_then(|s| s.as_str())
            .unwrap_or("");
        assert_eq!(token, "workos-token");
    }

    #[test]
    fn build_windows_constructs_all_five_when_full() {
        let usage = CursorUsage {
            requests_used: Some(100.0),
            requests_limit: Some(500.0),
            auto_percent: Some(30.0),
            api_percent: Some(50.0),
            plan_used_usd: Some(50.0),
            plan_limit_usd: Some(100.0),
            on_demand_used_usd: Some(20.0),
            on_demand_limit_usd: Some(50.0),
            billing_cycle_end: Some("2026-07-01T00:00:00Z".to_string()),
            membership_type: Some("pro".to_string()),
        };
        let windows = build_windows(&usage);
        assert_eq!(windows.len(), 5);
        assert_eq!(windows[0].label, "Total");
        assert_eq!(windows[0].used, 100.0);
        assert_eq!(windows[0].limit, Some(500.0));
        assert_eq!(windows[0].used_percent, Some(20.0));
        assert_eq!(windows[1].label, "Plan");
        assert_eq!(windows[2].label, "Auto");
        assert_eq!(windows[3].label, "API");
        assert_eq!(windows[4].label, "Credits");
        assert_eq!(windows[4].resets_at, None); // Credits 窗口不带 resets_at
    }

    #[test]
    fn build_windows_skips_absent_windows() {
        let usage = CursorUsage {
            requests_used: Some(100.0),
            requests_limit: Some(500.0),
            auto_percent: None,
            api_percent: None,
            plan_used_usd: None,
            plan_limit_usd: None,
            on_demand_used_usd: None,
            on_demand_limit_usd: None,
            billing_cycle_end: None,
            membership_type: None,
        };
        let windows = build_windows(&usage);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].label, "Total");
    }

    #[test]
    fn fetch_balance_returns_ok_result_regardless_of_env() {
        // 环境依赖测试:~/.cursor/auth.json 可能不存在(返回 NotConfigured)
        // 也可能存在(返回 Ok/Unauthorized/Unavailable)。只断言不 panic + Ok(Result)。
        let source = CursorProbeSource::new();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(source.fetch_balance());
        assert!(result.is_ok());
    }

    #[test]
    fn not_configured_snapshot_has_correct_status() {
        let snap = not_configured_snapshot();
        assert_eq!(snap.status, LimitStatus::NotConfigured);
        assert!(snap.windows.is_empty());
        assert_eq!(snap.source, LimitSource::Web);
    }

    #[test]
    fn unauthorized_snapshot_has_correct_status() {
        let snap = unauthorized_snapshot();
        assert_eq!(snap.status, LimitStatus::Unauthorized);
        assert_eq!(snap.source_detail, "token expired");
    }

    #[test]
    fn unavailable_snapshot_has_correct_status() {
        let snap = unavailable_snapshot("network error".to_string());
        assert_eq!(snap.status, LimitStatus::Unavailable);
        assert_eq!(snap.source_detail, "network error");
    }

    #[test]
    fn final_snapshot_sets_app_source_detail() {
        let snap = final_snapshot(
            Vec::new(),
            Some("user@example.com".to_string()),
            Some("Pro".to_string()),
        );
        assert_eq!(snap.status, LimitStatus::Ok);
        assert_eq!(snap.source_detail, "app");
        assert_eq!(snap.account_email.as_deref(), Some("user@example.com"));
        assert_eq!(snap.account_label.as_deref(), Some("Pro"));
    }

    #[test]
    fn cents_to_usd_rounds_correctly() {
        assert_eq!(cents_to_usd(Some(5000.0)), Some(50.0));
        assert_eq!(cents_to_usd(Some(250.0)), Some(3.0)); // 2.5 rounds to 3 (round half up)
        assert_eq!(cents_to_usd(None), None);
    }
}
