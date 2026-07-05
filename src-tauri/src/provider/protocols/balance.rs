// protocols/balance.rs — NEW generic balance protocol for user-custom providers
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

use crate::catalog::ResolvedProvider;
use crate::provider::registry::{render_auth_header, QuotaError, ValidateError};
use crate::types::{LimitSource, LimitStatus, LimitWindow, LimitWindowKind, QuotaSnapshot};

pub struct BalanceProtocol;

#[derive(Deserialize)]
struct BalanceResp {
    balance: f64,
    #[serde(default)]
    currency: Option<String>,
}

#[async_trait]
impl crate::provider::registry::ProtocolAdapter for BalanceProtocol {
    fn id(&self) -> &'static str {
        "balance"
    }

    async fn validate(&self, resolved: &ResolvedProvider, api_key: &str) -> Result<(), ValidateError> {
        let client = reqwest::Client::new();
        let base = resolved.base_url.trim_end_matches('/');
        let url = format!("{}{}", base, resolved.validate_probe.path);
        let auth = render_auth_header(resolved, api_key);

        match client
            .get(&url)
            .header("Authorization", auth)
            .timeout(Duration::from_secs(10))
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status().as_u16();
                if resolved.validate_probe.success_status.contains(&status) {
                    Ok(())
                } else if status == 401 || status == 403 {
                    Err(ValidateError::InvalidKey)
                } else {
                    Err(ValidateError::Ambiguous)
                }
            }
            Err(e) => Err(ValidateError::Network(e.to_string())),
        }
    }

    async fn fetch_quota(&self, resolved: &ResolvedProvider, api_key: &str) -> Result<QuotaSnapshot, QuotaError> {
        let client = reqwest::Client::new();
        let base = resolved.base_url.trim_end_matches('/');
        let url = format!("{}{}", base, resolved.quota_probe.path);
        let auth = render_auth_header(resolved, api_key);

        let resp = client
            .get(&url)
            .header("Authorization", auth)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| QuotaError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(QuotaError::Network(format!("HTTP {}", resp.status())));
        }

        let body: BalanceResp = resp
            .json()
            .await
            .map_err(|e| QuotaError::Parse(e.to_string()))?;

        let balance = body.balance;
        let currency = body.currency.unwrap_or_else(|| "USD".to_string());

        Ok(QuotaSnapshot {
            total: Some(balance),
            used: 0.0,
            remaining: Some(balance),
            unit: currency,
            level: Some("green".to_string()),
            reset_at: None,
            windows: vec![LimitWindow {
                kind: LimitWindowKind::Billing,
                label: "Balance".to_string(),
                used: 0.0,
                limit: Some(balance),
                remaining: Some(balance),
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
            balance: None,
            used_amount: None,
            balance_usd: None,
            used_usd: None,
        })
    }
}
