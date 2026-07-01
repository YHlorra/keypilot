pub mod adapter;
pub mod openai;
pub mod deepseek;
pub mod anthropic;
pub mod github;


pub use adapter::{ProviderAdapter, ValidateError, QuotaError, adapter_for};