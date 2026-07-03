use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

use crate::provider::adapter::{QuotaError, ValidateError};
use crate::timeutil;
use crate::types::{LimitSource, LimitStatus, LimitWindow, LimitWindowKind, MoneyAmount, QuotaSnapshot};

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

        
        
        let used = 0.0_f64;
        let remaining = total_balance;

        
        
        let usd_rate = if currency == "USD" { 1.0 } else { 6.8 };
        let balance_usd = remaining / usd_rate;
        let used_usd = used / usd_rate;

        
        let account_key = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(api_key.as_bytes());
            let digest = hasher.finalize();
            format!(
                "sha256:{}",
                crate::services::deepseek_balance_history::bytes_to_hex(&digest)
            )
        };
        
        
        
        let now_ms = timeutil::now_millis();
        let consumption = crate::services::deepseek_balance_history::record_consumption(
            &account_key,
            &currency,
            total_balance,
            now_ms,
        )
        .map_err(|e| QuotaError::Network(format!("record_consumption failed: {}", e)))?;

        Ok(QuotaSnapshot {
            
            total: None,
            used,
            remaining: Some(remaining),
            unit: currency.clone(),
            level,
            reset_at: None,
            
            windows: vec![LimitWindow {
                kind: LimitWindowKind::Billing,
                label: "Monthly".to_string(),
                used,
                limit: None,
                remaining: Some(remaining),
                used_percent: Some(0.0),
                remaining_percent: Some(100.0),
                resets_at: None,
                window_minutes: None,
                reset_description: String::new(),
                show_meter: true,
            }],
            status: LimitStatus::Ok,
            source: LimitSource::Api,
            source_detail: "app".to_string(),
            account_label: None,
            account_email: None,
            region: None,
            balance: Some(MoneyAmount {
                amount: remaining,
                currency: currency.clone(),
                today_spend: Some(consumption.today_spend),
                month_spend: Some(consumption.month_spend),
                month_since_tracking: Some(consumption.month_since_tracking),
            }),
            used_amount: Some(MoneyAmount {
                amount: used,
                currency,
                ..Default::default()
            }),
            balance_usd: Some(balance_usd),
            used_usd: Some(used_usd),
        })
    }
}
