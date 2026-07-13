























































use crate::provider::coding_plan::subscription::{
    extract_reset_ms, make_error, make_tier, parse_f64,
};
use crate::types::subscription::{
    CredentialStatus, QuotaTier, QuotaTierKind, SubscriptionQuota, TierStatus,
};


pub const PROVIDER_ID: &str = "volcengine";


const OPENAPI_HOST: &str = "open.volcengineapi.com";
const API_VERSION: &str = "2024-01-01";

const DEFAULT_REGION: &str = "cn-beijing";
const SERVICE: &str = "ark";
const CONTENT_TYPE: &str = "application/json; charset=utf-8";

const SIGNED_HEADERS: &str = "host;x-date;x-content-sha256;content-type";

const HTTP_TIMEOUT_SECS: u64 = 15;


const AKSK_HINT: &str =
    "Check the AccessKey ID / Secret are correct and the account has Ark usage-query (OpenAPI) permission.";







fn split_aksk(api_key: &str) -> Option<(&str, &str)> {
    let mut parts = api_key.splitn(2, '\n');
    let ak = parts.next()?.trim();
    let sk = parts.next()?.trim();
    if ak.is_empty() || sk.is_empty() {
        return None;
    }
    
    
    
    if ak.len() < 4 || sk.len() < 8 {
        return None;
    }
    Some((ak, sk))
}








fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};

    const BLOCK_SIZE: usize = 64;
    let mut normalized_key = [0u8; BLOCK_SIZE];
    let key_hash = Sha256::digest(key);
    let key_bytes: &[u8] = if key.len() > BLOCK_SIZE {
        &key_hash[..]
    } else {
        key
    };
    normalized_key[..key_bytes.len()].copy_from_slice(key_bytes);

    let mut ipad = [0x36u8; BLOCK_SIZE];
    let mut opad = [0x5cu8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        ipad[i] ^= normalized_key[i];
        opad[i] ^= normalized_key[i];
    }

    let mut inner = Sha256::new();
    inner.update(&ipad);
    inner.update(data);
    let inner = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(&opad);
    outer.update(&inner);
    outer.finalize().into()
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(data))
}


fn uri_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => {
                use std::fmt::Write;
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}



