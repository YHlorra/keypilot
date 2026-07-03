

























































use crate::provider::coding_plan::subscription::{extract_reset_ms, make_error, make_success, parse_f64};
use crate::types::subscription::{CredentialStatus, QuotaTier, QuotaTierKind, SubscriptionQuota, TierStatus};

const HTTP_TIMEOUT_SECS: u64 = 15;


const PROVIDER_ID: &str = "zenmux";




pub async fn fetch(base_url: &str, api_key: &str) -> SubscriptionQuota {
    let client = reqwest::Client::new();

    let resp = match client
        .get(base_url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return make_error(PROVIDER_ID, format!("Network: {e}"), CredentialStatus::Valid),
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return make_error(
            PROVIDER_ID,
            format!("Auth failed: HTTP {status}"),
            CredentialStatus::Invalid,
        );
    }
    if !status.is_success() {
        return make_error(PROVIDER_ID, format!("HTTP {status}"), CredentialStatus::Valid);
    }

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => return make_error(PROVIDER_ID, format!("Parse: {e}"), CredentialStatus::Valid),
    };

    
    
    if body.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let msg = body
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return make_error(PROVIDER_ID, format!("API error: {msg}"), CredentialStatus::Valid);
    }

    let Some(data) = body.get("data") else {
        return make_error(
            PROVIDER_ID,
            "Missing 'data' field in response".to_string(),
            CredentialStatus::Valid,
        );
    };

    let tiers = parse_tiers(data);

    
    
    let plan_tier = data
        .get("plan")
        .and_then(|p| p.get("tier"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let account_status = data
        .get("account_status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let credential_message = if plan_tier.is_empty() {
        None
    } else if account_status.is_empty() {
        Some(plan_tier.to_string())
    } else {
        Some(format!("{plan_tier} ({account_status})"))
    };

    let mut quota = make_success(PROVIDER_ID, tiers);
    quota.credential_message = credential_message;
    quota
}














pub(crate) fn parse_tiers(data: &serde_json::Value) -> Vec<QuotaTier> {
    let mut tiers = Vec::new();

    if let Some(q5h) = data.get("quota_5_hour") {
        if let Some(tier) = parse_window(q5h, QuotaTierKind::FiveHour, "5h") {
            tiers.push(tier);
        }
    }

    if let Some(q7d) = data.get("quota_7_day") {
        if let Some(tier) = parse_window(q7d, QuotaTierKind::Weekly, "7d") {
            tiers.push(tier);
        }
    }

    tiers
}











fn parse_window(window: &serde_json::Value, kind: QuotaTierKind, label_prefix: &str) -> Option<QuotaTier> {
    let obj = window.as_object()?;

    
    let used_fraction = obj.get("usage_percentage").and_then(parse_f64).unwrap_or(0.0);
    let used_percent = (used_fraction * 100.0).clamp(0.0, 100.0);
    let remaining_percent = (100.0 - used_percent).clamp(0.0, 100.0);

    let resets_at_ms = obj.get("resets_at").and_then(extract_reset_ms);

    let used = obj.get("used_value_usd").and_then(parse_f64);
    let limit = obj.get("max_value_usd").and_then(parse_f64);

    Some(QuotaTier {
        kind,
        label: format!("ZenMux {label_prefix}"),
        used,
        limit,
        used_percent: Some(used_percent),
        remaining_percent: Some(remaining_percent),
        resets_at_ms,
        reset_description: String::new(),
        
        
        
        status: TierStatus::Active,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tiers_both_windows_with_usd_and_iso_reset() {
        
        
        let data = serde_json::json!({
            "quota_5_hour": {
                "usage_percentage": 0.25,
                "resets_at": "2026-07-02T12:00:00Z",
                "used_value_usd": 1.25,
                "max_value_usd": 5.0
            },
            "quota_7_day": {
                "usage_percentage": 0.5,
                "resets_at": "2026-07-09T00:00:00Z",
                "used_value_usd": 5.0,
                "max_value_usd": 10.0
            },
            "plan": { "tier": "Pro" },
            "account_status": "active"
        });

        let tiers = parse_tiers(&data);
        assert_eq!(tiers.len(), 2);

        
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].label, "ZenMux 5h");
        assert_eq!(tiers[0].used_percent, Some(25.0));
        assert_eq!(tiers[0].remaining_percent, Some(75.0));
        assert_eq!(tiers[0].used, Some(1.25));
        assert_eq!(tiers[0].limit, Some(5.0));
        assert_eq!(tiers[0].status, TierStatus::Active);
        assert_eq!(
            tiers[0].resets_at_ms,
            Some(1_782_993_600_000), 
        );

        
        assert_eq!(tiers[1].kind, QuotaTierKind::Weekly);
        assert_eq!(tiers[1].label, "ZenMux 7d");
        assert_eq!(tiers[1].used_percent, Some(50.0));
        assert_eq!(tiers[1].remaining_percent, Some(50.0));
        assert_eq!(tiers[1].used, Some(5.0));
        assert_eq!(tiers[1].limit, Some(10.0));
    }

    #[test]
    fn parse_tiers_handles_missing_7_day_window() {
        
        
        let data = serde_json::json!({
            "quota_5_hour": {
                "usage_percentage": 0.1,
                "resets_at": "2026-07-02T12:00:00Z"
            }
        });

        let tiers = parse_tiers(&data);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].used_percent, Some(10.0));
        assert_eq!(tiers[0].remaining_percent, Some(90.0));
        
        assert_eq!(tiers[0].used, None);
        assert_eq!(tiers[0].limit, None);
    }
}