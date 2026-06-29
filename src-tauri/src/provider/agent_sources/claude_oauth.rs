//! Claude OAuth 余额查询 source。
//!
//! Task 3 实现:读 `~/.claude/.credentials.json`(支持 Windows WSL distros 发现)
//! → 调 `https://api.anthropic.com/api/oauth/usage` 拿 5h session + 7d weekly 窗口。
//!
//! 参考:token-monitor `limitCollector.js`(
//!   claudeCredentialPath / normalizeExpiresAt / listWslDistros /
//!   wslClaudeCredentialPaths / rankClaudeCredentialFiles /
//!   extractClaudeOauth / claudeCredentialsFromOauth / readClaudeCredentials /
//!   claudePlanLabelFromParts / planLabelFromParts / claudeRateLimitTierLabel /
//!   callClaudeUsage / mapClaudeUsageToProvider
//! )。

use async_trait::async_trait;
use chrono::Utc;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::provider::adapter::QuotaError;
use crate::provider::agent_source::AgentBalanceSource;
use crate::types::{LimitSource, LimitStatus, LimitWindow, LimitWindowKind, QuotaSnapshot};

const CLAUDE_USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const ANTHROPIC_BETA_HEADER: &str = "oauth-2025-04-20";
const KEYPILOT_USER_AGENT: &str = "keypilot/0.1";
const FETCH_TIMEOUT_SECS: u64 = 12;
/// token 在 5 分钟内即将过期 → 视为已过期(V0.1 不实现 refresh)
const TOKEN_EXPIRY_LEEWAY_MS: i64 = 5 * 60 * 1000;
/// 秒/毫秒阈值:对齐 token-monitor `normalizeExpiresAt`
const EXPIRES_AT_MS_THRESHOLD: i64 = 20_000_000_000;

pub struct ClaudeOAuthSource;

impl ClaudeOAuthSource {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClaudeOAuthSource {
    fn default() -> Self {
        Self::new()
    }
}

/// 从 `~/.claude/.credentials.json` 解析出的 OAuth 凭据。
struct ClaudeCredentials {
    access_token: String,
    #[allow(dead_code)]
    refresh_token: Option<String>,
    /// epoch ms(已 normalize)
    expires_at: Option<i64>,
    subscription_type: Option<String>,
    rate_limit_tier: Option<String>,
    #[allow(dead_code)]
    identity_label: String,
}

#[async_trait]
impl AgentBalanceSource for ClaudeOAuthSource {
    fn agent_type(&self) -> &'static str {
        "claude"
    }

