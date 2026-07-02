// SPDX-License-Identifier: MIT
//
// Adapted from cc-switch (MIT, Copyright 2025 Jason Young). See
// `../../../../docs/third-party/cc-switch.LICENSE` for the verbatim license copy.

//! Volcengine (火山方舟) coding plan provider.
//!
//! ## ⚠️ AK/SK 双字段约定（与 keypilot 默认 `api_key` 单字段 schema 不同）
//!
//! 火山方舟需要 HMAC-SHA256 签名，签名需要 AK + SK 两个 key。keypilot 现有
//! provider 表的 `api_key` 字段是单字符串，所以本 provider **约定**用户把
//! api_key 填成 `"<ak>\n<sk>"` 格式（用单个换行符分隔）。
//!
//! 示例（用户在 AddCredentialModal 里粘贴到 api_key 字段）：
//! ```text
//! AKLTxxxxxxxxxxxxxxxx
//! SKltxxxxxxxxxxxxxxxx
//! ```
//! （上面是 2 行；保存时整个字符串存为 `AKLT...\nSKlt...`）
//!
//! V0.2 计划：拆出独立 `access_key` / `secret_key` 字段，需要 schema migration。
//!
//! ---
//!
//! Endpoint: 火山方舟 Coding / Agent Plan 的用量查询走**控制面 OpenAPI**
//! 统一网关 `https://open.volcengineapi.com/`（区别于数据面推理域名
//! `ark.cn-beijing.volces.com`），调用形如
//! `POST https://open.volcengineapi.com/?Action=...&Version=2024-01-01&Region=...`，
//! 强制**火山引擎签名 V4（AK/SK）**——复用推理 Bearer Key 会被网关以
//! `400 InvalidAuthorization` 拒绝（格式层拒绝，非权限问题）。
//!
//! 自动探测：先调 `GetAFPUsage`（Agent Plan，回绝对额度 Quota/Used），未
//! 订阅再调 `GetCodingPlanUsage`（Coding Plan，回已用百分比）。两个 plan
//! 共用同一份 AK/SK，故鉴权类错误直接停、不再试另一个 plan。
//!
//! Auth: AK/SK 通过 HMAC-SHA256 签名（详见 [`sign`]）。火山签名是标准
//! SigV4 的火山变体，有两处关键差异（照搬标准 SigV4 会签名失败）：
//!   1. canonical headers 与 SignedHeaders 用**固定顺序**
//!      `host;x-date;x-content-sha256;content-type`（不按字母序）；
//!   2. algorithm 串 `HMAC-SHA256`（无 `AWS4` 前缀）、credential scope
//!      结尾 `request`（非 `aws4_request`）、签名密钥派生时 SK 不加
//!      `AWS4` 前缀。
//!
//! api_key 编码约定：调用方将 AccessKey ID 与 Secret 用换行拼接后传入
//! `api_key`（即 `"<ak>\n<sk>"`）。Lane C 在 dispatcher 处按本约定拆分
//! 后传入本模块；拆分失败视为凭据缺失，返回 `CredentialStatus::Invalid`。
//!
//! Response shapes (verified against cc-switch reference):
//! - `GetAFPUsage`: `{ Result: { AFPFiveHour: { Quota, Used, ResetTime },
//!   AFPWeekly, AFPMonthly, PlanType } }` — 绝对值，`Quota<=0` 视为未订阅。
//! - `GetCodingPlanUsage`: `{ Result: { QuotaUsage: [{ Level, Percent,
//!   ResetTime }, ...] } }` — 已用百分比（0-100）。
//!
//! Design pattern adapted from cc-switch (MIT, Copyright 2025 Jason Young).
//! See [`../../../../docs/third-party/cc-switch.LICENSE`](../../../../docs/third-party/cc-switch.LICENSE).

use crate::provider::coding_plan::subscription::{
    extract_reset_ms, make_error, make_tier, parse_f64,
};
use crate::types::subscription::{
    CredentialStatus, QuotaTier, QuotaTierKind, SubscriptionQuota, TierStatus,
};

/// Provider id surfaced in [`SubscriptionQuota::provider_id`].
pub const PROVIDER_ID: &str = "volcengine";

/// 控制面 OpenAPI 统一网关（区别于数据面推理域名 ark.cn-beijing.volces.com）。
const OPENAPI_HOST: &str = "open.volcengineapi.com";
const API_VERSION: &str = "2024-01-01";
/// ark 控制面 OpenAPI 的默认 Region（Agent / Coding Plan 当前均在 cn-beijing）。
const DEFAULT_REGION: &str = "cn-beijing";
const SERVICE: &str = "ark";
const CONTENT_TYPE: &str = "application/json; charset=utf-8";
/// 火山 SigV4 固定顺序，**不按字母序**（这是火山与标准 SigV4 的关键差异之一）。
const SIGNED_HEADERS: &str = "host;x-date;x-content-sha256;content-type";

const HTTP_TIMEOUT_SECS: u64 = 15;

/// 鉴权失败时的引导文案，附加在错误后提示用户检查 AK/SK。
const AKSK_HINT: &str =
    "Check the AccessKey ID / Secret are correct and the account has Ark usage-query (OpenAPI) permission.";

