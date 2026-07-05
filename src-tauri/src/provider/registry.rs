// provider/registry.rs — typed ProtocolAdapter registry (REQ-CAT-005)
// Replaces adapter_for() string-switch with typed match on ProtocolId.

use async_trait::async_trait;
use crate::catalog::{ProtocolId, ResolvedProvider};
use crate::types::QuotaSnapshot;
use super::protocols::{openai::OpenAiProtocol, anthropic::AnthropicProtocol,
                       github::GitHubProtocol, balance::BalanceProtocol,
                       deepseek::DeepSeekProtocol};

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
pub trait ProtocolAdapter: Send + Sync {
    fn id(&self) -> &'static str;
    async fn validate(&self, resolved: &ResolvedProvider, api_key: &str) -> Result<(), ValidateError>;
    async fn fetch_quota(&self, resolved: &ResolvedProvider, api_key: &str) -> Result<QuotaSnapshot, QuotaError>;
}

pub fn adapter_for(p: ProtocolId) -> &'static dyn ProtocolAdapter {
    match p {
        ProtocolId::Openai    => &OpenAiProtocol,
        ProtocolId::Anthropic => &AnthropicProtocol,
        ProtocolId::Github    => &GitHubProtocol,
        ProtocolId::Balance   => &BalanceProtocol,
        ProtocolId::Deepseek  => &DeepSeekProtocol,
    }
}

/// Render the auth_header template by substituting {api_key}.
pub fn render_auth_header(resolved: &ResolvedProvider, api_key: &str) -> String {
    resolved.auth_header.replace("{api_key}", api_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::ProtocolId;

    #[test]
    fn adapter_for_openai() {
        let a = adapter_for(ProtocolId::Openai);
        assert_eq!(a.id(), "openai");
    }

    #[test]
    fn adapter_for_anthropic() {
        assert_eq!(adapter_for(ProtocolId::Anthropic).id(), "anthropic");
    }

    #[test]
    fn adapter_for_github() {
        assert_eq!(adapter_for(ProtocolId::Github).id(), "github");
    }

    #[test]
    fn adapter_for_balance() {
        assert_eq!(adapter_for(ProtocolId::Balance).id(), "balance");
    }

    #[test]
    fn adapter_for_deepseek() {
        assert_eq!(adapter_for(ProtocolId::Deepseek).id(), "deepseek");
    }

    #[test]
    fn all_protocol_ids_have_adapter() {
        for p in [
            ProtocolId::Openai,
            ProtocolId::Anthropic,
            ProtocolId::Github,
            ProtocolId::Balance,
            ProtocolId::Deepseek,
        ] {
            let a = adapter_for(p);
            assert!(!a.id().is_empty(), "protocol {:?} should have non-empty id", p);
        }
    }
}
