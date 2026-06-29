use async_trait::async_trait;
use chrono::Utc;
use tokio::task::JoinSet;
use crate::provider::adapter::QuotaError;
use crate::types::{
    LimitProvider, LimitSource, LimitStatus, LimitsSummary, QuotaSnapshot,
};

/// 与 ProviderAdapter 并存的 agent 余额查询 trait。
/// ProviderAdapter::fetch_quota(base_url, api_key) 假定 API key auth,
/// 但 Claude/Codex/Cursor 用 OAuth/CLI 凭据,没有 api_key。
#[async_trait]
pub trait AgentBalanceSource: Send + Sync {
    /// agent 类型标识,如 "claude" / "codex" / "cursor"
    fn agent_type(&self) -> &'static str;
    /// 显示名,如 "Claude Code" / "Codex" / "Cursor"
    fn display_name(&self) -> &'static str;
    /// 凭据文件存在 = true(快速判断,不读内容)
    fn is_available(&self) -> bool;
    /// 查询余额,返回 QuotaSnapshot 或 QuotaError
    async fn fetch_balance(&self) -> Result<QuotaSnapshot, QuotaError>;
}

/// 返回所有内置 agent source。
/// Task 1 阶段:返回 3 个 stub 实现(都返回 NotConfigured)。
/// Task 6 会把 stub 替换为真实实现。
pub fn default_agent_sources() -> Vec<Box<dyn AgentBalanceSource>> {
    vec![
        Box::new(crate::provider::agent_sources::claude_oauth::ClaudeOAuthSource::new()),
        Box::new(crate::provider::agent_sources::codex_rpc::CodexRpcSource::new()),
        Box::new(crate::provider::agent_sources::cursor_probe::CursorProbeSource::new()),
    ]
}

/// 并行调所有 source,失败的降级为 QuotaSnapshot,聚合为 LimitsSummary。
pub async fn fetch_all_agent_balances() -> LimitsSummary {
    let sources = default_agent_sources();
    let now_epoch = Utc::now().timestamp();

    // 用 tokio::task::JoinSet 并行(不引入 futures crate)。
    // JoinSet::spawn 要求 future 为 Send + 'static;
    // Box<dyn AgentBalanceSource> 已是 Send + Sync + 'static,move 进闭包即可。
    let mut set: JoinSet<LimitProvider> = JoinSet::new();
    for source in sources {
        set.spawn(async move {
            let agent_type = source.agent_type().to_string();
            let display_name = source.display_name().to_string();
            match source.fetch_balance().await {
                Ok(snapshot) => limit_provider_from_snapshot(&agent_type, &display_name, snapshot),
                Err(err) => {
                    let snapshot = quota_error_to_snapshot(err, &agent_type);
                    limit_provider_from_snapshot(&agent_type, &display_name, snapshot)
                }
            }
        });
    }

    let mut providers = Vec::new();
    while let Some(res) = set.join_next().await {
        // 单个 source panic 会传播为 JoinError;Task 1 阶段 stub 不会 panic,直接 unwrap。
        // Task 6 真实实现若需更稳健的容错,可替换为 unwrap_or_else 返回 fallback LimitProvider。
        providers.push(res.unwrap());
    }

    LimitsSummary {
        providers,
        updated_at: now_epoch,
    }
}

/// 把 QuotaSnapshot 转 LimitProvider(对齐 token-monitor normalizeLimitProvider 输出)
fn limit_provider_from_snapshot(
    agent_type: &str,
    _display_name: &str,
    snapshot: QuotaSnapshot,
) -> LimitProvider {
    LimitProvider {
        provider: agent_type.to_string(),
        windows: snapshot.windows,
        status: snapshot.status,
        source: snapshot.source,
        source_detail: snapshot.source_detail,
        account_label: snapshot.account_label,
        account_email: snapshot.account_email,
        region: snapshot.region,
        balance: snapshot.balance,
        used_amount: snapshot.used_amount,
        balance_usd: snapshot.balance_usd,
        used_usd: snapshot.used_usd,
    }
}

/// QuotaError → QuotaSnapshot 状态映射
/// Network → Unavailable / Parse → Error / Unsupported → NotConfigured
pub fn quota_error_to_snapshot(err: QuotaError, agent_type: &str) -> QuotaSnapshot {
    let status = match &err {
        QuotaError::Network(_) => LimitStatus::Unavailable,
        QuotaError::Parse(_) => LimitStatus::Error,
        QuotaError::Unsupported => LimitStatus::NotConfigured,
    };
    let _ = agent_type; // 仅用于日志,目前不输出
    QuotaSnapshot {
        total: None,
        used: 0.0,
        remaining: None,
        unit: String::new(),
        level: None,
        reset_at: None,
        windows: Vec::new(),
        status,
        source: LimitSource::Manual,
        source_detail: format!("{:?}", err),
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
    fn default_agent_sources_returns_three_sources() {
        let sources = default_agent_sources();
        assert_eq!(sources.len(), 3);
        let types: Vec<&str> = sources.iter().map(|s| s.agent_type()).collect();
        assert!(types.contains(&"claude"));
        assert!(types.contains(&"codex"));
        assert!(types.contains(&"cursor"));
    }

    #[test]
    fn fetch_all_agent_balances_returns_three_providers() {
        // 不用 #[tokio::test](需要 macros feature,而 Cargo.toml 未启用);
        // 改用 current_thread runtime 手动驱动,避免修改 Cargo.toml。
        // rt feature 已通过 tokio::spawn 在 postgres.rs 中的使用证实可用。
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("failed to build tokio current_thread runtime");
        rt.block_on(async {
            let summary = fetch_all_agent_balances().await;
            assert_eq!(summary.providers.len(), 3);
            // 3 个 source 已是真实实现:无凭据时返回 NotConfigured;
            // codex 在仅找到裸命令名 "codex" 但 spawn 失败时返回 Unavailable(也是合法降级)。
            // 测试环境通常没装 Claude/Codex/Cursor,允许 NotConfigured 或 Unavailable。
            for provider in &summary.providers {
                assert!(
                    matches!(
                        provider.status,
                        LimitStatus::NotConfigured | LimitStatus::Unavailable
                    ),
                    "provider {} status = {:?}, expected NotConfigured or Unavailable",
                    provider.provider,
                    provider.status
                );
            }
        });
    }

    #[test]
    fn quota_error_to_snapshot_maps_network_to_unavailable() {
        let snap = quota_error_to_snapshot(QuotaError::Network("timeout".into()), "claude");
        assert_eq!(snap.status, LimitStatus::Unavailable);
        assert!(snap.windows.is_empty());
    }

    #[test]
    fn quota_error_to_snapshot_maps_parse_to_error() {
        let snap = quota_error_to_snapshot(QuotaError::Parse("bad json".into()), "codex");
        assert_eq!(snap.status, LimitStatus::Error);
    }

    #[test]
    fn quota_error_to_snapshot_maps_unsupported_to_not_configured() {
        let snap = quota_error_to_snapshot(QuotaError::Unsupported, "cursor");
        assert_eq!(snap.status, LimitStatus::NotConfigured);
    }
}
