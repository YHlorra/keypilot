use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

use crate::provider::adapter::{QuotaError, ValidateError};
use crate::types::QuotaSnapshot;

pub struct DeepSeekAdapter;

#[derive(Deserialize)]
struct DeepSeekResp {
    balance_infos: Vec<BalanceInfo>,
    is_available: Option<bool>,
}

#[derive(Deserialize)]
struct BalanceInfo {
    currency: String,
    total_balance: f64,
}

#[async_trait]
impl super::ProviderAdapter for DeepSeekAdapter {
    fn preset(&self) -> &'static str {
        "deepseek"
    }

    fn can_test(&self) -> bool {
        true
    }

    fn can_fetch_quota(&self) -> bool {
        true
    }

    async fn validate_key(&self, base_url: &str, api_key: &str) -> Result<(), ValidateError> {
        let client = reqwest::Client::new();
        let url = format!("{}/user/balance", base_url.trim_end_matches('/'));

        match client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .timeout(Duration::from_secs(10))
            .send()
            .await
        {
            Ok(resp) => {
                match resp.status().as_u16() {
                    200 => Ok(()),
                    401 | 403 => Err(ValidateError::InvalidKey),
                    _ => Err(ValidateError::Ambiguous),
                }
            }
            Err(e) => Err(ValidateError::Network(e.to_string())),
        }
    }

    async fn fetch_quota(
        &self,
        base_url: &str,
        api_key: &str,
    ) -> Result<QuotaSnapshot, QuotaError> {
        let client = reqwest::Client::new();
        let url = format!("{}/user/balance", base_url.trim_end_matches('/'));

        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| QuotaError::Network(e.to_string()))?;

        if resp.status().as_u16() == 401 || resp.status().as_u16() == 403 {
            return Err(QuotaError::Network("Invalid API key".to_string()));
        }
        if !resp.status().is_success() {
            return Err(QuotaError::Network(format!("HTTP {}", resp.status())));
        }

        let body: DeepSeekResp = resp
            .json()
            .await
            .map_err(|e| QuotaError::Parse(e.to_string()))?;

        if body.balance_infos.is_empty() {
            return Err(QuotaError::Parse("empty balance_infos".to_string()));
        }

        let info = &body.balance_infos[0];
        let currency = info.currency.clone();
        let total_balance = info.total_balance;

        let level = if body.is_available.unwrap_or(true) {
            Some("green".to_string())
        } else {
            Some("red".to_string())
        };

        Ok(QuotaSnapshot {
            total: None,
            used: 0.0,
            remaining: Some(total_balance),
            unit: currency,
            level,
            reset_at: None,
        })
    }
}
