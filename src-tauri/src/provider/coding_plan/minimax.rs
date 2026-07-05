










































use crate::provider::coding_plan::subscription::{make_error, make_success, make_tier, parse_f64};
use crate::types::subscription::{CredentialStatus, QuotaTierKind, SubscriptionQuota, TierStatus};



const HTTP_TIMEOUT_SECS: u64 = 15;




pub async fn fetch(base_url: &str, api_key: &str) -> SubscriptionQuota {
    
    
    
    let provider_id = if base_url.contains("api.minimax.io") {
        "minimax_en"
    } else {
        "minimax_cn"
    };

    // ponytail: derive token_plan endpoint from base_url so CN (api.minimaxi.com)
    // and EN (api.minimax.io) resolve to their own gateway. Hardcoding www.* broke EN.
    let token_plan_url = format!("{}/token_plan/remains", base_url.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let resp = client
        .get(&token_plan_url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
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

    
    if let Some(code) = body.get("base_resp").and_then(|b| b.get("status_code")).and_then(|c| c.as_i64()) {
        if code != 0 {
            let msg = body
                .get("base_resp")
                .and_then(|b| b.get("status_msg"))
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_string();
            return make_error(
                provider_id,
                format!("base_resp error {code}: {msg}"),
                CredentialStatus::Valid,
            );
        }
    }

    let tiers = parse_minimax_tiers(&body);
    make_success(provider_id, tiers)
}








pub(crate) fn parse_minimax_tiers(body: &serde_json::Value) -> Vec<crate::types::subscription::QuotaTier> {
    let mut tiers = Vec::new();

    let arr = match body.get("model_remains").and_then(|m| m.as_array()) {
        Some(a) => a,
        None => return tiers,
    };

    
    
    
    let Some(entry) = arr.iter().find(|item| {
        item.get("model_name")
            .and_then(|v| v.as_str())
            .map(|s| s == "general")
            .unwrap_or(false)
    }) else {
        return tiers;
    };

    
    let interval_pct = entry
        .get("current_interval_remaining_percent")
        .and_then(parse_f64);
    let interval_status = entry
        .get("current_interval_status")
        .and_then(|v| v.as_i64())
        .map(|n| match n {
            1 => TierStatus::Active,
            _ => TierStatus::Inactive,
        })
        .unwrap_or(TierStatus::Unknown);
    let interval_end_ms = entry.get("end_time").and_then(|v| v.as_i64());

    tiers.push(make_tier(
        QuotaTierKind::FiveHour,
        "General 5h",
        interval_pct,
        interval_end_ms,
        interval_status,
    ));

    
    
    let weekly_status_raw = entry.get("current_weekly_status").and_then(|v| v.as_i64());
    if weekly_status_raw == Some(1) {
        let weekly_pct = entry
            .get("current_weekly_remaining_percent")
            .and_then(parse_f64);
        let weekly_status = TierStatus::Active;
        let weekly_end_ms = entry.get("weekly_end_time").and_then(|v| v.as_i64());
        tiers.push(make_tier(
            QuotaTierKind::Weekly,
            "General Weekly",
            weekly_pct,
            weekly_end_ms,
            weekly_status,
        ));
    }

    tiers
}
