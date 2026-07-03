









pub mod kimi;
pub mod minimax;
pub mod subscription;
pub mod volcengine;
pub mod zenmux;
pub mod zhipu_cn;
pub mod zhipu_en;

#[cfg(test)]
mod tests;







#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodingPlanProvider {
    
    MiniMaxCn,
    
    MiniMaxEn,
    
    Kimi,
    
    ZhipuCn,
    
    ZhipuEn,
    
    Volcengine,
    
    ZenMux,
}





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
        
        Some(CodingPlanProvider::Volcengine)
    } else if url.contains("zenmux") {
        Some(CodingPlanProvider::ZenMux)
    } else {
        None
    }
}










pub async fn fetch_coding_plan_quota(
    base_url: &str,
    api_key: &str,
) -> crate::types::subscription::SubscriptionQuota {
    use crate::provider::coding_plan::subscription::now_millis;
    use crate::types::subscription::{CredentialStatus, SubscriptionQuota};

    let now_ms = now_millis();

    
    
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