    fn display_name(&self) -> &'static str {
        "Claude Code"
    }

    fn is_available(&self) -> bool {
        // 检查任一候选凭据文件是否存在(不读内容)
        credential_paths().iter().any(|(p, _)| p.exists())
    }

    async fn fetch_balance(&self) -> Result<QuotaSnapshot, QuotaError> {
        let credentials = match read_credentials().await {
            Ok(c) => c,
            Err(_) => return Ok(not_configured_snapshot()),
        };

        // 检查 token 是否过期(< now + 5min);V0.1 不尝试 refresh
        let now_ms = Utc::now().timestamp_millis();
        if let Some(expires_at) = credentials.expires_at {
            if expires_at < now_ms + TOKEN_EXPIRY_LEEWAY_MS {
                return Ok(unauthorized_snapshot());
            }
        }

        // 调 Anthropic usage API
        let client = reqwest::Client::new();
        let resp = client
            .get(CLAUDE_USAGE_URL)
            .header("Authorization", format!("Bearer {}", credentials.access_token))
            .header("anthropic-beta", ANTHROPIC_BETA_HEADER)
            .header("user-agent", KEYPILOT_USER_AGENT)
            .timeout(std::time::Duration::from_secs(FETCH_TIMEOUT_SECS))
            .send()
            .await
            .map_err(|e| QuotaError::Network(e.to_string()))?;

        match resp.status().as_u16() {
            401 => return Ok(unauthorized_snapshot()),
            429 => return Ok(source_rate_limited_snapshot()),
            s if !(200..300).contains(&s) => {
                return Ok(unavailable_snapshot(format!("HTTP {}", s)));
            }
            _ => {}
        }

        let usage: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| QuotaError::Parse(e.to_string()))?;

        // 解析 five_hour 窗口 → Session;seven_day 窗口 → Weekly
        let mut windows = Vec::new();
        if let Some(session) = usage.get("five_hour").or_else(|| usage.get("fiveHour")) {
            if let Some(window) = build_window(session, LimitWindowKind::Session, "Session (5h)", 300) {
                windows.push(window);
            }
        }
        if let Some(weekly) = usage.get("seven_day").or_else(|| usage.get("sevenDay")) {
            if let Some(window) = build_window(weekly, LimitWindowKind::Weekly, "Weekly (7d)", 10080) {
                windows.push(window);
            }
        }

        let plan_label = claude_plan_label(
            credentials.subscription_type.as_deref().unwrap_or(""),
            credentials.rate_limit_tier.as_deref().unwrap_or(""),
        );

        Ok(QuotaSnapshot {
            total: None,
            used: 0.0,
            remaining: None,
            unit: String::new(),
            level: None,
            reset_at: None,
            windows,
            status: LimitStatus::Ok,
            source: LimitSource::Oauth,
            source_detail: "app".to_string(),
            account_label: if plan_label.is_empty() {
                None
            } else {
                Some(plan_label)
            },
            account_email: None,
            region: None,
            balance: None,
            used_amount: None,
            balance_usd: None,
            used_usd: None,
        })
    }
}

// === 凭据路径发现 ===

/// 返回 Claude 凭据候选路径 + identity 标签。
///
/// - Windows: native `%USERPROFILE%\.claude\.credentials.json` + WSL distros 扫描
/// - Unix: `$CLAUDE_CONFIG_DIR/.credentials.json` 或 `~/.claude/.credentials.json`
///
/// WSL 扫描仅在 Windows 且未设置 `CLAUDE_CONFIG_DIR` 时进行(对齐 token-monitor)。
fn credential_paths() -> Vec<(PathBuf, String)> {
    let mut out: Vec<(PathBuf, String)> = Vec::new();

    let env_config = std::env::var("CLAUDE_CONFIG_DIR")
        .ok()
        .filter(|s| !s.is_empty());

    if let Some(cfg_dir) = env_config.as_deref() {
        out.push((
            PathBuf::from(cfg_dir).join(".credentials.json"),
            "CLAUDE_CONFIG_DIR/.credentials.json".to_string(),
        ));
    } else if let Some(home) = dirs::home_dir() {
        out.push((
            home.join(".claude").join(".credentials.json"),
            "path:~/.claude/.credentials.json".to_string(),
        ));
    }

    // Windows: 扫 WSL distros(仅当未设置 CLAUDE_CONFIG_DIR 时)
    #[cfg(windows)]
    if env_config.is_none() {
        for (path, identity) in wsl_claude_paths() {
            out.push((path, identity));
        }
    }

    out
}

/// 扫 `\\wsl$\` 列出 distros,对每个 distro 扫 `\home\<user>\.claude\.credentials.json`。
/// 不可访问时降级到空 vec,不 panic。对齐 token-monitor `wslClaudeCredentialPaths`。
#[cfg(windows)]
fn wsl_claude_paths() -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let wsl_root = PathBuf::from(r"\\wsl$");
    let distros = match std::fs::read_dir(&wsl_root) {
        Ok(rd) => rd,
        Err(_) => return out,
    };
    for entry in distros.flatten() {
        let distro_name = match entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => continue,
        };
        if distro_name.is_empty() || distro_name.starts_with('.') || distro_name.contains('$') {
            continue;
        }
        let home_root = wsl_root.join(&distro_name).join("home");
        let users = match std::fs::read_dir(&home_root) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for user_entry in users.flatten() {
            let user_name = match user_entry.file_name().into_string() {
                Ok(s) => s,
                Err(_) => continue,
            };
            if user_name.is_empty() {
                continue;
            }
            let creds_path = home_root
                .join(&user_name)
                .join(".claude")
                .join(".credentials.json");
            let identity = format!(r"wsl:{}\home\{}", distro_name, user_name);
            out.push((creds_path, identity));
        }
    }
    out
}

