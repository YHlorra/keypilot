// protocols/anthropic.rs — extracted from provider/anthropic.rs
use async_trait::async_trait;
use std::time::Duration;

use crate::catalog::ResolvedProvider;
use crate::provider::registry::{render_auth_header, QuotaError, ValidateError};
use crate::types::QuotaSnapshot;

pub struct AnthropicProtocol;

#[async_trait]
impl crate::provider::registry::ProtocolAdapter for AnthropicProtocol {
    fn id(&self) -> &'static str {
        "anthropic"
    }

    async fn validate(&self, resolved: &ResolvedProvider, api_key: &str) -> Result<(), ValidateError> {
        let client = reqwest::Client::new();
        let base = resolved.base_url.trim_end_matches('/');
        let url = format!("{}{}", base, resolved.validate_probe.path);
        let auth = render_auth_header(resolved, api_key);

        let body = serde_json::json!({
            "model": "claude-3-5-haiku-20241022",
            "max_tokens": 1,
            "messages": [{ "role": "user", "content": "test" }]
        });

        match client
            .post(&url)
            .header("x-api-key", auth)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .timeout(Duration::from_secs(10))
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status().as_u16();
                if resolved.validate_probe.success_status.contains(&status) {
                    Ok(())
                } else if status == 401 {
                    Err(ValidateError::InvalidKey)
                } else {
                    Err(ValidateError::Ambiguous)
                }
            }
            Err(e) => Err(ValidateError::Network(e.to_string())),
        }
    }

    async fn fetch_quota(&self, _resolved: &ResolvedProvider, _api_key: &str) -> Result<QuotaSnapshot, QuotaError> {
        // V0.2 does not implement Anthropic quota
        Err(QuotaError::Unsupported)
    }
}
