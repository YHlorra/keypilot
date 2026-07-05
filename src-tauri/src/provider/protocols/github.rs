// protocols/github.rs — extracted from provider/github.rs
use async_trait::async_trait;
use std::time::Duration;

use crate::catalog::ResolvedProvider;
use crate::provider::registry::{render_auth_header, QuotaError, ValidateError};
use crate::types::{LimitSource, LimitStatus, LimitWindow, LimitWindowKind, QuotaSnapshot};

pub struct GitHubProtocol;

#[async_trait]
impl crate::provider::registry::ProtocolAdapter for GitHubProtocol {
    fn id(&self) -> &'static str {
        "github"
    }

    async fn validate(&self, resolved: &ResolvedProvider, api_key: &str) -> Result<(), ValidateError> {
        let client = reqwest::Client::new();
        let base = resolved.base_url.trim_end_matches('/');
        let url = format!("{}{}", base, resolved.validate_probe.path);
        let auth = render_auth_header(resolved, api_key);

        match client
            .get(&url)
            .header("Authorization", auth)
            .header("Accept", "application/vnd.github+json")
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
            .header("Accept", "application/vnd.github+json")
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| QuotaError::Network(e.to_string()))?;

        if resp.status().as_u16() == 401 {
            return Err(QuotaError::Network("Invalid API token".to_string()));
        }
        if !resp.status().is_success() {
            return Err(QuotaError::Network(format!("HTTP {}", resp.status())));
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

        let level = if total > 0.0 {
            let pct = remaining / total;
            if pct > 0.5 { Some("green".to_string()) }
            else if pct > 0.2 { Some("amber".to_string()) }
            else if pct > 0.05 { Some("red".to_string()) }
            else { Some("ruby".to_string()) }
        } else {
            None
        };

        let used_percent = if total > 0.0 { Some((used / total) * 100.0) } else { None };
        let remaining_percent = if total > 0.0 { Some((remaining / total) * 100.0) } else { None };
        let resets_at_iso = chrono::DateTime::from_timestamp(core.reset, 0).map(|dt| dt.to_rfc3339());

        Ok(QuotaSnapshot {
            total: Some(total),
            used,
            remaining: Some(remaining),
            unit: "req".to_string(),
            level,
            reset_at: Some(core.reset),
            windows: vec![LimitWindow {
                kind: LimitWindowKind::Session,
                label: "Hourly".to_string(),
                used,
                limit: Some(total),
                remaining: Some(remaining),
                used_percent,
                remaining_percent,
                resets_at: resets_at_iso,
                window_minutes: Some(60),
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
