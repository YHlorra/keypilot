// protocols/openai.rs — extracted from provider/openai.rs
use async_trait::async_trait;
use chrono::{Datelike, Months, NaiveDate, TimeZone, Utc};
use serde::Deserialize;
use std::time::Duration;

use crate::catalog::ResolvedProvider;
use crate::provider::registry::{render_auth_header, QuotaError, ValidateError};
use crate::types::{LimitSource, LimitStatus, LimitWindow, LimitWindowKind, MoneyAmount, QuotaSnapshot};

pub struct OpenAiProtocol;

#[derive(Deserialize)]
struct SubResp {
    hard_limit_usd: f64,
}

#[derive(Deserialize)]
struct UsageResp {
    total_usage: f64,
}

#[async_trait]
impl crate::provider::registry::ProtocolAdapter for OpenAiProtocol {
    fn id(&self) -> &'static str {
        "openai"
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
        let auth = render_auth_header(resolved, api_key);

        // Subscription (hard limit)
        let sub_url = format!("{}/dashboard/billing/subscription", base);
        let sub_resp = client
            .get(&sub_url)
            .header("Authorization", auth.clone())
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| QuotaError::Network(e.to_string()))?;

        if !sub_resp.status().is_success() {
            return Err(QuotaError::Network(format!("subscription failed: HTTP {}", sub_resp.status())));
        }

        let sub: SubResp = sub_resp
            .json()
            .await
            .map_err(|e| QuotaError::Parse(e.to_string()))?;

        let hard_limit = sub.hard_limit_usd;

        // 3-month rolling usage
        let mut total_cents: f64 = 0.0;
        let mut start = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let now_date = Utc::now().date_naive();

        while start < now_date {
            let end_raw = start.checked_add_months(Months::new(3)).unwrap_or(now_date);
            let end = if end_raw > now_date { now_date } else { end_raw };

            let usage_url = format!(
                "{}/dashboard/billing/usage?start_date={}&end_date={}",
                base,
                start.format("%Y-%m-%d"),
                end.format("%Y-%m-%d")
            );

            let usage_resp = client
                .get(&usage_url)
                .header("Authorization", auth.clone())
                .timeout(Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| QuotaError::Network(e.to_string()))?;

            if usage_resp.status().is_success() {
                let usage: UsageResp = usage_resp
                    .json()
                    .await
                    .map_err(|e| QuotaError::Parse(e.to_string()))?;
                total_cents += usage.total_usage;
            }

            start = end;
        }

        let used = (total_cents / 100.0).ceil();
        let remaining = hard_limit - used;

        let level = match remaining / hard_limit {
            r if r > 0.5 => "green",
            r if r > 0.2 => "amber",
            r if r > 0.05 => "red",
            _ => "ruby",
        };

        let used_percent = if hard_limit > 0.0 { Some((used / hard_limit) * 100.0) } else { None };
        let remaining_percent = if hard_limit > 0.0 { Some((remaining / hard_limit) * 100.0) } else { None };

        let resets_at_iso = {
            let now = Utc::now();
            let next_month = now.date_naive().checked_add_months(Months::new(1));
            next_month.and_then(|d| d.with_day(1)).map(|d| {
                d.and_hms_opt(0, 0, 0)
                    .map(|dt| Utc.from_utc_datetime(&dt).to_rfc3339())
                    .unwrap_or_default()
            })
        };

        Ok(QuotaSnapshot {
            total: Some(hard_limit),
            used,
            remaining: Some(remaining),
            unit: "USD".to_string(),
            level: Some(level.to_string()),
            reset_at: None,
            windows: vec![LimitWindow {
                kind: LimitWindowKind::Billing,
                label: "Monthly".to_string(),
                used,
                limit: Some(hard_limit),
                remaining: Some(remaining),
                used_percent,
                remaining_percent,
                resets_at: resets_at_iso.clone(),
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
            balance: Some(MoneyAmount { amount: remaining, currency: "USD".to_string(), ..Default::default() }),
            used_amount: Some(MoneyAmount { amount: used, currency: "USD".to_string(), ..Default::default() }),
            balance_usd: Some(remaining),
            used_usd: Some(used),
        })
    }
}
