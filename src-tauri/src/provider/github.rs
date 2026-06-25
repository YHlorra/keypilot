use async_trait::async_trait;
use crate::provider::adapter::{QuotaError, ValidateError};
use crate::types::QuotaSnapshot;

pub struct GitHubAdapter;

#[async_trait]
impl super::ProviderAdapter for GitHubAdapter {
    fn preset(&self) -> &'static str {
        "github"
    }

    fn can_test(&self) -> bool {
        false // GitHub doesn't have a simple test_connection endpoint
    }

    fn can_fetch_quota(&self) -> bool {
        true
    }

    async fn validate_key(&self, _base_url: &str, _api_key: &str) -> Result<(), ValidateError> {
        // Not implemented - can_test() returns false, caller must check first
        Err(ValidateError::Ambiguous)
    }

    async fn fetch_quota(&self, base_url: &str, api_key: &str) -> Result<QuotaSnapshot, QuotaError> {
        let client = reqwest::Client::new();
        let url = format!("{}/rate_limit", base_url.trim_end_matches('/'));

        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Accept", "application/vnd.github+json")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| QuotaError::Network(e.to_string()))?;

        if resp.status().as_u16() == 401 {
            return Err(QuotaError::Network("Invalid API token".to_string()));
        }

        #[derive(serde::Deserialize)]
        struct RateLimitResponse {
            resources: ResourceWrapper,
        }

        #[derive(serde::Deserialize)]
        struct ResourceWrapper {
            core: CoreRateLimit,
        }

        #[derive(serde::Deserialize)]
        struct CoreRateLimit {
            limit: i64,
            used: i64,
            remaining: i64,
            reset: i64,
        }

        let rate_json: RateLimitResponse = resp
            .json()
            .await
            .map_err(|e| QuotaError::Parse(e.to_string()))?;

        let core = rate_json.resources.core;
        let total = core.limit as f64;
        let used = core.used as f64;
        let remaining = core.remaining as f64;

        // Determine level based on remaining percentage
        let level = if total > 0.0 {
            let pct = remaining / total;
            if pct > 0.5 {
                Some("green".to_string())
            } else if pct > 0.2 {
                Some("amber".to_string())
            } else if pct > 0.05 {
                Some("red".to_string())
            } else {
                Some("ruby".to_string())
            }
        } else {
            None
        };

        Ok(QuotaSnapshot {
            total: Some(total),
            used,
            remaining: Some(remaining),
            unit: "req".to_string(),
            level,
            reset_at: Some(core.reset),
        })
    }
}