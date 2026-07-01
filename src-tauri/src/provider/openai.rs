use async_trait::async_trait;
use chrono::{Datelike, Months, NaiveDate, TimeZone, Utc};
use serde::Deserialize;
use std::time::Duration;

use crate::provider::adapter::{QuotaError, ValidateError};
use crate::types::{LimitSource, LimitStatus, LimitWindow, LimitWindowKind, MoneyAmount, QuotaSnapshot};

pub struct OpenAiAdapter;

#[derive(Deserialize)]
struct SubResp {
    hard_limit_usd: f64,
}

#[derive(Deserialize)]
struct UsageResp {
    total_usage: f64,
}

#[async_trait]
impl super::ProviderAdapter for OpenAiAdapter {
    fn preset(&self) -> &'static str {
        "openai"
    }

    fn can_test(&self) -> bool {
        true
    }

    fn can_fetch_quota(&self) -> bool {
        true
    }

    async fn validate_key(&self, base_url: &str, api_key: &str) -> Result<(), ValidateError> {
        let client = reqwest::Client::new();
        let url = format!("{}/models", base_url.trim_end_matches('/'));

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
        let base = base_url.trim_end_matches('/');

        // Step 1: GET subscription → hard_limit_usd (already USD, NOT cents)
        let sub_url = format!("{}/dashboard/billing/subscription", base);
        let sub_resp = client
            .get(&sub_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| QuotaError::Network(e.to_string()))?;

        if !sub_resp.status().is_success() {
            return Err(QuotaError::Network(format!(
                "subscription failed: HTTP {}",
                sub_resp.status()
            )));
        }

        let sub: SubResp = sub_resp
            .json()
            .await
            .map_err(|e| QuotaError::Parse(e.to_string()))?;

        let hard_limit = sub.hard_limit_usd; // already in USD, NO division by 100

        // Step 2: 3-month window iteration (cumulative from 2000-01-01)
        // Spec L98-150: while start < now: end = min(start + 3 months, now); GET usage; start = end
        let mut total_cents: f64 = 0.0;
        let mut start = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        // intentional Utc: OpenAI billing API expects UTC calendar dates per its spec;
        // sending Local dates would shift the window by TZ offset and miss usage on edges.
        let now_date = Utc::now().date_naive();

        while start < now_date {
            let end_raw = start.checked_add_months(Months::new(3)).unwrap_or(now_date);
            let end = if end_raw > now_date {
                now_date
            } else {
                end_raw
            };

            let usage_url = format!(
                "{}/dashboard/billing/usage?start_date={}&end_date={}",
                base,
                start.format("%Y-%m-%d"),
                end.format("%Y-%m-%d")
            );

            let usage_resp = client
                .get(&usage_url)
                .header("Authorization", format!("Bearer {}", api_key))
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

        // cents → USD, then ceil to eliminate float errors (spec L147, L149)
        let used = (total_cents / 100.0).ceil();
        let remaining = hard_limit - used;

        let level = match remaining / hard_limit {
            r if r > 0.5 => "green",
            r if r > 0.2 => "amber",
            r if r > 0.05 => "red",
            _ => "ruby",
        };

        // 计算百分比与 ISO 重置时间(月度结算窗口)
        let used_percent = if hard_limit > 0.0 {
            Some((used / hard_limit) * 100.0)
        } else {
            None
        };
        let remaining_percent = if hard_limit > 0.0 {
            Some((remaining / hard_limit) * 100.0)
        } else {
            None
        };
        // 月度窗口:下个月 1 号 00:00 UTC
        // intentional Utc: OpenAI wallet reset_at is reported in UTC; the unit is
        // stored verbatim and shown as a UTC clock string to match the source.
        let resets_at_iso = {
            let now = Utc::now();
            // 加 1 个月,然后把日改为 1 号;若失败 fallback 到加 1 月的日期
            let next_month = now.date_naive().checked_add_months(Months::new(1));
            next_month.and_then(|d| d.with_day(1)).map(|d| {
                d.and_hms_opt(0, 0, 0)
                    .map(|dt| Utc.from_utc_datetime(&dt).to_rfc3339())
                    .unwrap_or_default()
            })
        };

        Ok(QuotaSnapshot {
            // 旧字段(向后兼容)
            total: Some(hard_limit),
            used,
            remaining: Some(remaining),
            unit: "USD".to_string(),
            level: Some(level.to_string()),
            reset_at: None,
            // 新字段(对齐 token-monitor normalizeLimitProvider 输出)
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