fn canonical_query(action: &str, region: &str) -> String {
    let mut pairs = [
        ("Action", action),
        ("Region", region),
        ("Version", API_VERSION),
    ];
    pairs.sort_by(|a, b| a.0.cmp(b.0));
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", uri_encode(k), uri_encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}



fn region_from_base_url(base_url: &str) -> String {
    let host = base_url
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(base_url)
        .split('/')
        .next()
        .unwrap_or("");
    host.split('.')
        .find(|p| p.starts_with("cn-") || p.starts_with("ap-"))
        .map(|p| p.to_string())
        .unwrap_or_else(|| DEFAULT_REGION.to_string())
}




fn sign(
    access_key_id: &str,
    secret_access_key: &str,
    region: &str,
    canonical_query_str: &str,
    body: &[u8],
    now: chrono::DateTime<chrono::Utc>,
) -> (String, String, String) {
    let x_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let short_date = now.format("%Y%m%d").to_string();
    let x_content_sha256 = sha256_hex(body);

    
    let canonical_headers = format!(
        "host:{OPENAPI_HOST}\nx-date:{x_date}\nx-content-sha256:{x_content_sha256}\ncontent-type:{CONTENT_TYPE}\n"
    );
    let canonical_request = format!(
        "POST\n/\n{canonical_query_str}\n{canonical_headers}\n{SIGNED_HEADERS}\n{x_content_sha256}"
    );

    let credential_scope = format!("{short_date}/{region}/{SERVICE}/request");
    let string_to_sign = format!(
        "HMAC-SHA256\n{x_date}\n{credential_scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );

    
    let k_date = hmac_sha256(secret_access_key.as_bytes(), short_date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, SERVICE.as_bytes());
    let k_signing = hmac_sha256(&k_service, b"request");
    let signature: String = hmac_sha256(&k_signing, string_to_sign.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    let authorization = format!(
        "HMAC-SHA256 Credential={access_key_id}/{credential_scope}, SignedHeaders={SIGNED_HEADERS}, Signature={signature}"
    );
    (authorization, x_date, x_content_sha256)
}




fn is_auth_error_code(code: &str) -> bool {
    let c = code.to_lowercase();
    c.contains("auth")
        || c.contains("signature")
        || c.contains("accessdenied")
        || c.contains("denied")
        || c.contains("unauthorized")
        || c.contains("forbidden")
        || c.contains("credential")
        || c.contains("token")
}


fn response_error(body: &serde_json::Value) -> Option<(String, String)> {
    let err = body
        .get("ResponseMetadata")
        .and_then(|m| m.get("Error"))
        .or_else(|| body.get("Error"))?;
    let code = err
        .get("Code")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let msg = err
        .get("Message")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if code.is_empty() && msg.is_empty() {
        None
    } else {
        Some((code, msg))
    }
}


enum VolcCall {
    
    Body(serde_json::Value),
    
    
    Auth(String),
    
    Soft(String),
}

async fn openapi_call(
    region: &str,
    access_key_id: &str,
    secret_access_key: &str,
    action: &str,
) -> VolcCall {
    let client = reqwest::Client::new();
    
    let cq = canonical_query(action, region);
    let url = format!("https://{OPENAPI_HOST}/?{cq}");
    let body: &[u8] = b"";
    let (authorization, x_date, x_content_sha256) = sign(
        access_key_id,
        secret_access_key,
        region,
        &cq,
        body,
        chrono::Utc::now(), // intentional Utc: SigV4 X-Date header must be UTC
    );

    let resp = match client
        .post(&url)
        .header("X-Date", x_date)
        .header("X-Content-Sha256", x_content_sha256)
        .header("Content-Type", CONTENT_TYPE)
        .header("Authorization", authorization)
        .body(body.to_vec())
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return VolcCall::Soft(format!("Network: {e}")),
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return VolcCall::Auth(format!("Auth failed: HTTP {status}. {AKSK_HINT}"));
    }
    if !status.is_success() {
        
        
        
        let raw = resp.text().await.unwrap_or_default();
        if let Ok(body) = serde_json::from_str::<serde_json::Value>(&raw) {
            if let Some((code, msg)) = response_error(&body) {
                if is_auth_error_code(&code) {
                    return VolcCall::Auth(format!(
                        "Auth failed: HTTP {status}, {code}: {msg}. {AKSK_HINT}"
                    ));
                }
                return VolcCall::Soft(format!("API error: HTTP {status}, {code}: {msg}"));
            }
        }
        return VolcCall::Soft(format!("HTTP {status}: {raw}"));
    }

    let body_value: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => return VolcCall::Soft(format!("Parse: {e}")),
    };

    
    if let Some((code, msg)) = response_error(&body_value) {
        if is_auth_error_code(&code) {
            return VolcCall::Auth(format!("Auth failed: {code}: {msg}. {AKSK_HINT}"));
        }
        return VolcCall::Soft(format!("API error: {code}: {msg}"));
    }

    VolcCall::Body(body_value)
}









pub(crate) fn parse_afp_tiers(result: &serde_json::Value) -> Vec<QuotaTier> {
    let mut tiers = Vec::new();
    for (key, kind, label) in [
        ("AFPFiveHour", QuotaTierKind::FiveHour, "AFP 5h"),
        ("AFPWeekly", QuotaTierKind::Weekly, "AFP Weekly"),
        ("AFPMonthly", QuotaTierKind::Monthly, "AFP Monthly"),
    ] {
        let Some(win) = result.get(key) else { continue };
        let quota = win.get("Quota").and_then(parse_f64).unwrap_or(0.0);
        if quota <= 0.0 {
            continue;
        }
        let used = win.get("Used").and_then(parse_f64).unwrap_or(0.0);
        let remaining_percent = ((quota - used) / quota * 100.0).clamp(0.0, 100.0);
        let used_percent = (100.0 - remaining_percent).clamp(0.0, 100.0);
        let resets_at_ms = win.get("ResetTime").and_then(extract_reset_ms);
        
        tiers.push(QuotaTier {
            kind,
            label: label.to_string(),
            used: Some(used),
            limit: Some(quota),
            used_percent: Some(used_percent),
            remaining_percent: Some(remaining_percent),
            resets_at_ms,
            reset_description: String::new(),
            status: TierStatus::Active,
        });
    }
    tiers
}


fn coding_window_kind(label: &str) -> Option<QuotaTierKind> {
    match label.to_lowercase().as_str() {
        "session" | "5h" | "fivehour" | "five_hour" | "rolling_5h" => Some(QuotaTierKind::FiveHour),
        "weekly" | "week" | "7d" => Some(QuotaTierKind::Weekly),
        "monthly" | "month" => Some(QuotaTierKind::Monthly),
        _ => None,
    }
}

fn coding_window_label(label: &str) -> &'static str {
    match label.to_lowercase().as_str() {
        "session" | "5h" | "fivehour" | "five_hour" | "rolling_5h" => "Coding 5h",
        "weekly" | "week" | "7d" => "Coding Weekly",
        "monthly" | "month" => "Coding Monthly",
        _ => "Coding",
    }
}





