








































































use crate::provider::coding_plan::subscription::{make_error, make_tier};
use crate::types::subscription::{CredentialStatus, QuotaTierKind, SubscriptionQuota, TierStatus};




const QUOTA_URL: &str = "https://api.z.ai/api/monitor/usage/quota/limit";

const HTTP_TIMEOUT_SECS: u64 = 15;



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZhipuWindow {
    
    FiveHour,
    
    Weekly,
}









pub async fn fetch(base_url: &str, api_key: &str) -> SubscriptionQuota {
    let provider_id = "zhipu_en";
    
    
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
        Err(e) => return make_error(provider_id, format!("Network: {e}"), CredentialStatus::Valid),
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return make_error(
            provider_id,
            format!("Auth failed: HTTP {status}"),
            CredentialStatus::Invalid,
        );
    }
    if !status.is_success() {
        return make_error(provider_id, format!("HTTP {status}"), CredentialStatus::Valid);
    }

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => return make_error(provider_id, format!("Parse: {e}"), CredentialStatus::Valid),
    };

    
    
    
    if body.get("success").and_then(|v| v.as_bool()) == Some(false) {
        let msg = body
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return make_error(
            provider_id,
            format!("API error: {msg}"),
            CredentialStatus::Valid,
        );
    }

    let data = match body.get("data") {
        Some(d) => d,
        None => {
            return make_error(
                provider_id,
                "Missing 'data' field in response".to_string(),
                CredentialStatus::Valid,
            );
        }
    };

    let tiers = parse_zhipu_en_tiers(data);

    let level = data
        .get("level")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    SubscriptionQuota {
        provider_id: provider_id.to_string(),
        credential_status: CredentialStatus::Valid,
        credential_message: level,
        success: true,
        tiers,
        error: None,
        queried_at_ms: crate::provider::coding_plan::subscription::now_millis(),
    }
}





pub(crate) fn parse_zhipu_en_tiers(data: &serde_json::Value) -> Vec<crate::types::subscription::QuotaTier> {
    type Entry = (Option<i64>, Option<f64>);

    let mut five_hour: Option<Entry> = None;
    let mut weekly: Option<Entry> = None;
    let mut unclassified: Vec<Entry> = Vec::new();

    let limits = match data.get("limits").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };

    for limit_item in limits {
        
        
        
        let limit_type = limit_item
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !limit_type.eq_ignore_ascii_case("TOKENS_LIMIT") {
            continue;
        }

        let used_pct = limit_item.get("percentage").and_then(|v| v.as_f64());
        let reset_ms = limit_item.get("nextResetTime").and_then(|v| v.as_i64());
        let entry = (reset_ms, used_pct);

        match classify_zhipu_window(limit_item) {
            Some(ZhipuWindow::FiveHour) if five_hour.is_none() => five_hour = Some(entry),
            Some(ZhipuWindow::Weekly) if weekly.is_none() => weekly = Some(entry),
            _ => unclassified.push(entry),
        }
    }

    
    
    
    
    
    unclassified.sort_by_key(|(reset, _)| (reset.is_none(), reset.unwrap_or(i64::MIN)));
    for entry in unclassified {
        if five_hour.is_none() {
            five_hour = Some(entry);
        } else if weekly.is_none() {
            weekly = Some(entry);
        }
        
        
    }

    let mut tiers = Vec::new();
    if let Some((reset_ms, used_pct)) = five_hour {
        tiers.push(make_tier(
            QuotaTierKind::FiveHour,
            "GLM 5h",
            used_pct.map(|p| 100.0 - p),
            reset_ms,
            TierStatus::Active,
        ));
    }
    if let Some((reset_ms, used_pct)) = weekly {
        tiers.push(make_tier(
            QuotaTierKind::Weekly,
            "GLM Weekly",
            used_pct.map(|p| 100.0 - p),
            reset_ms,
            TierStatus::Active,
        ));
    }
    tiers
}






fn classify_zhipu_window(item: &serde_json::Value) -> Option<ZhipuWindow> {
    match item.get("unit").and_then(|v| v.as_i64()) {
        Some(3) => Some(ZhipuWindow::FiveHour),
        Some(6) => Some(ZhipuWindow::Weekly),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::subscription::QuotaTierKind;

    

    #[test]
    fn classify_zhipu_window_known_units() {
        let v_five = serde_json::json!({ "unit": 3, "number": 5 });
        let v_week = serde_json::json!({ "unit": 6, "number": 7 });
        assert_eq!(classify_zhipu_window(&v_five), Some(ZhipuWindow::FiveHour));
        assert_eq!(classify_zhipu_window(&v_week), Some(ZhipuWindow::Weekly));
    }

    #[test]
    fn classify_zhipu_window_unknown_or_missing_returns_none() {
        assert_eq!(classify_zhipu_window(&serde_json::json!({})), None);
        assert_eq!(classify_zhipu_window(&serde_json::json!({ "unit": 9 })), None);
        assert_eq!(classify_zhipu_window(&serde_json::json!({ "unit": "3" })), None);
    }

    

    #[test]
    fn parse_zhipu_en_tiers_inverts_percentage_and_emits_both_windows() {
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
                    "percentage": 50.0,
                    "nextResetTime": 1_783_267_200_000_i64
                }
            ]
        });
        let tiers = parse_zhipu_en_tiers(&data);
        assert_eq!(tiers.len(), 2);

        
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].label, "GLM 5h");
        assert_eq!(tiers[0].remaining_percent, Some(85.0));
        assert_eq!(tiers[0].used_percent, Some(15.0));
        assert_eq!(tiers[0].resets_at_ms, Some(1_783_008_000_000));

        
        assert_eq!(tiers[1].kind, QuotaTierKind::Weekly);
        assert_eq!(tiers[1].label, "GLM Weekly");
        assert_eq!(tiers[1].remaining_percent, Some(50.0));
        assert_eq!(tiers[1].used_percent, Some(50.0));
        assert_eq!(tiers[1].resets_at_ms, Some(1_783_267_200_000));
    }

    #[test]
    fn parse_zhipu_en_tiers_handles_old_plan_single_tier() {
        
        
        
        let data = serde_json::json!({
            "level": "lite",
            "limits": [{
                "type": "TOKENS_LIMIT",
                "unit": 99, 
                "number": 1,
                "percentage": 25.0,
                "nextResetTime": 1_783_008_000_000_i64
            }]
        });
        let tiers = parse_zhipu_en_tiers(&data);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].remaining_percent, Some(75.0));
    }

    #[test]
    fn parse_zhipu_en_tiers_skips_non_tokens_limit_entries() {
        
        
        let data = serde_json::json!({
            "level": "pro",
            "limits": [
                { "type": "RPM_LIMIT", "unit": 3, "percentage": 99.0 },
                {
                    "type": "TOKENS_LIMIT",
                    "unit": 3,
                    "number": 5,
                    "percentage": 20.0,
                    "nextResetTime": 1_783_008_000_000_i64
                }
            ]
        });
        let tiers = parse_zhipu_en_tiers(&data);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].remaining_percent, Some(80.0));
    }

    #[test]
    fn parse_zhipu_en_tiers_empty_when_no_limits() {
        let data = serde_json::json!({ "level": "pro" });
        assert!(parse_zhipu_en_tiers(&data).is_empty());
    }
}