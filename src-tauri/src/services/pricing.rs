use std::collections::HashMap;
use std::sync::Arc;
use once_cell::sync::Lazy;
use serde::Deserialize;
use crate::types::{PricingEntry, TokenUsageCostBreakdown};
use crate::error::AppError;

#[derive(Debug, Clone, Deserialize)]
struct PricingData {
    version: String,
    models: Vec<PricingEntry>,
}

static PRICING_DATA: Lazy<PricingData> = Lazy::new(|| {
    let raw = include_str!("../../data/pricing.json");
    serde_json::from_str(raw).expect("Invalid pricing.json")
});

pub struct PricingService {
    model_index: HashMap<String, Arc<PricingEntry>>,
}

impl PricingService {
    pub fn new() -> Self {
        let mut model_index = HashMap::new();
        for entry in &PRICING_DATA.models {
            model_index.insert(entry.model.clone(), Arc::new(entry.clone()));
        }
        Self { model_index }
    }

    /// Build a PricingService from an explicit model list (test-only entry point).
    /// `version()` still reads the bundled pricing.json version, since pricing
    /// version is a property of the shipping data, not the in-memory index.
    pub(crate) fn from_models(models: Vec<PricingEntry>) -> Self {
        let mut model_index = HashMap::new();
        for entry in models {
            model_index.insert(entry.model.clone(), Arc::new(entry));
        }
        Self { model_index }
    }

    pub fn lookup(&self, model: &str) -> Option<Arc<PricingEntry>> {
        self.model_index.get(model).cloned()
    }

    /// 通过模型名查找对应的 provider 字符串(例如 gpt-4o → "OpenAI")。
    /// 模型不在 pricing.json 时返回 None。
    pub fn lookup_provider_by_model(&self, model: &str) -> Option<String> {
        self.model_index.get(model).map(|entry| entry.provider.clone())
    }

    pub fn version(&self) -> &str {
        &PRICING_DATA.version
    }

    pub fn all_entries(&self) -> Vec<&PricingEntry> {
        self.model_index.values().map(|arc| arc.as_ref()).collect()
    }

    pub fn calculate_token_usage_cost(
        &self,
        model: &str,
        input_tokens: i64,
        output_tokens: i64,
        cache_read_tokens: i64,
        cache_creation_tokens: i64,
        reasoning_tokens: i64,
    ) -> Result<TokenUsageCostBreakdown, AppError> {
        let Some(entry) = self.lookup(model) else {
            return Ok(TokenUsageCostBreakdown {
                input_cost: 0.0,
                output_cost: 0.0,
                cache_read_cost: 0.0,
                cache_creation_cost: 0.0,
                reasoning_cost: 0.0,
                total_cost: 0.0,
                currency: "USD".into(),
                pricing_missing_for: Some(model.to_string()),
            });
        };
        let input_cost = entry
            .input_price_per_1m
            .map(|r| r * input_tokens as f64 / 1_000_000.0)
            .unwrap_or(0.0);
        let output_cost = entry
            .output_price_per_1m
            .map(|r| r * output_tokens as f64 / 1_000_000.0)
            .unwrap_or(0.0);
        let cache_read_cost = entry
            .cache_read_price_per_1m
            .map(|r| r * cache_read_tokens as f64 / 1_000_000.0)
            .unwrap_or(0.0);
        let cache_creation_cost = entry
            .cache_creation_price_per_1m
            .map(|r| r * cache_creation_tokens as f64 / 1_000_000.0)
            .unwrap_or(0.0);
        let reasoning_cost = entry
            .reasoning_price_per_1m
            .map(|r| r * reasoning_tokens as f64 / 1_000_000.0)
            .unwrap_or(0.0);
        let total_cost =
            input_cost + output_cost + cache_read_cost + cache_creation_cost + reasoning_cost;
        Ok(TokenUsageCostBreakdown {
            input_cost,
            output_cost,
            cache_read_cost,
            cache_creation_cost,
            reasoning_cost,
            total_cost,
            currency: "USD".into(),
            pricing_missing_for: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_provider_by_model_returns_provider_for_known_model() {
        let svc = PricingService::new();

        let provider = svc.lookup_provider_by_model("gpt-4o");
        assert_eq!(provider.as_deref(), Some("OpenAI"));
    }

    #[test]
    fn lookup_provider_by_model_returns_none_for_unknown() {
        let svc = PricingService::new();

        let provider = svc.lookup_provider_by_model("totally-unknown-xyz-123");
        assert!(provider.is_none(), "expected None for unknown model");
    }
}