pub(crate) fn parse_coding_plan_tiers(result: &serde_json::Value) -> Vec<QuotaTier> {
    let mut tiers = Vec::new();
    let arr = result
        .get("QuotaUsage")
        .and_then(|v| v.as_array())
        .or_else(|| result.get("Usages").and_then(|v| v.as_array()))
        .or_else(|| result.get("Details").and_then(|v| v.as_array()));
    let Some(arr) = arr else { return tiers };

    for item in arr {
        let label = item
            .get("Level")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("Type").and_then(|v| v.as_str()))
            .or_else(|| item.get("Period").and_then(|v| v.as_str()))
            .or_else(|| item.get("Label").and_then(|v| v.as_str()))
            .or_else(|| item.get("Window").and_then(|v| v.as_str()))
            .unwrap_or("");
        let Some(kind) = coding_window_kind(label) else {
            continue;
        };
        let used_percent = item
            .get("Percent")
            .and_then(parse_f64)
            .or_else(|| item.get("UsedPercent").and_then(parse_f64))
            .or_else(|| item.get("UsagePercent").and_then(parse_f64))
            .unwrap_or(0.0)
            .clamp(0.0, 100.0);
        let resets_at_ms = item
            .get("ResetTime")
            .or_else(|| item.get("ResetTimestamp"))
            .and_then(extract_reset_ms);
        tiers.push(make_tier(
            kind,
            coding_window_label(label),
            Some(100.0 - used_percent),
            resets_at_ms,
            TierStatus::Active,
        ));
    }

    tiers
}






pub async fn fetch(base_url: &str, api_key: &str) -> SubscriptionQuota {
    let (ak, sk) = match split_aksk(api_key) {
        Some(v) => v,
        None => {
            return make_error(
                PROVIDER_ID,
                "Volcengine requires AccessKey ID + Secret (api_key format: 'ak\\nsk')"
                    .to_string(),
                CredentialStatus::Invalid,
            );
        }
    };
    query_volcengine(base_url, ak, sk).await
}



async fn query_volcengine(base_url: &str, ak: &str, sk: &str) -> SubscriptionQuota {
    let region = region_from_base_url(base_url);
    let mut soft_errors: Vec<String> = Vec::new();
    
    
    let mut empty_responses: Vec<String> = Vec::new();
    let summarize = |action: &str, body: &serde_json::Value| -> String {
        let raw: String = body.to_string().chars().take(700).collect();
        format!("{action}={raw}")
    };

    
    match openapi_call(&region, ak, sk, "GetAFPUsage").await {
        VolcCall::Auth(detail) => {
            return make_error(PROVIDER_ID, detail, CredentialStatus::Invalid);
        }
        VolcCall::Soft(detail) => soft_errors.push(format!("GetAFPUsage: {detail}")),
        VolcCall::Body(body) => {
            let result = body.get("Result").unwrap_or(&body);
            let tiers = parse_afp_tiers(result);
            if !tiers.is_empty() {
                let plan = result
                    .get("PlanType")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| format!("Agent Plan {s}"));
                return make_quota(tiers, plan);
            }
            empty_responses.push(summarize("GetAFPUsage", &body));
        }
    }

    
    match openapi_call(&region, ak, sk, "GetCodingPlanUsage").await {
        VolcCall::Auth(detail) => {
            return make_error(PROVIDER_ID, detail, CredentialStatus::Invalid);
        }
        VolcCall::Soft(detail) => soft_errors.push(format!("GetCodingPlanUsage: {detail}")),
        VolcCall::Body(body) => {
            let result = body.get("Result").unwrap_or(&body);
            let tiers = parse_coding_plan_tiers(result);
            if !tiers.is_empty() {
                return make_quota(tiers, Some("Coding Plan".to_string()));
            }
            empty_responses.push(summarize("GetCodingPlanUsage", &body));
        }
    }

    if !soft_errors.is_empty() {
        make_error(PROVIDER_ID, soft_errors.join("; "), CredentialStatus::Valid)
    } else if !empty_responses.is_empty() {
        
        
        make_error(
            PROVIDER_ID,
            format!(
                "No active subscription found (signature OK). Raw: {}",
                empty_responses.join(" || ")
            ),
            CredentialStatus::Valid,
        )
    } else {
        make_error(
            PROVIDER_ID,
            "No active Agent Plan or Coding Plan subscription found for this credential"
                .to_string(),
            CredentialStatus::Valid,
        )
    }
}


