use async_trait::async_trait;
use crate::provider::adapter::{QuotaError, ValidateError};
use crate::types::{LimitSource, LimitStatus, QuotaSnapshot};

pub struct AnthropicAdapter;

#[async_trait]
impl super::ProviderAdapter for AnthropicAdapter {
    fn preset(&self) -> &'static str {
        "anthropic"
    }

    fn can_test(&self) -> bool {
        true
    }

    fn can_fetch_quota(&self) -> bool {
        // 改为 true:虽然 Anthropic 没有 quota API,但我们现在总是返回
        // 一个 NotConfigured 状态的 QuotaSnapshot,让前端能展示提示
        true
    }

    async fn validate_key(&self, base_url: &str, api_key: &str) -> Result<(), ValidateError> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": "claude-3-5-haiku-20241022",
            "max_tokens": 1,
            "messages": [{
                "role": "user",
                "content": "test"
            }]
        });

        match client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(resp) => {
                match resp.status().as_u16() {
                    200 | 201 => Ok(()),
                    401 => Err(ValidateError::InvalidKey),
                    _ => Err(ValidateError::Ambiguous),
                }
            }
            Err(e) => Err(ValidateError::Network(e.to_string())),
        }
    }

    async fn fetch_quota(&self, _base_url: &str, _api_key: &str) -> Result<QuotaSnapshot, QuotaError> {
        // Anthropic 不暴露 quota API,但不再返回错误 — 而是返回一个
        // NotConfigured 状态的 QuotaSnapshot,让前端能显示
        // "Anthropic: not configured, set manually"。
        // 用户可以通过 set_manual_quota 命令覆盖此快照。
        Ok(QuotaSnapshot {
            // 旧字段(空值)
            total: None,
            used: 0.0,
            remaining: None,
            unit: "USD".to_string(),
            level: None,
            reset_at: None,
            // 新字段(对齐 token-monitor normalizeLimitProvider 输出)
            windows: Vec::new(),
            status: LimitStatus::NotConfigured,
            source: LimitSource::Manual,
            source_detail: "unknown".to_string(),
            account_label: None,
            account_email: None,
            region: None,
            balance: None,
            used_amount: None,
            balance_usd: None,
            used_usd: None,
        })
    }
}