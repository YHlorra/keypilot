


























































use crate::provider::coding_plan::subscription::{make_error, make_tier};
use crate::types::subscription::{CredentialStatus, QuotaTier, QuotaTierKind, SubscriptionQuota, TierStatus};



const QUOTA_URL: &str = "https://open.bigmodel.cn/api/monitor/usage/quota/limit";


const PROVIDER_ID: &str = "zhipu_cn";

const HTTP_TIMEOUT_SECS: u64 = 15;




const LIMIT_TYPE_TOKENS: &str = "TOKENS_LIMIT";


const UNIT_FIVE_HOUR: i64 = 3;


const UNIT_WEEKLY: i64 = 6;






pub async fn fetch(base_url: &str, api_key: &str) -> SubscriptionQuota {
    
    
    let _ = base_url; 

    let client = reqwest::Client::new();
    let resp = client
        .get(QUOTA_URL)
        
        .header("Authorization", api_key)
        .header("Content-Type", "application/json")
        
        
        .header("Accept-Language", "en-US,en")
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .send()
        .await;

    let resp = match resp {
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

    
    if body.get("success").and_then(|v| v.as_bool()) == Some(false) {
        let msg = body
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return make_error(
            PROVIDER_ID,
            format!("API error: {msg}"),
            CredentialStatus::Valid,
        );
    }

    let data = match body.get("data") {
        Some(d) => d,
        None => return make_error(PROVIDER_ID, "Missing 'data' field".into(), CredentialStatus::Valid),
    };

    let tiers = parse_zhipu_token_tiers(data);

    
    
    
    let level = data
        .get("level")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty());

    SubscriptionQuota {
        provider_id: PROVIDER_ID.to_string(),
        credential_status: CredentialStatus::Valid,
        credential_message: level,
        success: true,
        tiers,
        error: None,
        queried_at_ms: crate::provider::coding_plan::subscription::now_millis(),
    }
}














pub(crate) fn parse_zhipu_token_tiers(data: &serde_json::Value) -> Vec<QuotaTier> {
    type Entry = (Option<i64>, f64);
    let mut five_hour: Option<Entry> = None;
    let mut weekly: Option<Entry> = None;
    let mut unclassified: Vec<Entry> = Vec::new();

    if let Some(limits) = data.get("limits").and_then(|v| v.as_array()) {
        for item in limits {
            let kind = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if !kind.eq_ignore_ascii_case(LIMIT_TYPE_TOKENS) {
                continue;
            }
            
            
            let used_pct = item
                .get("percentage")
                .and_then(crate::provider::coding_plan::subscription::parse_f64)
                .unwrap_or(0.0);
            
            
            let reset_ms = item
                .get("nextResetTime")
                .and_then(crate::provider::coding_plan::subscription::extract_reset_ms);
            let entry = (reset_ms, used_pct);

            match item.get("unit").and_then(|v| v.as_i64()) {
                Some(UNIT_FIVE_HOUR) if five_hour.is_none() => five_hour = Some(entry),
                Some(UNIT_WEEKLY) if weekly.is_none() => weekly = Some(entry),
                _ => unclassified.push(entry),
            }
        }
    }

    
    
    
    
    
    
    unclassified.sort_by_key(|(reset, _)| (reset.is_some(), reset.unwrap_or(0)));
    for entry in unclassified {
        if five_hour.is_none() {
            five_hour = Some(entry);
        } else if weekly.is_none() {
            weekly = Some(entry);
        }
        
        
    }

    let mut tiers = Vec::new();
    if let Some((reset_ms, used_pct)) = five_hour {
        tiers.push(tier_from_used_pct(QuotaTierKind::FiveHour, "General 5h", used_pct, reset_ms));
    }
    if let Some((reset_ms, used_pct)) = weekly {
        tiers.push(tier_from_used_pct(QuotaTierKind::Weekly, "General Weekly", used_pct, reset_ms));
    }
    tiers
}




