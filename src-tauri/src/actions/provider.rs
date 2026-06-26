// src-tauri/src/actions/provider.rs
// Provider domain actions — list, get, add, update, delete, test_connection.

use super::ActionDef;
use serde_json::json;

pub fn actions() -> Vec<ActionDef> {
    vec![
        ActionDef {
            id: "provider.list".into(),
            name: "List Providers".into(),
            description: "Return all providers with their fields inline.".into(),
            category: "provider".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            output_schema: json!({
                "type": "array",
                "items": { "$ref": "#/components/schemas/Provider" }
            }),
        },
        ActionDef {
            id: "provider.get".into(),
            name: "Get Provider".into(),
            description: "Return a single provider by id, including its fields.".into(),
            category: "provider".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "id": { "type": "integer", "description": "Provider id" } },
                "required": ["id"]
            }),
            output_schema: json!({ "$ref": "#/components/schemas/Provider" }),
        },
        ActionDef {
            id: "provider.add".into(),
            name: "Add Provider".into(),
            description: "Create a new provider with optional initial fields.".into(),
            category: "provider".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "preset": { "type": ["string", "null"], "description": "Preset name or null for Custom" },
                    "category_id": { "type": "integer" },
                    "pinned": { "type": "boolean" },
                    "notes": { "type": ["string", "null"] },
                    "icon": { "type": ["string", "null"] },
                    "icon_color": { "type": ["string", "null"] },
                    "fields": {
                        "type": "array",
                        "items": { "$ref": "#/components/schemas/ProviderFieldInput" }
                    }
                },
                "required": ["name", "category_id"]
            }),
            output_schema: json!({ "$ref": "#/components/schemas/Provider" }),
        },
        ActionDef {
            id: "provider.update".into(),
            name: "Update Provider".into(),
            description: "Update an existing provider. Fields array (if provided) replaces all existing fields.".into(),
            category: "provider".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer" },
                    "name": { "type": "string" },
                    "category_id": { "type": "integer" },
                    "pinned": { "type": "boolean" },
                    "notes": { "type": ["string", "null"] },
                    "icon": { "type": ["string", "null"] },
                    "icon_color": { "type": ["string", "null"] },
                    "fields": {
                        "type": "array",
                        "items": { "$ref": "#/components/schemas/ProviderFieldInput" }
                    }
                },
                "required": ["id"]
            }),
            output_schema: json!({ "$ref": "#/components/schemas/Provider" }),
        },
        ActionDef {
            id: "provider.delete".into(),
            name: "Delete Provider".into(),
            description: "Delete a provider and all its fields (CASCADE).".into(),
            category: "provider".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "id": { "type": "integer" } },
                "required": ["id"]
            }),
            output_schema: json!({ "type": "null" }),
        },
        ActionDef {
            id: "provider.test_connection".into(),
            name: "Test Connection".into(),
            description: "Test the API key/connection for a provider. Only enabled for OpenAI, DeepSeek, and Anthropic presets.".into(),
            category: "provider".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "id": { "type": "integer" } },
                "required": ["id"]
            }),
            output_schema: json!({ "type": "null" }),
        },
    ]
}
