// protocols/deepseek.rs — extracted from provider/deepseek.rs (5th protocol, distinct from OpenAI)
use async_trait::async_trait;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::time::Duration;

use crate::catalog::ResolvedProvider;
use crate::provider::registry::{render_auth_header, QuotaError, ValidateError};
use crate::services::deepseek_balance_history;
use crate::timeutil;
use crate::types::{LimitSource, LimitStatus, LimitWindow, LimitWindowKind, MoneyAmount, QuotaSnapshot};

pub struct DeepSeekProtocol;

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
impl crate::provider::registry::ProtocolAdapter for DeepSeekProtocol {
    fn id(&self) -> &'static str {
        "deepseek"
    }

    async fn validate(&self, resolved: &ResolvedProvider, api_key: &str) -> Result<(), ValidateError> {
        let client = reqwest::Client::new();
        let base = resolved.base_url.trim_end_matches('/');
        let url = format!("{}/user/balance", base);
        let auth = render_auth_header(resolved, api_key);

        match client
            .get(&url)
            .header("Authorization", auth)
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

    async fn fetch_quota(&self, resolved: &ResolvedProvider, api_key: &str) -> Result<QuotaSnapshot, QuotaError> {
        let client = reqwest::Client::new();
        let base = resolved.base_url.trim_end_matches('/');
        let url = format!("{}/user/balance", base);
        let auth = render_auth_header(resolved, api_key);

        let resp = client
            .get(&url)
            .header("Authorization", auth)
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

        // USD rate heuristic: USD → 1.0, else → 6.8
        let usd_rate = if currency == "USD" { 1.0 } else { 6.8 };
        let balance_usd = remaining / usd_rate;
        let used_usd = used / usd_rate;

        // Compute account_key via sha256(api_key) for balance history side effect
        let account_key = {
            let mut hasher = Sha256::new();
            hasher.update(api_key.as_bytes());
            let digest = hasher.finalize();
            format!(
                "sha256:{}",
                deepseek_balance_history::bytes_to_hex(&digest)
            )
        };

        let now_ms = timeutil::now_millis();
        let consumption = deepseek_balance_history::record_consumption(
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
