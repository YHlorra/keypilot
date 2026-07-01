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
        assert!(adapter_for("openai").is_some());
        assert!(adapter_for("deepseek").is_some());
        assert!(adapter_for("anthropic").is_some());
        assert!(adapter_for("github").is_some());
    }
}