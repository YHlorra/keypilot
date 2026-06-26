use std::collections::HashMap;
use std::sync::Arc;
use once_cell::sync::Lazy;
use serde::Deserialize;
use crate::types::{PricingEntry, TokenCounts, TokenUsageCostBreakdown};
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

    pub fn lookup(&self, model: &str) -> Option<Arc<PricingEntry>> {
        self.model_index.get(model).cloned()
    }

    pub fn version(&self) -> &str {
        &PRICING_DATA.version
    }

    pub fn calculate_cost(&self, entry: &PricingEntry, tokens: &TokenCounts) -> f64 {
        let mut total = 0.0;
        if let Some(rate) = entry.input_price_per_1m {
            total += rate * tokens.input as f64 / 1_000_000.0;
        }
        if let Some(rate) = entry.output_price_per_1m {
            total += rate * tokens.output as f64 / 1_000_000.0;
        }
        if let Some(rate) = entry.cache_read_price_per_1m {
            total += rate * tokens.cache_read as f64 / 1_000_000.0;
        }
        if let Some(rate) = entry.cache_creation_price_per_1m {
            total += rate * tokens.cache_creation as f64 / 1_000_000.0;
        }
        if let Some(rate) = entry.reasoning_price_per_1m {
            total += rate * tokens.reasoning as f64 / 1_000_000.0;
        }
        total
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
