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
        
        
        
        
        
        "kimi" | "zhipu" | "qwen" | "openrouter" | "groq" | "mistral"
        | "siliconflow" | "together" | "volcengine" | "stepfun"
        | "cohere" | "perplexity"
        
        
        | "minimax" | "minimax-overseas" => Some(Box::new(crate::provider::openai::OpenAiAdapter)),
        
        
        "kimi-anthropic" | "zhipu-anthropic" | "deepseek-anthropic"
        | "volcengine-anthropic"
        | "minimax-anthropic" | "minimax-overseas-anthropic" => Some(Box::new(crate::provider::anthropic::AnthropicAdapter)),
        _ => None,
    }
}















pub fn coding_plan_adapter_for(preset: &str) -> Option<CodingPlanProvider> {
    match preset {
        
        
        "minimax" | "minimax-overseas"
        | "minimax-anthropic" | "minimax-overseas-anthropic" => Some(CodingPlanProvider::MiniMaxCn),
        
        "kimi" | "kimi-anthropic" => Some(CodingPlanProvider::Kimi),
        
        
        "zhipu" | "zhipu-anthropic" => Some(CodingPlanProvider::ZhipuCn),
        
        
        
        
        
        
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
        
        for p in ["openai", "deepseek", "anthropic", "github"] {
            assert!(adapter_for(p).is_some(), "preset {p} should resolve");
        }
        
        
        
        
        
        
        for preset_id in crate::database::preset_ids() {
            let a = adapter_for(preset_id)
                .unwrap_or_else(|| panic!("preset {preset_id} should resolve"));
            let expected = if preset_id.ends_with("-anthropic") {
                "anthropic"
            } else if matches!(preset_id, "anthropic" | "deepseek" | "github") {
                
                
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

    

    #[test]
    fn coding_plan_adapter_for_covers_all_five_families() {
        
        
        
        
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