/// 对每个路径 stat mtime,过滤不存在的,按 mtime 降序排序(最新在前)。
/// 对齐 token-monitor `rankClaudeCredentialFiles`。
fn rank_by_mtime(paths: Vec<(PathBuf, String)>) -> Vec<(PathBuf, String, SystemTime)> {
    let mut stamped: Vec<(PathBuf, String, SystemTime)> = Vec::new();
    for (path, identity) in paths {
        let metadata = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let mtime = match metadata.modified() {
            Ok(t) => t,
            Err(_) => continue,
        };
        stamped.push((path, identity, mtime));
    }
    stamped.sort_by(|a, b| b.2.cmp(&a.2));
    stamped
}

// === 凭据读取与解析 ===

/// 异步读凭据。遍历 ranked paths,读第一个能成功解析的。
/// 支持 `claudeAiOauth` 包裹格式和 root 格式。
/// 全部路径都失败 → `Err(QuotaError::Unsupported)`(表示 NotConfigured)。
async fn read_credentials() -> Result<ClaudeCredentials, QuotaError> {
    let candidates = credential_paths();
    let ranked = rank_by_mtime(candidates);

    for (path, identity, _mtime) in ranked {
        // tokio::fs::read_to_string — async,不阻塞 runtime;只读不写
        let text = match tokio::fs::read_to_string(&path).await {
            Ok(t) => t,
            Err(_) => continue,
        };
        let v: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // 支持 claudeAiOauth 包裹 + root 格式(对齐 extractClaudeOauth)
        let oauth = v
            .get("claudeAiOauth")
            .or_else(|| v.get("oauth"))
            .unwrap_or(&v);

        let access_token = match oauth.get("accessToken").and_then(|s| s.as_str()) {
            Some(t) if !t.is_empty() => t.to_string(),
            _ => continue, // 此文件无 accessToken,试下一个
        };

        let refresh_token = oauth
            .get("refreshToken")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
        let expires_at = oauth.get("expiresAt").and_then(parse_expires_at);
        let subscription_type = oauth
            .get("subscriptionType")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
        let rate_limit_tier = oauth
            .get("rateLimitTier")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());

        return Ok(ClaudeCredentials {
            access_token,
            refresh_token,
            expires_at,
            subscription_type,
            rate_limit_tier,
            identity_label: identity,
        });
    }

    Err(QuotaError::Unsupported)
}

/// 解析 `expiresAt` 字段:数字(秒或毫秒)或 ISO 8601 字符串。
/// 对齐 token-monitor `normalizeExpiresAt`:若 `< 20_000_000_000` 视为秒,× 1000。
fn parse_expires_at(val: &serde_json::Value) -> Option<i64> {
    if let Some(n) = val.as_i64() {
        return Some(normalize_expires_at(n));
    }
    if let Some(n) = val.as_f64() {
        return Some(normalize_expires_at(n as i64));
    }
    if let Some(s) = val.as_str() {
        if let Ok(n) = s.parse::<i64>() {
            return Some(normalize_expires_at(n));
        }
        if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(s) {
            return Some(parsed.timestamp_millis());
        }
    }
    None
}

fn normalize_expires_at(value: i64) -> i64 {
    if value > EXPIRES_AT_MS_THRESHOLD {
        value
    } else {
        value.saturating_mul(1000)
    }
}

// === Plan 标签 ===

