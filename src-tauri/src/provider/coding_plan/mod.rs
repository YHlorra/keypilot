//! Coding plan provider abstraction.
//!
//! Each provider exposes an async `fetch(base_url, api_key) -> SubscriptionQuota`
//! entry point. [`detect_provider`] dispatches by `base_url` substring.
//! [`fetch_coding_plan_quota`] is the single integration seam used by the
//! `fetch_coding_plan_quota` IPC command.
//!
//! Design inspired by cc-switch (MIT, Copyright 2025 Jason Young). See
//! [`../../docs/third-party/cc-switch.LICENSE`](../../docs/third-party/cc-switch.LICENSE).

pub mod kimi;
pub mod minimax;
pub mod subscription;
pub mod volcengine;
pub mod zenmux;
pub mod zhipu_cn;
pub mod zhipu_en;

#[cfg(test)]
mod tests;

/// Coding plan providers recognized by [`detect_provider`].
///
/// All variants are wired with real implementations — adding a new provider
/// means writing a per-provider module under [`crate::provider::coding_plan`]
/// and adding a `pub mod` declaration + a match arm in
/// [`fetch_coding_plan_quota`] below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodingPlanProvider {
    /// MiniMax China — `https://api.minimaxi.com`.
    MiniMaxCn,
    /// MiniMax overseas — `https://api.minimax.io`.
    MiniMaxEn,
    /// Kimi For Coding — `https://api.kimi.com/coding`.
    Kimi,
    /// 智谱 GLM China — `https://open.bigmodel.cn`.
    ZhipuCn,
    /// Zhipu GLM overseas — `https://api.z.ai`.
    ZhipuEn,
    /// 火山方舟 Coding / Agent Plan — `*.volces.com/api/coding`.
    Volcengine,
    /// ZenMux — host substring `zenmux`.
    ZenMux,
}

/// Route a `base_url` to its `CodingPlanProvider`.
///
/// Order matters: more specific matches (e.g. `api.minimaxi.com`) must come
/// before generic substring matches that might overlap.
pub fn detect_provider(base_url: &str) -> Option<CodingPlanProvider> {
    let url = base_url.to_lowercase();
    if url.contains("api.minimaxi.com") {
        Some(CodingPlanProvider::MiniMaxCn)
    } else if url.contains("api.minimax.io") {
        Some(CodingPlanProvider::MiniMaxEn)
    } else if url.contains("api.kimi.com/coding") || url.contains("api.moonshot.cn") {
        Some(CodingPlanProvider::Kimi)
    } else if url.contains("open.bigmodel.cn") {
        Some(CodingPlanProvider::ZhipuCn)
    } else if url.contains("api.z.ai") {
        Some(CodingPlanProvider::ZhipuEn)
    } else if url.contains("volces.com/api/") {
        // ponytail: covers both /api/v3 and /api/coding preset defaults.
        Some(CodingPlanProvider::Volcengine)
    } else if url.contains("zenmux") {
        Some(CodingPlanProvider::ZenMux)
    } else {
        None
    }
}

/// Top-level entry point for the coding plan quota IPC path.
///
/// Always returns a `SubscriptionQuota` (no `Result`): callers can surface
/// success / failure uniformly without exception translation. Network errors
/// and parse failures are encoded in the `error` field with `success = false`.
///
/// `api_key` is allowed to be empty — the dispatcher short-circuits to an
/// `Invalid` credential status rather than dispatching to a provider that
/// would just reject the request with 401.
pub async fn fetch_coding_plan_quota(
    base_url: &str,
    api_key: &str,
) -> crate::types::subscription::SubscriptionQuota {
    use crate::provider::coding_plan::subscription::now_millis;
    use crate::types::subscription::{CredentialStatus, SubscriptionQuota};

    let now_ms = now_millis();

    // ponytail: empty-key short-circuit is one shared guard here, not repeated
    // in every provider. Matches cc-switch::get_coding_plan_quota pre-validation.
    if api_key.trim().is_empty() {
        return SubscriptionQuota {
            provider_id: "unknown".into(),
            credential_status: CredentialStatus::Invalid,
            credential_message: Some("API key is empty".into()),
            success: false,
            tiers: vec![],
            error: Some("API key is empty".into()),
            queried_at_ms: now_ms,
        };
    }

    if detect_provider(base_url).is_none() {
        return SubscriptionQuota {
            provider_id: "unknown".into(),
            credential_status: CredentialStatus::Unknown,
            credential_message: Some(format!("Unrecognized coding plan base_url: {base_url}")),
            success: false,
            tiers: vec![],
            error: Some("Provider not detected".into()),
            queried_at_ms: now_ms,
        };
    }

    // ponytail: single dispatch table. Adding a provider = (1) write the
    // module, (2) declare it at top of this file, (3) add the arm here.
    // The provider is re-detected below so the typed variant can flow
    // straight into the arm — a single `detect_provider` call would force
    // duplicating the dispatch table or threading the raw `&str` through.
    let provider = detect_provider(base_url)
        .expect("detect_provider returned Some above; this branch is unreachable");

    match provider {
        CodingPlanProvider::MiniMaxCn | CodingPlanProvider::MiniMaxEn => {
            crate::provider::coding_plan::minimax::fetch(base_url, api_key).await
        }
        CodingPlanProvider::Kimi => {
            crate::provider::coding_plan::kimi::fetch(base_url, api_key).await
        }
        CodingPlanProvider::ZhipuCn => {
            crate::provider::coding_plan::zhipu_cn::fetch(base_url, api_key).await
        }
        CodingPlanProvider::ZhipuEn => {
            crate::provider::coding_plan::zhipu_en::fetch(base_url, api_key).await
        }
        CodingPlanProvider::Volcengine => {
            crate::provider::coding_plan::volcengine::fetch(base_url, api_key).await
        }
        CodingPlanProvider::ZenMux => {
            crate::provider::coding_plan::zenmux::fetch(base_url, api_key).await
        }
    }
}