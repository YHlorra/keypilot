use async_trait::async_trait;
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
}