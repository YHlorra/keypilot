// src-tauri/src/actions/system.rs
// System domain actions — get_theme, set_theme, quit.

use super::ActionDef;
use serde_json::json;

pub fn actions() -> Vec<ActionDef> {
    vec![
        ActionDef {
            id: "system.get_theme".into(),
            name: "Get Theme".into(),
            description: "Return the current theme setting (dark | light | auto).".into(),
            category: "system".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            output_schema: json!({
                "type": "string",
                "enum": ["dark", "light", "auto"]
            }),
        },
        ActionDef {
            id: "system.set_theme".into(),
            name: "Set Theme".into(),
            description: "Set the theme (dark | light | auto).".into(),
            category: "system".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "theme": { "type": "string", "enum": ["dark", "light", "auto"] }
                },
                "required": ["theme"]
            }),
            output_schema: json!({ "type": "null" }),
        },
        ActionDef {
            id: "system.quit".into(),
            name: "Quit App".into(),
            description: "Quit the application. Exits the process.".into(),
            category: "system".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            output_schema: json!({ "type": "null" }),
        },
    ]
}
