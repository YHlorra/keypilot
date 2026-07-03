







use crate::types::subscription::{CredentialStatus, QuotaTier, QuotaTierKind, SubscriptionQuota, TierStatus};




pub fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}





pub fn parse_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}








pub fn extract_reset_ms(value: &serde_json::Value) -> Option<i64> {
    if let Some(s) = value.as_str() {
        
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            return Some(dt.timestamp_millis());
        }
        
        if let Ok(n) = s.parse::<i64>() {
            return normalize_epoch(n);
        }
        return None;
    }
    if let Some(n) = value.as_i64() {
        return normalize_epoch(n);
    }
    None
}


fn normalize_epoch(n: i64) -> Option<i64> {
    if n <= 0 {
        return None;
    }
    
    
    
    Some(if n < 1_000_000_000_000 { n * 1000 } else { n })
}








pub fn make_tier(
    kind: QuotaTierKind,
    label: &str,
    remaining_percent: Option<f64>,
    resets_at_ms: Option<i64>,
    status: TierStatus,
) -> QuotaTier {
    let remaining_percent_norm = remaining_percent.map(|p| p.clamp(0.0, 100.0));
    let used_percent = remaining_percent_norm.map(|p| 100.0 - p);
    QuotaTier {
        kind,
        label: label.to_string(),
        used: None,
        limit: None,
        used_percent,
        remaining_percent: remaining_percent_norm,
        resets_at_ms,
        reset_description: String::new(),
        status,
    }
}







pub fn make_error(
    provider_id: &str,
    msg: String,
    credential_status: CredentialStatus,
) -> SubscriptionQuota {
    SubscriptionQuota {
        provider_id: provider_id.to_string(),
        credential_status,
        credential_message: None,
        success: false,
        tiers: vec![],
        error: Some(msg),
        queried_at_ms: now_millis(),
    }
}




pub fn make_success(provider_id: &str, tiers: Vec<QuotaTier>) -> SubscriptionQuota {
    SubscriptionQuota {
        provider_id: provider_id.to_string(),
        credential_status: CredentialStatus::Valid,
        credential_message: None,
        success: true,
        tiers,
        error: None,
        queried_at_ms: now_millis(),
    }
}