fn make_quota(tiers: Vec<QuotaTier>, plan: Option<String>) -> SubscriptionQuota {
    SubscriptionQuota {
        provider_id: PROVIDER_ID.to_string(),
        credential_status: CredentialStatus::Valid,
        credential_message: plan,
        success: true,
        tiers,
        error: None,
        queried_at_ms: crate::provider::coding_plan::subscription::now_millis(),
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    
    
    #[test]
    fn sign_uses_volcengine_specific_variant() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-07-02T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let cq = "Action=GetAFPUsage&Region=cn-beijing&Version=2024-01-01";
        let (authorization, x_date, x_content_sha256) =
            sign("AKIDEXAMPLE", "secretexample", "cn-beijing", cq, b"", now);
        
        assert!(
            authorization.starts_with("HMAC-SHA256 Credential=AKIDEXAMPLE/"),
            "algorithm must be HMAC-SHA256 without AWS4 prefix, got: {authorization}"
        );
        
        assert!(
            authorization.contains("/cn-beijing/ark/request,"),
            "credential scope must end with /request (not aws4_request), got: {authorization}"
        );
        
        assert!(
            authorization.contains("SignedHeaders=host;x-date;x-content-sha256;content-type"),
            "SignedHeaders must use the fixed volcengine order, got: {authorization}"
        );
        
        assert_eq!(x_date, "20260702T000000Z");
        
        assert_eq!(
            x_content_sha256,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        
        let (auth2, _, _) = sign("AKIDEXAMPLE", "secretexample", "cn-beijing", cq, b"", now);
        assert_eq!(authorization, auth2);
    }

    
    #[test]
    fn parse_afp_tiers_full_response_with_zero_quota_filter() {
        let result = json!({
            "AFPFiveHour": { "Quota": 1000.0, "Used": 250.0, "ResetTime": 1_782_993_600_000_i64 },
            "AFPWeekly":   { "Quota": 0.0,    "Used": 0.0,    "ResetTime": 1_783_267_200_000_i64 },
            "AFPMonthly":  { "Quota": 50000.0, "Used": 20000.0, "ResetTime": 1_785_148_800_000_i64 },
            "PlanType": "Premium"
        });
        let tiers = parse_afp_tiers(&result);
        
        assert_eq!(tiers.len(), 2);

        assert_eq!(tiers[0].kind, QuotaTierKind::FiveHour);
        assert_eq!(tiers[0].label, "AFP 5h");
        assert_eq!(tiers[0].used, Some(250.0));
        assert_eq!(tiers[0].limit, Some(1000.0));
        assert_eq!(tiers[0].remaining_percent, Some(75.0));
        assert_eq!(tiers[0].used_percent, Some(25.0));
        assert_eq!(tiers[0].resets_at_ms, Some(1_782_993_600_000));
        assert_eq!(tiers[0].status, TierStatus::Active);

        assert_eq!(tiers[1].kind, QuotaTierKind::Monthly);
        assert_eq!(tiers[1].used, Some(20000.0));
        assert_eq!(tiers[1].limit, Some(50000.0));
        assert_eq!(tiers[1].remaining_percent, Some(60.0));
        assert_eq!(tiers[1].used_percent, Some(40.0));
    }
}