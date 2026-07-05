// provider/coding_plan/registry.rs — typed VendorId match for coding plan quota
// Replaces detect_provider() 8-string-contains with typed routing.

use crate::catalog::VendorId;
use crate::types::subscription::SubscriptionQuota;
use super::{kimi, mimo, minimax, volcengine, zenmux, zhipu_cn, zhipu_en};

/// Routes a coding_plan quota fetch by VendorId.
pub async fn fetch(vendor: VendorId, base_url: &str, api_key: &str) -> SubscriptionQuota {
    match vendor {
        VendorId::Kimi       => kimi::fetch(base_url, api_key).await,
        VendorId::Mimo       => mimo::fetch(base_url, api_key).await,
        VendorId::MinimaxCn  => minimax::fetch(base_url, api_key).await,
        VendorId::MinimaxEn  => minimax::fetch(base_url, api_key).await,
        VendorId::ZhipuCn    => zhipu_cn::fetch(base_url, api_key).await,
        VendorId::ZhipuEn    => zhipu_en::fetch(base_url, api_key).await,
        VendorId::Volcengine => volcengine::fetch(base_url, api_key).await,
        VendorId::ZenMux     => zenmux::fetch(base_url, api_key).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog;

    #[test]
    fn typed_match_kimi_resolves_via_catalog() {
        // Verify catalog's kimi toml declares coding_plan = "kimi"
        let resolved = catalog::resolve("kimi", None).expect("kimi in catalog");
        assert_eq!(resolved.coding_plan, Some(VendorId::Kimi));
    }

    #[test]
    fn typed_match_minimax_en_has_minimax_en() {
        let resolved = catalog::resolve("minimax-en", None).expect("in catalog");
        assert_eq!(resolved.coding_plan, Some(VendorId::MinimaxEn));
    }

    #[test]
    fn typed_match_openai_has_no_coding_plan() {
        let resolved = catalog::resolve("openai", None).expect("in catalog");
        assert_eq!(resolved.coding_plan, None);
    }
}
