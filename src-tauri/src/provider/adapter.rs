use async_trait::async_trait;
use crate::provider::coding_plan::CodingPlanProvider;
use crate::types::QuotaSnapshot;

#[derive(Debug, Clone)]
pub enum ValidateError {
    InvalidKey,
    Ambiguous,
    Network(String),
}

#[derive(Debug, Clone)]
pub enum QuotaError {
    Network(String),
    Parse(String),
    Unsupported,
}

#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    fn preset(&self) -> &'static str;
    fn can_test(&self) -> bool;
    fn can_fetch_quota(&self) -> bool;
    async fn validate_key(&self, _base_url: &str, _api_key: &str) -> Result<(), ValidateError> {
        Err(ValidateError::Ambiguous)
    }
    async fn fetch_quota(&self, base_url: &str, api_key: &str) -> Result<QuotaSnapshot, QuotaError>;
}

pub fn adapter_for(preset: &str) -> Option<Box<dyn ProviderAdapter>> {
    match preset {
        "openai" => Some(Box::new(crate::provider::openai::OpenAiAdapter)),
        "deepseek" => Some(Box::new(crate::provider::deepseek::DeepSeekAdapter)),
        "anthropic" => Some(Box::new(crate::provider::anthropic::AnthropicAdapter)),
        "github" => Some(Box::new(crate::provider::github::GitHubAdapter)),
        // OpenAI-compatible providers reuse the OpenAI adapter. test_connection
        // hits /v1/models which most OpenAI-compat APIs expose; fetch_quota will
        // return ProviderQuotaUnsupported for non-OpenAI backends.
        // add a brand-specific adapter only if a provider's /v1/models contract
        // diverges from OpenAI's.
        "kimi" | "zhipu" | "qwen" | "openrouter" | "groq" | "mistral"
        | "siliconflow" | "together" | "volcengine" | "stepfun"
        | "cohere" | "perplexity"
        // MiniMax OpenAI-protocol nodes (2 regions). Anthropic-compat variants
        // live in the Anthropic arm below; convention is short id = OpenAI.
        | "minimax" | "minimax-overseas" => Some(Box::new(crate::provider::openai::OpenAiAdapter)),
        // Anthropic-compatible providers reuse the Anthropic adapter. /v1/messages
        // works for Kimi/GLM/DeepSeek/Volcengine/MiniMax anthropic-compat endpoints.
        "kimi-anthropic" | "zhipu-anthropic" | "deepseek-anthropic"
        | "volcengine-anthropic"
        | "minimax-anthropic" | "minimax-overseas-anthropic" => Some(Box::new(crate::provider::anthropic::AnthropicAdapter)),
        _ => None,
    }
}