// ── AK/SK 拆分 ──────────────────────────────────────────────

/// 从 `fetch()` 的 `api_key` 参数拆分 AccessKey ID 与 Secret。
///
/// 约定格式：`"<AccessKeyId>\n<SecretAccessKey>"`（换行分隔）。换行不会
/// 出现在 AK/SK 中，分隔无歧义。任一段为空字符串即视为凭据缺失。
fn split_aksk(api_key: &str) -> Option<(&str, &str)> {
    let mut parts = api_key.splitn(2, '\n');
    let ak = parts.next()?.trim();
    let sk = parts.next()?.trim();
    if ak.is_empty() || sk.is_empty() {
        return None;
    }
    // ponytail: minimum-length guard — Volcengine AK is 20+ chars, SK is
    // 40+ chars in the wild; rejecting shorter inputs catches typos and
    // pasted-partial-credential bugs without affecting any legit AK/SK.
    if ak.len() < 4 || sk.len() < 8 {
        return None;
    }
    Some((ak, sk))
}

// ── 火山引擎签名 V4（AK/SK）───────────────────────────────
//
// ponytail: 不引 `hmac` crate 以避免新增依赖（任务硬约束）。HMAC-SHA256
// 内部构造：`HMAC(K, m) = SHA256((K xor opad) || SHA256((K xor ipad) || m))`，
// opad=0x5c / ipad=0x36，K 长度超过 SHA256 块大小（64B）时先哈希一次。

/// HMAC-SHA256（RFC 2104）。Key 长度超过 64B 时先 SHA256 一次再使用。
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

/// RFC3986 unreserved 字符之外全部按 `%XX` 编码（用于 canonical query）。
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

/// 按 key 字母序排序的 canonical query string。同样的字符串既用于签名也用于
/// 实际请求 URL，保证两者逐字一致（否则签名不匹配）。
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

/// 从数据面 base_url 提取控制面 OpenAPI Region（如
/// `ark.cn-beijing.volces.com` → `cn-beijing`）；无法识别时回落 `cn-beijing`。
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

/// 生成火山引擎签名 V4 的鉴权头，返回 `(Authorization, X-Date, X-Content-Sha256)`。
/// 三者都要塞进请求头；`canonical_query_str` 必须与实际请求 URL 的 query 完全一致。
/// `now` 作参数传入便于写确定性单测。
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

    // 固定顺序 canonical headers（火山特有，**不排序**）。
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

    // 签名密钥派生：kDate=HMAC(SK, date)（SK **不加** AWS4 前缀），终止串 `request`。
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

// ── OpenAPI 响应处理 ──────────────────────────────────────

/// 判断 OpenAPI 错误码是否属于鉴权类（命中即停、提示换 AK/SK）。
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

/// 提取火山 OpenAPI 响应里的 `ResponseMetadata.Error`（或顶层 `Error`）。
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

/// 单次 OpenAPI 调用的归类结果。
enum VolcCall {
    /// 2xx 且 JSON 可解析、无 OpenAPI 级错误（业务 Result 仍可能为空=未订阅）。
    Body(serde_json::Value),
    /// 硬鉴权失败（HTTP 401/403 或 AccessDenied/Signature 等错误码）——两个 plan
    /// 共用凭据，命中即停。
    Auth(String),
    /// 网络 / 非鉴权 HTTP 错误 / 解析失败——记录后可继续尝试另一个 plan。
    Soft(String),
}

