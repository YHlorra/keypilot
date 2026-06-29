pub mod adapter;
pub mod openai;
pub mod deepseek;
pub mod anthropic;
pub mod github;
pub mod postgres;
pub mod agent_source;
pub mod agent_sources;

pub use adapter::{ProviderAdapter, ValidateError, QuotaError, adapter_for};