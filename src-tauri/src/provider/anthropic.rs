use async_trait::async_trait;
use crate::provider::adapter::{QuotaError, ValidateError};
use crate::types::QuotaSnapshot;

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
        false // Anthropic doesn't expose quota API
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
        // Anthropic doesn't expose quota API
        Err(QuotaError::Unsupported)
    }
}