fn tier_from_used_pct(kind: QuotaTierKind, label: &str, used_pct: f64, resets_at_ms: Option<i64>) -> QuotaTier {
    let remaining = (100.0 - used_pct).clamp(0.0, 100.0);
    
    
    make_tier(kind, label, Some(remaining), resets_at_ms, TierStatus::Active)
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::coding_plan::subscription::extract_reset_ms;

    #[test]
    fn parses_two_tier_response_with_unit_classifier() {
        
        let data = serde_json::json!({
            "level": "pro",
            "limits": [
                {
                    "type": "TOKENS_LIMIT",
                    "unit": 3,
                    "number": 5,
                    "percentage": 15.0,
                    "nextResetTime": 1_783_008_000_000_i64
                },
                {
                    "type": "TOKENS_LIMIT",
                    "unit": 6,
                    "number": 7,
                    "percentage": 25.0,
                    "nextResetTime": 1_785_600_000_000_i64
                }
            ]
        });

        let tiers = parse_zhipu_token_tiers(&data);
        assert_eq!(tiers.len(), 2);

        
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].label, "General 5h");
        assert_eq!(tiers[0].remaining_percent, Some(85.0));
        assert_eq!(tiers[0].used_percent, Some(15.0));
        assert_eq!(tiers[0].resets_at_ms, Some(1_783_008_000_000));
        assert_eq!(tiers[0].status, TierStatus::Active);

        
        assert_eq!(tiers[1].kind, QuotaTierKind::Weekly);
        assert_eq!(tiers[1].label, "General Weekly");
        assert_eq!(tiers[1].remaining_percent, Some(75.0));
        assert_eq!(tiers[1].used_percent, Some(25.0));
        assert_eq!(tiers[1].resets_at_ms, Some(1_785_600_000_000));
        assert_eq!(tiers[1].status, TierStatus::Active);
    }

    #[test]
    fn legacy_plan_emits_only_five_hour_tier() {
        
        let data = serde_json::json!({
            "level": "lite",
            "limits": [
                {
                    "type": "TOKENS_LIMIT",
                    "unit": 3,
                    "number": 5,
                    "percentage": 0.0,
                    "nextResetTime": 1_783_008_000_000_i64
                }
            ]
        });

        let tiers = parse_zhipu_token_tiers(&data);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].remaining_percent, Some(100.0));
        assert_eq!(tiers[0].used_percent, Some(0.0));
    }

    #[test]
    fn ignores_non_tokens_limit_entries() {
        
        
        let data = serde_json::json!({
            "level": "pro",
            "limits": [
                {
                    "type": "TIME_RANGE_LIMIT",
                    "unit": 3,
                    "percentage": 50.0,
                    "nextResetTime": 1_783_008_000_000_i64
                },
                {
                    "type": "TOKENS_LIMIT",
                    "unit": 3,
                    "number": 5,
                    "percentage": 10.0,
                    "nextResetTime": 1_783_008_000_000_i64
                }
            ]
        });

        let tiers = parse_zhipu_token_tiers(&data);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].remaining_percent, Some(90.0));
    }

    #[test]
    fn fallback_heuristic_when_unit_missing() {
        
        
        
        let data = serde_json::json!({
            "level": "pro",
            "limits": [
                {
                    "type": "TOKENS_LIMIT",
                    "percentage": 20.0,
                    "nextResetTime": 1_785_600_000_000_i64
                },
                {
                    "type": "TOKENS_LIMIT",
                    "percentage": 0.0
                    
                }
            ]
        });

        let tiers = parse_zhipu_token_tiers(&data);
        assert_eq!(tiers.len(), 2);
        
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].remaining_percent, Some(100.0));
        assert_eq!(tiers[0].resets_at_ms, None);
        
        assert_eq!(tiers[1].kind, QuotaTierKind::Weekly);
        assert_eq!(tiers[1].resets_at_ms, Some(1_785_600_000_000));
    }

    #[test]
    fn missing_data_field_returns_empty() {
        let data = serde_json::json!({});
        assert!(parse_zhipu_token_tiers(&data).is_empty());
    }

    #[test]
    fn percentage_clamped_via_remaining_inversion() {
        
        
        let data = serde_json::json!({
            "level": "pro",
            "limits": [{
                "type": "TOKENS_LIMIT",
                "unit": 3,
                "number": 5,
                "percentage": 110.0,
                "nextResetTime": 1_783_008_000_000_i64
            }]
        });
        let tiers = parse_zhipu_token_tiers(&data);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].remaining_percent, Some(0.0));
        assert_eq!(tiers[0].used_percent, Some(100.0));
    }

    #[test]
    fn end_to_end_success_path_attaches_level_to_credential_message() {
        
        
        
        
        let body = serde_json::json!({
            "success": true,
            "msg": "",
            "data": {
                "level": "max",
                "limits": [{
                    "type": "TOKENS_LIMIT",
                    "unit": 3,
                    "number": 5,
                    "percentage": 5.0,
                    "nextResetTime": 1_783_008_000_000_i64
                }]
            }
        });

        
        
        let data = body.get("data").expect("data");
        let tiers = parse_zhipu_token_tiers(data);
        let level = data.get("level").and_then(|v| v.as_str()).map(str::to_string);

        let quota = SubscriptionQuota {
            provider_id: PROVIDER_ID.to_string(),
            credential_status: CredentialStatus::Valid,
            credential_message: level,
            success: true,
            tiers,
            error: None,
            queried_at_ms: 0,
        };

        assert_eq!(quota.provider_id, "zhipu_cn");
        assert_eq!(quota.credential_message.as_deref(), Some("max"));
        assert!(quota.success);
        assert_eq!(quota.tiers.len(), 1);
        assert_eq!(quota.tiers[0].remaining_percent, Some(95.0));
    }

    
    
    
    #[test]
    fn zhipu_reset_timestamp_is_milliseconds() {
        let v = serde_json::json!(1_783_008_000_000_i64);
        assert_eq!(extract_reset_ms(&v), Some(1_783_008_000_000));
    }
}