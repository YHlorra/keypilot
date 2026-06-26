// src-tauri/src/actions/category.rs
// Category domain actions — list, add, delete.

use super::ActionDef;
use serde_json::json;

pub fn actions() -> Vec<ActionDef> {
    vec![
        ActionDef {
            id: "category.list".into(),
            name: "List Categories".into(),
            description: "Return all categories ordered by sort_index.".into(),
            category: "category".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            output_schema: json!({
                "type": "array",
                "items": { "$ref": "#/components/schemas/Category" }
            }),
        },
        ActionDef {
            id: "category.add".into(),
            name: "Add Category".into(),
            description: "Create a new category. Default categories cannot be created (V0.1 seed only).".into(),
            category: "category".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "name": { "type": "string" } },
                "required": ["name"]
            }),
            output_schema: json!({ "$ref": "#/components/schemas/Category" }),
        },
        ActionDef {
            id: "category.delete".into(),
            name: "Delete Category".into(),
            description: "Delete a category. Providers in this category are migrated to migrate_to. Default categories cannot be deleted.".into(),
            category: "category".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Category to delete" },
                    "migrate_to": { "type": "integer", "description": "Category id to receive the deleted category's providers" }
                },
                "required": ["id", "migrate_to"]
            }),
            output_schema: json!({ "type": "null" }),
        },
    ]
}
