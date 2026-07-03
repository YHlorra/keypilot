


use super::ActionDef;
use serde_json::json;

pub fn actions() -> Vec<ActionDef> {
    vec![
        ActionDef {
            id: "quota.fetch".into(),
            name: "Fetch Quota".into(),
            description: "Fetch the quota snapshot for a provider. Returns Anthropic quota error for Anthropic preset. Uses 15-minute cache.".into(),
            category: "quota".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "id": { "type": "integer" } },
                "required": ["id"]
            }),
            output_schema: json!({ "$ref": "#/components/schemas/QuotaSnapshot" }),
        },
    ]
}
