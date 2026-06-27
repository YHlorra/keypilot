// src-tauri/src/actions/token_usage.rs
// Token Usage domain actions — record, list, summary, import, pricing.

use super::ActionDef;
use serde_json::json;

pub fn actions() -> Vec<ActionDef> {
    vec![
        ActionDef {
            id: "token_usage.record".into(),
            name: "Record Usage".into(),
            description: "Record a single token usage row. Computes cost from pricing. Idempotent via deterministic id (idempotency key derived from content hash).".into(),
            category: "token_usage".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "occurred_at": { "type": "string", "description": "ISO 8601 timestamp" },
                    "finished_at": { "type": "string" },
                    "latency_ms": { "type": "integer" },
                    "provider": { "type": "string" },
                    "model": { "type": "string" },
                    "agent_type": { "type": "string" },
                    "user_id": { "type": "string" },
                    "session_id": { "type": "string" },
                    "observation_type": { "type": "string" },
                    "status": { "type": "string" },
                    "error_code": { "type": "string" },
                    "cache_hit": { "type": "integer" },
                    "usage_details": {
                        "type": "object",
                        "properties": {
                            "input": { "type": "integer" },
                            "output": { "type": "integer" },
                            "cache_read": { "type": "integer" },
                            "cache_creation": { "type": "integer" },
                            "reasoning": { "type": "integer" }
                        }
                    },
                    "cost_details": { "type": "object" },
                    "pricing_version": { "type": "string" },
                    "messages": { "type": "string" },
                    "response": { "type": "string" },
                    "tags": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["occurred_at", "provider", "model", "usage_details"]
            }),
            output_schema: json!({ "$ref": "#/components/schemas/UsageRecord" }),
        },
        ActionDef {
            id: "token_usage.list".into(),
            name: "List Usage Records".into(),
            description: "List token usage records with filter + pagination.".into(),
            category: "token_usage".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "filter": { "$ref": "#/components/schemas/UsageFilter" },
                    "page": { "type": "integer", "minimum": 1, "default": 1 },
                    "per_page": { "type": "integer", "minimum": 1, "maximum": 200, "default": 50 }
                },
                "required": ["filter", "page", "per_page"]
            }),
            output_schema: json!({ "$ref": "#/components/schemas/PaginatedUsageRecord" }),
        },
        ActionDef {
            id: "token_usage.summary".into(),
            name: "Get Usage Summary".into(),
            description: "Get aggregated usage summary (agent pairs + daily series) for a date range.".into(),
            category: "token_usage".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "start_date": { "type": "string", "format": "date" },
                    "end_date": { "type": "string", "format": "date" },
                    "agent_type": { "type": "string" },
                    "model": { "type": "string" },
                    "provider": { "type": "string" },
                    "status": { "type": "string" }
                }
            }),
            output_schema: json!({ "$ref": "#/components/schemas/UsageSummary" }),
        },
        ActionDef {
            id: "token_usage.import".into(),
            name: "Import Usage".into(),
            description: "Batch import usage from JSONL or CSV content. Returns counts of imported/skipped rows + per-line errors.".into(),
            category: "token_usage".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "content": { "type": "string" },
                    "format": { "type": "string", "enum": ["jsonl", "csv"] },
                    "source_hint": { "type": "string" }
                },
                "required": ["content", "format"]
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "imported": { "type": "integer" },
                    "skipped": { "type": "integer" },
                    "errors": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "line": { "type": "integer" },
                                "message": { "type": "string" }
                            }
                        }
                    }
                }
            }),
        },
        ActionDef {
            id: "token_usage.pricing".into(),
            name: "Get Pricing".into(),
            description: "Return the full pricing table (Top 50 models).".into(),
            category: "token_usage".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            output_schema: json!({
                "type": "array",
                "items": { "$ref": "#/components/schemas/PricingEntry" }
            }),
        },
        ActionDef {
            id: "token_usage.import_opencode_db".into(),
            name: "Import opencode.db".into(),
            description: "Import token usage from an opencode.db SQLite file (READ ONLY). Reads the session table and feeds each row through record_usage so existing FNV-1a dedup applies.".into(),
            category: "token_usage".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "db_path": {
                        "type": "string",
                        "description": "Absolute path to opencode.db (READ ONLY). Table 'session' must exist with columns: id, model, cost, tokens_input, tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write, time_created."
                    }
                },
                "required": ["db_path"]
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "imported": { "type": "integer" },
                    "skipped": { "type": "integer" },
                    "errors": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "line": { "type": "integer" },
                                "message": { "type": "string" }
                            }
                        }
                    }
                }
            }),
        },
    ]
}