/// 对齐 token-monitor `claudePlanLabelFromParts` + `planLabelFromParts` +
/// `claudeRateLimitTierLabel`:把 subscriptionType + rateLimitTier 转 "Pro" / "Max 5x" 等。
fn claude_plan_label(subscription_type: &str, rate_limit_tier: &str) -> String {
    let sub = clean_plan_text(subscription_type);
    let tier = clean_plan_text(rate_limit_tier);
    let sub_label = match sub.as_str() {
        "free" => "Free".to_string(),
        "plus" => "Plus".to_string(),
        "pro" => "Pro".to_string(),
        "max" => "Max".to_string(),
        "team" | "teams" => "Team".to_string(),
        "enterprise" => "Enterprise".to_string(),
        "ultra" => "Ultra".to_string(),
        "" => String::new(),
        other => capitalize(other),
    };
    if sub_label == "Max" {
        let tier_lower = tier.to_lowercase();
        if tier_lower == "max 5x" || tier_lower.contains("5x") {
            return "Max 5x".to_string();
        }
        if tier_lower == "max 20x" || tier_lower.contains("20x") {
            return "Max 20x".to_string();
        }
    }
    if sub_label.is_empty() && !tier.is_empty() {
        return capitalize(&tier);
    }
    sub_label
}

/// 对齐 token-monitor `cleanPlanText`:剥离 "claude" / "chatgpt" / "openai" 前缀,
/// 替换 `_` / `-` 为空格,合并空白,转小写。
fn clean_plan_text(text: &str) -> String {
    let raw = text.trim();
    if raw.is_empty() || raw.contains('@') {
        return String::new();
    }
    let prefixes = ["claude", "chatgpt", "openai"];
    let mut clean = raw.to_lowercase();
    loop {
        let mut found = false;
        for p in prefixes {
            for sep in [' ', '_', '-'] {
                let pattern = format!("{}{}", p, sep);
                if clean.starts_with(&pattern) {
                    clean = clean[pattern.len()..].to_string();
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        if !found {
            break;
        }
    }
    clean = clean.replace('_', " ").replace('-', " ");
    clean.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

// === Window 构造 ===

/// 从 Claude usage API 的窗口数据构造 LimitWindow。
/// Claude usage API 只返回 `usedPercent`,没有 used/limit/remaining 数值,
/// 所以 `used = 0.0` / `limit = None` / `remaining = None`。
fn build_window(
    v: &serde_json::Value,
    kind: LimitWindowKind,
    label: &str,
    window_minutes: i64,
) -> Option<LimitWindow> {
    let used_percent = v
        .get("usedPercent")
        .or_else(|| v.get("used_percent"))
        .or_else(|| v.get("utilization"))
        .or_else(|| v.get("percent"))
        .and_then(|p| p.as_f64());
    let resets_at = v
        .get("resets_at")
        .or_else(|| v.get("resetsAt"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    Some(LimitWindow {
        kind,
        label: label.to_string(),
        used: 0.0,
        limit: None,
        remaining: None,
        used_percent,
        remaining_percent: used_percent.map(|p| (100.0 - p).max(0.0)),
        resets_at,
        window_minutes: Some(window_minutes),
        reset_description: String::new(),
        show_meter: true,
    })
}

// === QuotaSnapshot 工厂 ===

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
        source: LimitSource::Oauth,
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
        source: LimitSource::Oauth,
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

fn source_rate_limited_snapshot() -> QuotaSnapshot {
    QuotaSnapshot {
        total: None,
        used: 0.0,
        remaining: None,
        unit: String::new(),
        level: None,
        reset_at: None,
        windows: Vec::new(),
        status: LimitStatus::SourceRateLimited,
        source: LimitSource::Oauth,
        source_detail: "rate limited".to_string(),
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
        source: LimitSource::Oauth,
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
    fn claude_plan_label_maps_aliases() {
        assert_eq!(claude_plan_label("free", ""), "Free");
        assert_eq!(claude_plan_label("plus", ""), "Plus");
        assert_eq!(claude_plan_label("pro", ""), "Pro");
        assert_eq!(claude_plan_label("max", ""), "Max");
        assert_eq!(claude_plan_label("max", "Max 5x"), "Max 5x");
        assert_eq!(claude_plan_label("max", "Max 20x"), "Max 20x");
        assert_eq!(claude_plan_label("enterprise", ""), "Enterprise");
        assert_eq!(claude_plan_label("team", ""), "Team");
        assert_eq!(claude_plan_label("teams", ""), "Team");
        assert_eq!(claude_plan_label("", "Pro"), "Pro"); // tier fallback
        assert_eq!(claude_plan_label("claude_pro", ""), "Pro"); // prefix strip
    }

    #[test]
    fn claude_plan_label_handles_max_tier_variants() {
        // tier 含 "5x" / "20x" 子串也应匹配
        assert_eq!(claude_plan_label("max", "Max 5x tier"), "Max 5x");
        assert_eq!(claude_plan_label("max", "max 20x"), "Max 20x");
        // sub != "max" 时 tier 不接管
        assert_eq!(claude_plan_label("pro", "Max 5x"), "Pro");
    }

    #[test]
    fn credential_paths_returns_at_least_native_path() {
        let paths = credential_paths();
        assert!(!paths.is_empty());
    }

    #[test]
    fn read_credentials_parses_claude_ai_oauth_wrapper() {
        // 验证 claudeAiOauth 包裹格式的 JSON 解析路径
        let json = r#"{
            "claudeAiOauth": {
                "accessToken": "tok_abc",
                "refreshToken": "refresh_xyz",
                "expiresAt": 1800000000000,
                "subscriptionType": "pro",
                "rateLimitTier": ""
            }
        }"#;
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let oauth = v.get("claudeAiOauth").unwrap();
        assert_eq!(oauth["accessToken"].as_str().unwrap(), "tok_abc");
        assert_eq!(oauth["expiresAt"].as_i64().unwrap(), 1_800_000_000_000);
    }

    #[test]
    fn read_credentials_parses_root_format() {
        // 验证 root 格式(无 claudeAiOauth 包裹)的 JSON 解析路径
        let json = r#"{
            "accessToken": "tok_def",
            "refreshToken": "refresh_uvw",
            "expiresAt": 1800000000000,
            "subscriptionType": "max",
            "rateLimitTier": "Max 5x"
        }"#;
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        // root 格式下 v 本身就是 oauth 对象
        let oauth = v.get("claudeAiOauth").or_else(|| v.get("oauth")).unwrap_or(&v);
        assert_eq!(oauth["accessToken"].as_str().unwrap(), "tok_def");
        assert_eq!(oauth["subscriptionType"].as_str().unwrap(), "max");
    }

    #[test]
    fn clean_plan_text_strips_prefixes() {
        assert_eq!(clean_plan_text("claude_pro"), "pro");
        assert_eq!(clean_plan_text("claude-pro"), "pro");
        assert_eq!(clean_plan_text("claude pro"), "pro");
        assert_eq!(clean_plan_text("chatgpt_plus"), "plus");
        assert_eq!(clean_plan_text("openai-max"), "max");
        assert_eq!(clean_plan_text("Pro"), "pro");
        assert_eq!(clean_plan_text("MAX"), "max");
        assert_eq!(clean_plan_text("Ultra"), "ultra");
        assert_eq!(clean_plan_text(""), "");
        assert_eq!(clean_plan_text("user@example.com"), "");
    }

    #[test]
    fn normalize_expires_at_distinguishes_seconds_and_millis() {
        // 秒(< 20_000_000_000)→ × 1000
        assert_eq!(normalize_expires_at(1_700_000_000), 1_700_000_000_000);
        assert_eq!(normalize_expires_at(0), 0);
        // 毫秒(> 20_000_000_000)→ 原样
        assert_eq!(normalize_expires_at(1_800_000_000_000), 1_800_000_000_000);
        assert_eq!(
            normalize_expires_at(20_000_000_001),
            20_000_000_001
        );
    }

    #[test]
    fn parse_expires_at_handles_multiple_formats() {
        // 数字毫秒
        let v = serde_json::json!(1_800_000_000_000_i64);
        assert_eq!(parse_expires_at(&v), Some(1_800_000_000_000));
        // 数字秒
        let v = serde_json::json!(1_700_000_000);
        assert_eq!(parse_expires_at(&v), Some(1_700_000_000_000));
        // 字符串数字
        let v = serde_json::json!("1700000000");
        assert_eq!(parse_expires_at(&v), Some(1_700_000_000_000));
        // RFC 3339
        let v = serde_json::json!("2026-12-31T00:00:00Z");
        assert!(parse_expires_at(&v).is_some());
        // 无效
        let v = serde_json::json!("not-a-date");
        assert_eq!(parse_expires_at(&v), None);
        let v = serde_json::json!(null);
        assert_eq!(parse_expires_at(&v), None);
    }

    #[test]
    fn build_window_extracts_used_percent() {
        let json = r#"{"usedPercent": 42.5, "resets_at": "2026-06-30T00:00:00Z"}"#;
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let w = build_window(&v, LimitWindowKind::Session, "Session (5h)", 300).unwrap();
        assert_eq!(w.kind, LimitWindowKind::Session);
        assert_eq!(w.label, "Session (5h)");
        assert_eq!(w.used_percent, Some(42.5));
        assert_eq!(w.remaining_percent, Some(57.5));
        assert_eq!(w.resets_at.as_deref(), Some("2026-06-30T00:00:00Z"));
        assert_eq!(w.window_minutes, Some(300));
        assert!(w.show_meter);
        assert_eq!(w.used, 0.0);
        assert!(w.limit.is_none());
        assert!(w.remaining.is_none());
    }

    #[test]
    fn build_window_handles_snake_case_field() {
        let json = r#"{"used_percent": 80.0}"#;
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let w = build_window(&v, LimitWindowKind::Weekly, "Weekly (7d)", 10080).unwrap();
        assert_eq!(w.used_percent, Some(80.0));
        assert_eq!(w.remaining_percent, Some(20.0));
        assert_eq!(w.window_minutes, Some(10080));
    }

    #[test]
    fn build_window_clamps_remaining_percent_to_zero() {
        // used_percent > 100 时 remaining_percent 不应为负
        let json = r#"{"usedPercent": 120.0}"#;
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let w = build_window(&v, LimitWindowKind::Session, "Session (5h)", 300).unwrap();
        assert_eq!(w.remaining_percent, Some(0.0));
    }

    #[test]
    fn rank_by_mtime_filters_nonexistent_paths() {
        let nonexistent = PathBuf::from("/nonexistent-keypilot-test-12345/.credentials.json");
        let paths = vec![(nonexistent, "test".to_string())];
        let ranked = rank_by_mtime(paths);
        assert!(ranked.is_empty());
    }

    #[test]
    fn fetch_balance_returns_not_configured_when_no_credentials() {
        // 设置 CLAUDE_CONFIG_DIR 指向不存在的目录,模拟无凭据环境
        // (使用 temp_dir + 唯一后缀避免与其他测试冲突)
        let tmp = std::env::temp_dir().join(format!(
            "keypilot-test-no-claude-creds-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_millis()
        ));
        std::env::set_var("CLAUDE_CONFIG_DIR", &tmp);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio current_thread runtime");
        let result = rt.block_on(async {
            let source = ClaudeOAuthSource::new();
            source.fetch_balance().await
        });
        std::env::remove_var("CLAUDE_CONFIG_DIR");
        assert!(result.is_ok());
        let snap = result.unwrap();
        assert_eq!(snap.status, LimitStatus::NotConfigured);
        assert_eq!(snap.source, LimitSource::Oauth);
        assert!(snap.windows.is_empty());
    }

    #[test]
    fn is_available_returns_false_when_no_credentials() {
        // 同上:用 CLAUDE_CONFIG_DIR 指向不存在的目录
        let tmp = std::env::temp_dir().join(format!(
            "keypilot-test-claude-avail-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_millis()
        ));
        std::env::set_var("CLAUDE_CONFIG_DIR", &tmp);
        let source = ClaudeOAuthSource::new();
        let result = source.is_available();
        std::env::remove_var("CLAUDE_CONFIG_DIR");
        assert!(!result);
    }
}
