pub mod adapter;
pub mod openai;
pub mod deepseek;
pub mod anthropic;
pub mod github;
pub mod coding_plan;


pub use adapter::{ProviderAdapter, ValidateError, QuotaError, adapter_for, coding_plan_adapter_for};
pub use coding_plan::{CodingPlanProvider, detect_provider, fetch_coding_plan_quota};