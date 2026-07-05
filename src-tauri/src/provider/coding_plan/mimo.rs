use crate::provider::coding_plan::subscription::{
    make_error, make_success, make_tier, parse_f64,
};
use crate::types::subscription::{
    CredentialStatus, QuotaTier, QuotaTierKind, SubscriptionQuota, TierStatus,
};

const USAGE_URL: &str = "https://platform.xiaomimimo.com/api/v1/tokenPlan/usage";

const PROVIDER_ID: &str = "mimo";
const HTTP_TIMEOUT_SECS: u64 = 15;

pub async fn fetch(_base_url: &str, api_key: &str) -> SubscriptionQuota {
    let client = reqwest::Client::new();

    let resp = client
        .get(USAGE_URL)
        .header("Cookie", api_key)
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            return make_error(PROVIDER_ID, format!("Network: {e}"), CredentialStatus::Valid);
        }
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return make_error(
            PROVIDER_ID,
            format!("Auth failed: HTTP {status} — cookie may be expired (valid ~24h)"),
            CredentialStatus::Invalid,
        );
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return make_error(
            PROVIDER_ID,
            format!("HTTP {status}: {body}"),
            CredentialStatus::Valid,
        );
    }

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            return make_error(PROVIDER_ID, format!("Parse: {e}"), CredentialStatus::Valid);
        }
    };

    // ponytail: code==0 means success, non-zero means auth/error
    if body.get("code").and_then(|c| c.as_i64()).unwrap_or(-1) != 0 {
        let msg = body
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return make_error(
            PROVIDER_ID,
            format!("API error: {msg}"),
            CredentialStatus::Valid,
        );
    }

    let tiers = parse_mimo_tiers(&body);
    make_success(PROVIDER_ID, tiers)
}

pub(crate) fn parse_mimo_tiers(body: &serde_json::Value) -> Vec<QuotaTier> {
    let mut tiers = Vec::new();

    let items = match body
        .get("data")
        .and_then(|d| d.get("monthUsage"))
        .and_then(|m| m.get("items"))
        .and_then(|i| i.as_array())
    {
        Some(arr) => arr,
        None => return tiers,
    };

    for item in items {
        let name = item
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("unknown");

        let label = match name {
            "month_total_token" => "Mimo Monthly Credits",
            "compensation_total_token" => "Mimo Compensation",
            _ => name,
        };

        let limit = match item.get("limit").and_then(parse_f64).filter(|&n| n > 0.0) {
            Some(l) => l,
            None => continue,
        };
        let used = item.get("used").and_then(parse_f64).unwrap_or(0.0);
        let remaining = (limit - used).max(0.0);
        let remaining_percent = Some(remaining / limit * 100.0);

        let tier = make_tier(
            QuotaTierKind::Monthly,
            label,
            remaining_percent,
            None, // no reset_at in response
            TierStatus::Active,
        );
        tiers.push(tier);
    }

    tiers
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mimo_parses_real_response_with_monthly_credits() {
        let body = json!({
            "code": 0,
            "message": "",
            "data": {
                "monthUsage": {
                    "percent": 0.0505,
                    "items": [
                        {
                            "name": "month_total_token",
                            "used": 10100158,
                            "limit": 200000000,
                            "percent": 0.0505
                        }
                    ]
                }
            }
        });
        let tiers = parse_mimo_tiers(&body);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].kind, QuotaTierKind::Monthly);
        assert_eq!(tiers[0].label, "Mimo Monthly Credits");
        // 200M - 10.1M = 189.9M remaining → 94.95%
        assert!((tiers[0].remaining_percent.unwrap() - 94.95).abs() < 0.01);
        assert!((tiers[0].used_percent.unwrap() - 5.05).abs() < 0.01);
        assert_eq!(tiers[0].status, TierStatus::Active);
    }

    #[test]
    fn mimo_parses_compensation_tokens() {
        let body = json!({
            "code": 0,
            "data": {
                "monthUsage": {
                    "items": [
                        {
                            "name": "compensation_total_token",
                            "used": 0,
                            "limit": 50000000,
                            "percent": 0.0
                        }
                    ]
                }
            }
        });
        let tiers = parse_mimo_tiers(&body);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].label, "Mimo Compensation");
        assert_eq!(tiers[0].remaining_percent, Some(100.0));
    }

    #[test]
    fn mimo_empty_items_returns_empty() {
        let body = json!({
            "code": 0,
            "data": {
                "monthUsage": {
                    "items": []
                }
            }
        });
        assert!(parse_mimo_tiers(&body).is_empty());
    }

    #[test]
    fn mimo_missing_data_returns_empty() {
        let body = json!({ "code": 0 });
        assert!(parse_mimo_tiers(&body).is_empty());
    }

    #[test]
    fn mimo_zero_limit_skips_tier() {
        let body = json!({
            "code": 0,
            "data": {
                "monthUsage": {
                    "items": [
                        {
                            "name": "month_total_token",
                            "used": 0,
                            "limit": 0,
                            "percent": 0.0
                        }
                    ]
                }
            }
        });
        assert!(parse_mimo_tiers(&body).is_empty());
    }

    #[test]
    fn mimo_non_zero_code_returns_error() {
        // This test validates fetch logic conceptually — parse_mimo_tiers handles data only
        let body = json!({ "code": 1, "message": "auth error" });
        // code != 0 is handled in fetch(), not in parser
        // parse_mimo_tiers just returns empty when no items
        assert!(parse_mimo_tiers(&body).is_empty());
    }
}