async fn openapi_call(
    region: &str,
    access_key_id: &str,
    secret_access_key: &str,
    action: &str,
) -> VolcCall {
    let client = reqwest::Client::new();
    // canonical query 同时用于签名与实际 URL，确保两者逐字一致。
    let cq = canonical_query(action, region);
    let url = format!("https://{OPENAPI_HOST}/?{cq}");
    let body: &[u8] = b"";
    let (authorization, x_date, x_content_sha256) = sign(
        access_key_id,
        secret_access_key,
        region,
        &cq,
        body,
        chrono::Utc::now(),
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
        // 火山 OpenAPI 网关对签名/凭据类错误常返 4xx（多为 HTTP 400）并携带与 200
        // 路径相同的 ResponseMetadata.Error 信封，而非 401/403。这里也解析信封，
        // 让 Bearer 被拒时仍能给出 AK/SK 引导并标记凭据失效。
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

    // 火山 OpenAPI 业务错误常以 200 + ResponseMetadata.Error 返回。
    if let Some((code, msg)) = response_error(&body_value) {
        if is_auth_error_code(&code) {
            return VolcCall::Auth(format!("Auth failed: {code}: {msg}. {AKSK_HINT}"));
        }
        return VolcCall::Soft(format!("API error: {code}: {msg}"));
    }

    VolcCall::Body(body_value)
}

// ── Tier 解析 ──────────────────────────────────────────────

/// 解析 `GetAFPUsage` 的 `Result` 为 tier 列表。
///
/// 展示 5h / 周 / 月三个窗口（与控制台一致）；`Quota/Used` 是绝对 AFP 值，
/// 已用百分比 = `Used/Quota*100`；`Quota<=0` 视为该窗口未订阅/未启用，
/// 跳过——也用于把"已鉴权但无 Agent Plan"识别为空结果，从而回落到
/// Coding Plan 探测。
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
        // 直接构造 QuotaTier 以填入绝对值 `used` / `limit`（make_tier 不支持）。
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

/// 把 `GetCodingPlanUsage` 的 window 标签归一到 [`QuotaTierKind`]。不识别返回 `None`。
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

/// 解析 `GetCodingPlanUsage` 的 `Result` 为 tier 列表（防御式）。
///
/// 真实字段是 `Level`（实测：`session` / `weekly` / `monthly`）、`Percent`、
/// `ResetTime`；这里宽松匹配多种字段名以应对上游命名变动。
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

// ── 入口 ────────────────────────────────────────────────────

/// Top-level entry point. Mirrors minimax's `fetch(base_url, api_key)` shape so
/// the dispatcher can call both providers uniformly; AK/SK are packed as
/// `"<ak>\n<sk>"` per the encoding contract above.
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

/// Probe `GetAFPUsage` (Agent Plan) then `GetCodingPlanUsage` (Coding Plan).
/// Two plans share the same AK/SK so any auth error short-circuits the ladder.
async fn query_volcengine(base_url: &str, ak: &str, sk: &str) -> SubscriptionQuota {
    let region = region_from_base_url(base_url);
    let mut soft_errors: Vec<String> = Vec::new();
    // 2xx + 无 Error 信封但解析不出额度时，截断原始响应用于诊断（区分"真没订阅"
    // 与"字段名/包裹层猜错"）。签名若不通会走 Auth/Soft 分支，到不了这里。
    let mut empty_responses: Vec<String> = Vec::new();
    let summarize = |action: &str, body: &serde_json::Value| -> String {
        let raw: String = body.to_string().chars().take(700).collect();
        format!("{action}={raw}")
    };

    // 1) Agent Plan: GetAFPUsage
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

    // 2) Coding Plan: GetCodingPlanUsage
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
        // 签名已通过、请求到达业务层，但响应里没有可解析的额度。带上原始响应，
        // 便于核对真实字段名/包裹层，或确认确实未订阅。
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

/// 构造成功响应（Volcengine 专用：可选 plan 信息写入 `credential_message`）。
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

// ── Unit tests ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 验证签名使用火山变体（非标准 SigV4）——这是整个 provider 最容易出错
    /// 也最致命的地方：签名错了整个鉴权就废了，照搬标准 SigV4 会签名失败。
    #[test]
    fn sign_uses_volcengine_specific_variant() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-07-02T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let cq = "Action=GetAFPUsage&Region=cn-beijing&Version=2024-01-01";
        let (authorization, x_date, x_content_sha256) =
            sign("AKIDEXAMPLE", "secretexample", "cn-beijing", cq, b"", now);
        // 1. algorithm 串 = HMAC-SHA256（**不带** AWS4 前缀——火山关键差异）。
        assert!(
            authorization.starts_with("HMAC-SHA256 Credential=AKIDEXAMPLE/"),
            "algorithm must be HMAC-SHA256 without AWS4 prefix, got: {authorization}"
        );
        // 2. credential scope 结尾 = `request`（**不是** `aws4_request`）。
        assert!(
            authorization.contains("/cn-beijing/ark/request,"),
            "credential scope must end with /request (not aws4_request), got: {authorization}"
        );
        // 3. SignedHeaders 用**固定顺序**（**不**按字母序）。
        assert!(
            authorization.contains("SignedHeaders=host;x-date;x-content-sha256;content-type"),
            "SignedHeaders must use the fixed volcengine order, got: {authorization}"
        );
        // 4. X-Date 必须是 YYYYMMDDTHHMMSSZ 格式。
        assert_eq!(x_date, "20260702T000000Z");
        // 5. X-Content-Sha256 = 空 body 的 SHA256。
        assert_eq!(
            x_content_sha256,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        // 6. 同一 (ak, sk, now, cq) 必须产出确定签名。
        let (auth2, _, _) = sign("AKIDEXAMPLE", "secretexample", "cn-beijing", cq, b"", now);
        assert_eq!(authorization, auth2);
    }

    /// AFP 主路径：三个窗口 → 三个 tier，保留绝对值、算出百分比、过滤零额度。
    #[test]
    fn parse_afp_tiers_full_response_with_zero_quota_filter() {
        let result = json!({
            "AFPFiveHour": { "Quota": 1000.0, "Used": 250.0, "ResetTime": 1_782_993_600_000_i64 },
            "AFPWeekly":   { "Quota": 0.0,    "Used": 0.0,    "ResetTime": 1_783_267_200_000_i64 },
            "AFPMonthly":  { "Quota": 50000.0, "Used": 20000.0, "ResetTime": 1_785_148_800_000_i64 },
            "PlanType": "Premium"
        });
        let tiers = parse_afp_tiers(&result);
        // Weekly 因 Quota==0 被跳过，只剩 5h + monthly。
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