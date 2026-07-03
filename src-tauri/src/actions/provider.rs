


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
            id: "provider.open_for_edit".into(),
            name: "Open Provider For Edit".into(),
            description: "Fetch a provider by id and return its current state for the edit modal. Mirrors provider.get but semantically signals the caller is about to edit.".into(),
            category: "provider".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "id": { "type": "integer", "description": "Provider id" } },
                "required": ["id"]
            }),
            output_schema: json!({ "$ref": "#/components/schemas/Provider" }),
        },
        ActionDef {
            id: "provider.copy_credential".into(),
            name: "Copy Primary Credential".into(),
            description: "Return the value of the primary credential field (api_key by default, or first field if not present). For clipboard copy use case — caller's responsibility to write to clipboard.".into(),
            category: "provider".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Provider id" },
                    "field_key": { "type": ["string", "null"], "description": "Optional: override the field key. Defaults to 'api_key'." }
                },
                "required": ["id"]
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "value": { "type": "string", "description": "The credential value (plaintext, V0.1)" },
                    "field_key": { "type": "string", "description": "The actual field key resolved" }
                },
                "required": ["value", "field_key"]
            }),
        },
        ActionDef {
            id: "provider.test_and_refresh".into(),
            name: "Test Connection And Refresh Quota".into(),
            description: "Test the provider connection AND fetch the latest quota in one call. Multi-end invocation: runs test_connection_by_state then fetch_quota_by_state sequentially, returning both results.".into(),
            category: "provider".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "id": { "type": "integer" } },
                "required": ["id"]
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "test": { "type": "string", "description": "Test result status: 'ok' or error message" },
                    "quota": {
                        "oneOf": [
                            { "type": "null" },
                            { "$ref": "#/components/schemas/QuotaSnapshot" }
                        ]
                    }
                },
                "required": ["test", "quota"]
            }),
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
    ]
}