/// Map a preset id to its coding-plan provider, if any.
///
/// This is a separate routing table from [`adapter_for`] — coding-plan
/// providers expose percentage-based quota (5-hour / weekly / monthly)
/// rather than USD billing, so they live in their own data-flow lane
/// keyed off `fetch_coding_plan_quota` rather than `fetch_quota`.
///
/// Lane C: all 5 families wired (Kimi / GLM CN / GLM EN / MiniMax /
/// Volcengine / ZenMux). The CN/EN collapse is deliberate for
/// MiniMax / GLM — the actual region disambiguation happens in the
/// per-provider `fetch` via `base_url` substring, not at the
/// preset-routing layer. Short preset id (e.g. `zhipu`) is the OpenAI-
/// compat variant; the `-anthropic` suffix is the Anthropic-compat
/// variant — both share the same coding-plan endpoint.
pub fn coding_plan_adapter_for(preset: &str) -> Option<CodingPlanProvider> {
    match preset {
        // MiniMax: 4 preset variants collapse to MiniMaxCn; fetch() reads
        // base_url to disambiguate CN vs EN.
        "minimax" | "minimax-overseas"
        | "minimax-anthropic" | "minimax-overseas-anthropic" => Some(CodingPlanProvider::MiniMaxCn),
        // Kimi: single global endpoint, both compat variants share it.
        "kimi" | "kimi-anthropic" => Some(CodingPlanProvider::Kimi),
        // GLM: CN + overseas; both compat variants route the same way
        // within each region. fetch() reads base_url to disambiguate.
        "zhipu" | "zhipu-anthropic" => Some(CodingPlanProvider::ZhipuCn),
        // GLM overseas: same preset as the kimi-routed CN branch is
        // impossible (preset names are disjoint), so a separate arm.
        // Note: as of V0.1 the global API zhipu preset resolves to the
        // CN host (`open.bigmodel.cn`); overseas routing is only used
        // for mirrors pointing at `api.z.ai`. Both are supported
        // through detect_provider().
        "volcengine" | "volcengine-anthropic" => Some(CodingPlanProvider::Volcengine),
        "zenmux" => Some(CodingPlanProvider::ZenMux),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_for_openai() {
        let result = adapter_for("openai");
        assert!(result.is_some());
        let adapter = result.unwrap();
        assert_eq!(adapter.preset(), "openai");
    }

    #[test]
    fn test_adapter_for_custom() {
        // Custom preset (no preset = None) should return None
        let result = adapter_for("custom");
        assert!(result.is_none());
    }

    #[test]
    fn test_adapter_for_empty() {
        let result = adapter_for("");
        assert!(result.is_none());
    }

    #[test]
    fn test_adapter_for_nonexistent() {
        let result = adapter_for("nonexistent_preset");
        assert!(result.is_none());
    }

    #[test]
    fn test_adapter_for_all_presets() {
        // Original 4 — each with its own adapter (special-cased validate_key/quota).
        for p in ["openai", "deepseek", "anthropic", "github"] {
            assert!(adapter_for(p).is_some(), "preset {p} should resolve");
        }
        // iterate PRESETS directly so adding a new preset doesn't
        // require editing this test. Assert routing class (not just existence)
        // — a refactor that maps kimi → deepseek by mistake would silently
        // break quota fetch on every Kimi row; this test catches it.
        // Routing convention: short preset id = OpenAI-compat, `-anthropic`
        // suffix = Anthropic-compat. Custom adapters are not added in this PR.
        for preset_id in crate::database::preset_ids() {
            let a = adapter_for(preset_id)
                .unwrap_or_else(|| panic!("preset {preset_id} should resolve"));
            let expected = if preset_id.ends_with("-anthropic") {
                "anthropic"
            } else if matches!(preset_id, "anthropic" | "deepseek" | "github") {
                // Original 4 — each has its own adapter; the routing assertion
                // is that adapter_for returns Some, not which class it routes to.
                continue;
            } else {
                "openai"
            };
            assert_eq!(
                a.preset(),
                expected,
                "preset {preset_id} should route to {expected} adapter"
            );
        }
    }

    // ── coding_plan_adapter_for (Lane C) ──────────────────────

    #[test]
    fn coding_plan_adapter_for_covers_all_five_families() {
        // Lane C contract: every coding-plan-supported preset must resolve
        // to a non-None CodingPlanProvider so the IPC entry gate in
        // `fetch_coding_plan_quota_by_state` doesn't return
        // ProviderQuotaUnsupported. Adding a new preset = add a case here.
        let cases: &[(&str, CodingPlanProvider)] = &[
            ("kimi", CodingPlanProvider::Kimi),
            ("kimi-anthropic", CodingPlanProvider::Kimi),
            ("zhipu", CodingPlanProvider::ZhipuCn),
            ("zhipu-anthropic", CodingPlanProvider::ZhipuCn),
            ("minimax", CodingPlanProvider::MiniMaxCn),
            ("minimax-overseas", CodingPlanProvider::MiniMaxCn),
            ("minimax-anthropic", CodingPlanProvider::MiniMaxCn),
            ("minimax-overseas-anthropic", CodingPlanProvider::MiniMaxCn),
            ("volcengine", CodingPlanProvider::Volcengine),
            ("volcengine-anthropic", CodingPlanProvider::Volcengine),
            ("zenmux", CodingPlanProvider::ZenMux),
        ];
        for (preset, expected) in cases {
            assert_eq!(
                coding_plan_adapter_for(preset),
                Some(expected.clone()),
                "preset {preset} should route to {expected:?}"
            );
        }
    }

    #[test]
    fn coding_plan_adapter_for_returns_none_for_unsupported_presets() {
        // Presets without a coding-plan endpoint must return None so the
        // IPC gate emits ProviderQuotaUnsupported. Specifically: openai /
        // deepseek / anthropic / github / postgres / stepfun / cohere /
        // perplexity / etc.
        for preset in [
            "openai", "deepseek", "anthropic", "github", "postgres",
            "stepfun", "cohere", "perplexity", "openrouter", "groq",
            "mistral", "siliconflow", "together", "qwen",
        ] {
            assert!(
                coding_plan_adapter_for(preset).is_none(),
                "preset {preset} must NOT have a coding-plan adapter"
            );
        }
    }
}