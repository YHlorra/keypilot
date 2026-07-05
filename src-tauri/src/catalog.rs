// Phase 1: catalog data layer (single file, REQ-CAT-016)
// public API: CatalogEntry, CustomSpec, ResolvedProvider, ProtocolId, ParserId,
//             VendorId, ProbeOverride, CatalogLoader, CatalogMerger, all_preset_ids

use serde::{Deserialize, Serialize};

// 5 variants per REQ-CAT-007
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ProtocolId {
    Openai,
    Anthropic,
    Github,
    Balance,
    Deepseek,
}

impl ProtocolId {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtocolId::Openai => "openai",
            ProtocolId::Anthropic => "anthropic",
            ProtocolId::Github => "github",
            ProtocolId::Balance => "balance",
            ProtocolId::Deepseek => "deepseek",
        }
    }
}

// 4 variants per REQ-CAT-008
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ParserId {
    OpenaiUsage,
    AnthropicUsage,
    Balance,
    None,
}

// 7 variants per REQ-CAT-015
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum VendorId {
    Kimi,
    Mimo,
    MinimaxCn,
    MinimaxEn,
    ZhipuCn,
    ZhipuEn,
    Volcengine,
    ZenMux,
}

// Probe spec shared by validate + quota
#[derive(Deserialize, Debug, Clone)]
pub struct ProbeSpec {
    pub path: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default = "default_success_status")]
    pub success_status: Vec<u16>,
    pub parser: ParserId,
}

fn default_method() -> String {
    "GET".to_string()
}

fn default_success_status() -> Vec<u16> {
    vec![200]
}

// ProbeOverride for custom_spec overrides
#[derive(Deserialize, Debug, Clone)]
pub struct ProbeOverride {
    pub path: String,
    pub parser: ParserId,
    #[serde(default)]
    pub method: Option<String>,
}

// toml schema structs
#[derive(Deserialize, Debug, Clone)]
pub struct CatalogMeta {
    pub id: String,
    pub name: String,
    pub protocol: ProtocolId,
    pub icon: String,
    #[serde(default)]
    pub coding_plan: Option<VendorId>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct EndpointSpec {
    pub default_base_url: String,
    #[serde(default)]
    pub docs_url: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AuthSpec {
    pub auth_header: String,
    #[serde(default = "default_key_field")]
    pub key_field: String,
}

fn default_key_field() -> String {
    "api_key".to_string()
}

// ExtraEndpoint for multi-protocol presets (REQ-CAT-019)
#[derive(Deserialize, Debug, Clone)]
pub struct ExtraEndpoint {
    pub protocol: ProtocolId,
    pub base_url: String,
    pub auth_header: String,
    pub validate_probe: ProbeSpec,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CatalogEntry {
    pub meta: CatalogMeta,
    pub endpoint: EndpointSpec,
    pub auth: AuthSpec,
    pub validate_probe: ProbeSpec,
    pub quota_probe: ProbeSpec,
    #[serde(default)]
    pub extras: Vec<ExtraEndpoint>,
}

// CustomSpec JSON (user-written) per REQ-CAT-003
#[derive(Deserialize, Debug, Clone)]
pub struct CustomSpec {
    pub protocol: ProtocolId,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub auth_header: Option<String>,
    #[serde(default)]
    pub validate: Option<ProbeOverride>,
    #[serde(default)]
    pub quota: Option<ProbeOverride>,
    #[serde(default)]
    pub notes: Option<String>,
}

// Runtime result of merge (catalog defaults + custom_spec overrides)
#[derive(Debug, Clone)]
pub struct ResolvedProvider {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub protocol: ProtocolId,
    pub base_url: String,
    pub auth_header: String,
    pub validate_probe: ProbeSpec,
    pub quota_probe: ProbeSpec,
    pub coding_plan: Option<VendorId>,
    pub extras: Vec<ExtraEndpoint>,
}

#[derive(Debug, thiserror::Error)]
pub enum MergerError {
    #[error("preset not found in catalog: {0}")]
    UnknownPreset(String),
    #[error("invalid custom_spec: {0}")]
    InvalidCustomSpec(String),
}

// CATALOG: compiled-in via include_str! (21 entries after Fireworks + AI21 added)
static CATALOG: once_cell::sync::Lazy<Vec<CatalogEntry>> = once_cell::sync::Lazy::new(|| {
    let entries = vec![
        include_str!("../data/catalog/openai.toml"),
        include_str!("../data/catalog/anthropic.toml"),
        include_str!("../data/catalog/deepseek.toml"),
        include_str!("../data/catalog/github.toml"),
        include_str!("../data/catalog/kimi.toml"),
        include_str!("../data/catalog/zhipu.toml"),
        include_str!("../data/catalog/qwen.toml"),
        include_str!("../data/catalog/openrouter.toml"),
        include_str!("../data/catalog/groq.toml"),
        include_str!("../data/catalog/mistral.toml"),
        include_str!("../data/catalog/siliconflow.toml"),
        include_str!("../data/catalog/together.toml"),
        include_str!("../data/catalog/volcengine.toml"),
        include_str!("../data/catalog/stepfun.toml"),
        include_str!("../data/catalog/cohere.toml"),
        include_str!("../data/catalog/perplexity.toml"),
        include_str!("../data/catalog/minimax-cn.toml"),
        include_str!("../data/catalog/minimax-en.toml"),
        include_str!("../data/catalog/mimo.toml"),
        include_str!("../data/catalog/fireworks.toml"),
        include_str!("../data/catalog/ai21.toml"),
    ];
    let mut out = Vec::with_capacity(21);
    for (i, s) in entries.iter().enumerate() {
        match toml::from_str::<CatalogEntry>(s) {
            Ok(e) => out.push(e),
            Err(e) => log::error!("catalog entry #{i} failed to parse: {e}"),
        }
    }
    out
});

// Cached static ids for all_preset_ids (leak-once, ~400 bytes total)
static PRESET_IDS: once_cell::sync::Lazy<Vec<&'static str>> = once_cell::sync::Lazy::new(|| {
    let mut ids: Vec<&'static str> = Vec::with_capacity(21);
    for entry in CATALOG.iter() {
        let s = entry.meta.id.clone();
        let leaked: &'static str = Box::leak(s.into_boxed_str());
        ids.push(leaked);
    }
    ids
});

/// Load all catalog entries (thin wrapper around the lazy static)
pub fn load_all() -> Vec<CatalogEntry> {
    CATALOG.clone()
}

/// Returns all preset ids as static strings (REQ-CAT-014)
pub fn all_preset_ids() -> Vec<&'static str> {
    PRESET_IDS.clone()
}

/// Returns catalog metadata for the frontend "add credential" modal.
/// This is the single source of truth for available presets — no hardcoded list in UI.
#[derive(Serialize, Clone)]
pub struct CatalogExtraPresetMeta {
    pub protocol: String,
    pub base_url: String,
}

#[derive(Serialize)]
pub struct CatalogPresetMeta {
    pub id: String,
    pub name: String,
    pub protocol: String,
    pub icon: Option<String>,
    pub coding_plan: Option<String>,
    pub default_base_url: String,
    pub docs_url: Option<String>,
    pub key_field: String,
    /// Secondary protocol endpoints sharing one api_key (V0.2.1 multi-endpoint catalog).
    /// Empty for single-protocol presets.
    pub extras: Vec<CatalogExtraPresetMeta>,
}

pub fn list_catalog_presets() -> Vec<CatalogPresetMeta> {
    CATALOG
        .iter()
        .map(|entry| CatalogPresetMeta {
            id: entry.meta.id.clone(),
            name: entry.meta.name.clone(),
            protocol: entry.meta.protocol.as_str().to_string(),
            icon: Some(entry.meta.icon.clone()),
            coding_plan: entry.meta.coding_plan.map(|v| format!("{:?}", v).to_lowercase()),
            default_base_url: entry.endpoint.default_base_url.clone(),
            docs_url: entry.endpoint.docs_url.clone(),
            key_field: entry.auth.key_field.clone(),
            extras: entry
                .extras
                .iter()
                .map(|e| CatalogExtraPresetMeta {
                    protocol: e.protocol.as_str().to_string(),
                    base_url: e.base_url.clone(),
                })
                .collect(),
        })
        .collect()
}

/// Look up icon path for a preset id (used by add_provider to auto-fill icon column
/// when the frontend omits it). Returns None if preset is unknown.
pub fn preset_icon(preset_id: &str) -> Option<String> {
    CATALOG
        .iter()
        .find(|e| e.meta.id == preset_id)
        .map(|e| e.meta.icon.clone())
}

/// Resolve a preset_id + optional custom_spec into a ResolvedProvider
pub fn resolve(
    preset_id: &str,
    custom_spec: Option<&CustomSpec>,
) -> Result<ResolvedProvider, MergerError> {
    // 1. Look up preset in catalog
    let entry = CATALOG
        .iter()
        .find(|e| e.meta.id == preset_id)
        .ok_or_else(|| MergerError::UnknownPreset(preset_id.to_string()))?;

    // 2. Build base resolved provider from catalog entry, then apply overrides.
    let mut resolved = base_from_entry(entry);
    if let Some(cs) = custom_spec {
        apply_custom_spec(&mut resolved, cs);
    }
    Ok(resolved)
}

/// Resolve a bare custom_spec (no preset_id) into a ResolvedProvider.
/// Used by Phase 3 `provider.preflight` action to validate an unsaved form before submission.
///
/// Probes default per protocol when not provided in custom_spec (Phase 3 dialog only sends
/// `{ protocol, base_url, auth_header, notes }`):
/// - openai: validate=`/models`, quota=`/dashboard/billing/subscription`
/// - anthropic: validate=`/v1/messages`, quota=`/v1/messages` (V0.2 doesn't implement)
/// - github: validate=`/rate_limit`, quota=`/rate_limit`
/// - balance: validate=`/`, quota=`/`
/// - deepseek: validate=`/user/balance`, quota=`/user/balance`
pub fn resolve_custom(cs: &CustomSpec) -> Result<ResolvedProvider, MergerError> {
    let base_url = cs
        .base_url
        .clone()
        .ok_or_else(|| MergerError::InvalidCustomSpec("base_url is required".into()))?;
    let auth_header = cs
        .auth_header
        .clone()
        .ok_or_else(|| MergerError::InvalidCustomSpec("auth_header is required".into()))?;
    let (default_validate_path, default_quota_path) = match cs.protocol {
        ProtocolId::Openai => ("/models", "/dashboard/billing/subscription"),
        ProtocolId::Anthropic => ("/v1/messages", "/v1/messages"),
        ProtocolId::Github => ("/rate_limit", "/rate_limit"),
        ProtocolId::Balance => ("/", "/"),
        ProtocolId::Deepseek => ("/user/balance", "/user/balance"),
    };
    let validate_probe = cs
        .validate
        .as_ref()
        .map(|v| ProbeSpec {
            path: v.path.clone(),
            method: v.method.clone().unwrap_or_else(|| "GET".to_string()),
            success_status: vec![200],
            parser: v.parser,
        })
        .unwrap_or_else(|| ProbeSpec {
            path: default_validate_path.to_string(),
            method: "GET".to_string(),
            success_status: vec![200],
            parser: ParserId::None,
        });
    let quota_probe = cs
        .quota
        .as_ref()
        .map(|q| ProbeSpec {
            path: q.path.clone(),
            method: q.method.clone().unwrap_or_else(|| "GET".to_string()),
            success_status: vec![200],
            parser: q.parser,
        })
        .unwrap_or_else(|| ProbeSpec {
            path: default_quota_path.to_string(),
            method: "GET".to_string(),
            success_status: vec![200],
            parser: ParserId::None,
        });
    Ok(ResolvedProvider {
        id: format!("custom:{}", cs.protocol.as_str()),
        name: "Custom Provider".to_string(),
        icon: String::new(),
        protocol: cs.protocol,
        base_url,
        auth_header,
        validate_probe,
        quota_probe,
        coding_plan: None,
        extras: vec![],
    })
}

fn base_from_entry(entry: &CatalogEntry) -> ResolvedProvider {
    ResolvedProvider {
        id: entry.meta.id.clone(),
        name: entry.meta.name.clone(),
        icon: entry.meta.icon.clone(),
        protocol: entry.meta.protocol,
        base_url: entry.endpoint.default_base_url.clone(),
        auth_header: entry.auth.auth_header.clone(),
        validate_probe: entry.validate_probe.clone(),
        quota_probe: entry.quota_probe.clone(),
        coding_plan: entry.meta.coding_plan,
        extras: entry.extras.clone(),
    }
}

fn apply_custom_spec(resolved: &mut ResolvedProvider, cs: &CustomSpec) {
    resolved.protocol = cs.protocol;
    if let Some(base_url) = &cs.base_url {
        resolved.base_url = base_url.clone();
    }
    if let Some(auth_header) = &cs.auth_header {
        resolved.auth_header = auth_header.clone();
    }
    if let Some(validate) = &cs.validate {
        resolved.validate_probe.path = validate.path.clone();
        resolved.validate_probe.parser = validate.parser;
        if let Some(method) = &validate.method {
            resolved.validate_probe.method = method.clone();
        }
    }
    if let Some(quota) = &cs.quota {
        resolved.quota_probe.path = quota.path.clone();
        resolved.quota_probe.parser = quota.parser;
        if let Some(method) = &quota.method {
            resolved.quota_probe.method = method.clone();
        }
    }
}

/// Seed providers from catalog into the database.
/// Idempotent via `INSERT OR IGNORE` on the preset column (REQ-CAT-002 dual-seeder with V0.1).
/// Phase 2 wires this into `lib.rs::run()` after `db.setup_schema()`.
pub fn seed_providers(db: &crate::database::Database) -> Result<(), crate::error::AppError> {
    use rusqlite::params;
    let conn = db.conn();
    let now = crate::timeutil::now_secs();
    for (idx, entry) in CATALOG.iter().enumerate() {
        conn.execute(
            "INSERT OR IGNORE INTO providers
                (name, preset, is_preset, category_id, pinned, icon, sort_index, created_at, updated_at)
             VALUES (?1, ?2, 1, 1, 1, ?3, ?4, ?5, ?5)",
            params![entry.meta.name, entry.meta.id, entry.meta.icon, idx as i64, now],
        )
        .map_err(crate::error::AppError::Database)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_catalogs_parse() {
        // All 21 toml files must parse successfully (V0.2.1: 19 base + Fireworks + AI21)
        let entries = load_all();
        assert_eq!(entries.len(), 21, "expected 21 catalog entries");
        for entry in entries {
            assert!(!entry.meta.id.is_empty());
            assert!(!entry.meta.name.is_empty());
        }
    }

    #[test]
    fn custom_spec_legal_openai_compat() {
        let json = r#"{
            "protocol": "openai",
            "base_url": "https://proxy.example.com/v1",
            "validate": { "path": "/models", "parser": "none" },
            "quota": { "path": "/usage", "parser": "openai_usage" }
        }"#;
        let spec: CustomSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.protocol, ProtocolId::Openai);
        assert_eq!(spec.base_url.as_deref(), Some("https://proxy.example.com/v1"));
        assert_eq!(spec.validate.as_ref().unwrap().path, "/models");
        assert_eq!(spec.quota.as_ref().unwrap().parser, ParserId::OpenaiUsage);
    }

    #[test]
    fn custom_spec_legal_anthropic_compat() {
        let json = r#"{
            "protocol": "anthropic",
            "base_url": "https://api.mymodel.cn/anthropic",
            "auth_header": "x-api-key: {api_key}"
        }"#;
        let spec: CustomSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.protocol, ProtocolId::Anthropic);
    }

    #[test]
    fn custom_spec_illegal_unknown_protocol() {
        let json = r#"{
            "protocol": "bogus",
            "base_url": "https://example.com"
        }"#;
        let result: Result<CustomSpec, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown protocol should fail serde");
    }

    #[test]
    fn merger_openai_default() {
        let resolved = resolve("openai", None).unwrap();
        assert_eq!(resolved.id, "openai");
        assert_eq!(resolved.base_url, "https://api.openai.com/v1");
        assert_eq!(resolved.protocol, ProtocolId::Openai);
        assert!(resolved.auth_header.contains("{api_key}"));
    }

    #[test]
    fn merger_openai_override_base_url() {
        let custom = CustomSpec {
            protocol: ProtocolId::Openai,
            base_url: Some("https://proxy.example.com/v1".to_string()),
            auth_header: None,
            validate: None,
            quota: None,
            notes: None,
        };
        let resolved = resolve("openai", Some(&custom)).unwrap();
        assert_eq!(resolved.base_url, "https://proxy.example.com/v1");
        // Other fields remain from catalog
        assert_eq!(resolved.protocol, ProtocolId::Openai);
    }

    #[test]
    fn merger_unknown_preset() {
        let result = resolve("nonexistent", None);
        assert!(matches!(result, Err(MergerError::UnknownPreset(_))));
    }

    #[test]
    fn resolve_custom_minimal_openai() {
        let cs = CustomSpec {
            protocol: ProtocolId::Openai,
            base_url: Some("https://proxy.example.com/v1".into()),
            auth_header: Some("Authorization: Bearer {api_key}".into()),
            validate: Some(ProbeOverride {
                path: "/models".into(),
                parser: ParserId::None,
                method: None,
            }),
            quota: Some(ProbeOverride {
                path: "/dashboard/billing/usage".into(),
                parser: ParserId::OpenaiUsage,
                method: None,
            }),
            notes: None,
        };
        let resolved = resolve_custom(&cs).expect("resolve_custom OK");
        assert_eq!(resolved.protocol, ProtocolId::Openai);
        assert_eq!(resolved.base_url, "https://proxy.example.com/v1");
        assert_eq!(resolved.validate_probe.path, "/models");
        assert_eq!(resolved.quota_probe.path, "/dashboard/billing/usage");
        assert_eq!(resolved.coding_plan, None);
    }

    #[test]
    fn resolve_custom_missing_base_url_rejected() {
        let cs = CustomSpec {
            protocol: ProtocolId::Anthropic,
            base_url: None,
            auth_header: Some("x-api-key: {api_key}".into()),
            validate: None,
            quota: None,
            notes: None,
        };
        let result = resolve_custom(&cs);
        assert!(matches!(result, Err(MergerError::InvalidCustomSpec(_))));
    }

    #[test]
    fn success_status_defaults_to_200_when_omitted_in_toml() {
        // If a toml omits success_status entirely, default must be [200] (REQ-CAT-001).
        let toml_text = r#"
            [meta]
            id = "tmp"
            name = "Tmp"
            protocol = "openai"
            icon = ""
            [endpoint]
            default_base_url = "https://example.com"
            [auth]
            auth_header = "X-Key: {api_key}"
            [validate_probe]
            path = "/health"
            parser = "none"
            [quota_probe]
            path = "/quota"
            parser = "none"
        "#;
        let entry: CatalogEntry = toml::from_str(toml_text).expect("parse");
        assert_eq!(entry.validate_probe.success_status, vec![200]);
        assert_eq!(entry.validate_probe.method, "GET");
    }

    #[test]
    fn resolve_custom_uses_protocol_defaults_when_probes_missing() {
        // Phase 3 dialog only sends { protocol, base_url, auth_header, notes }.
        // resolve_custom must default validate_probe + quota_probe per protocol.
        let cs = CustomSpec {
            protocol: ProtocolId::Openai,
            base_url: Some("https://proxy.example.com/v1".into()),
            auth_header: Some("Authorization: Bearer {api_key}".into()),
            validate: None,
            quota: None,
            notes: Some("company proxy".into()),
        };
        let resolved = resolve_custom(&cs).expect("resolve_custom OK");
        // openai defaults
        assert_eq!(resolved.validate_probe.path, "/models");
        assert_eq!(resolved.validate_probe.parser, ParserId::None);
        assert_eq!(resolved.quota_probe.path, "/dashboard/billing/subscription");
    }

    #[test]
    fn resolve_custom_overrides_default_probes_when_provided() {
        let cs = CustomSpec {
            protocol: ProtocolId::Anthropic,
            base_url: Some("https://example.com".into()),
            auth_header: Some("x-api-key: {api_key}".into()),
            validate: Some(ProbeOverride {
                path: "/health".into(),
                parser: ParserId::None,
                method: None,
            }),
            quota: Some(ProbeOverride {
                path: "/custom-quota".into(),
                parser: ParserId::Balance,
                method: None,
            }),
            notes: None,
        };
        let resolved = resolve_custom(&cs).expect("resolve_custom OK");
        assert_eq!(resolved.validate_probe.path, "/health");
        assert_eq!(resolved.quota_probe.path, "/custom-quota");
        assert_eq!(resolved.quota_probe.parser, ParserId::Balance);
    }

    #[test]
    fn all_preset_ids_count() {
        let ids = all_preset_ids();
        assert_eq!(ids.len(), 21, "all_preset_ids should return 21 ids");
        assert!(ids.contains(&"openai"));
        assert!(ids.contains(&"anthropic"));
        assert!(ids.contains(&"deepseek"));
        assert!(ids.contains(&"github"));
        assert!(ids.contains(&"minimax-cn"));
        assert!(ids.contains(&"minimax-en"));
        assert!(ids.contains(&"mimo"));
        // merged -anthropic tomls no longer exist as separate entries
        assert!(!ids.contains(&"minimax-cn-anthropic"));
        assert!(!ids.contains(&"minimax-en-anthropic"));
    }

    // Phase 5: extras field tests

    #[test]
    fn resolve_preserves_extras() {
        // resolve() should carry extras from catalog entry
        let resolved = resolve("minimax-cn", None).unwrap();
        assert_eq!(resolved.extras.len(), 1, "resolved minimax-cn should have 1 extra");
        assert_eq!(resolved.extras[0].protocol, ProtocolId::Anthropic);
        assert!(resolved.extras[0].auth_header.contains("{api_key}"));
    }

    #[test]
    fn resolve_custom_does_not_have_extras() {
        // resolve_custom (preflight) must NOT have extras per REQ-CAT-021
        let cs = CustomSpec {
            protocol: ProtocolId::Openai,
            base_url: Some("https://proxy.example.com/v1".into()),
            auth_header: Some("Authorization: Bearer {api_key}".into()),
            validate: None,
            quota: None,
            notes: None,
        };
        let resolved = resolve_custom(&cs).expect("resolve_custom OK");
        assert!(resolved.extras.is_empty(), "resolve_custom must not have extras");
    }

    #[test]
    fn catalog_extras_table_driven() {
        // 6 dual-protocol presets each have 1 anthropic extra
        // covers minimax-cn / minimax-en / kimi / zhipu / volcengine / deepseek
        let cases: &[(&str, &str)] = &[
            ("minimax-cn", "https://api.minimaxi.com/anthropic"),
            ("minimax-en", "https://api.minimax.io/anthropic"),
            ("kimi", "https://api.moonshot.cn/anthropic"),
            ("zhipu", "https://open.bigmodel.cn/api/anthropic"),
            ("volcengine", "https://ark.cn-beijing.volces.com/api/coding"),
            ("deepseek", "https://api.deepseek.com/anthropic"),
        ];
        for (preset_id, expected_url) in cases {
            let entry = load_all().into_iter().find(|e| e.meta.id == *preset_id)
                .unwrap_or_else(|| panic!("preset {preset_id} missing from catalog"));
            assert_eq!(entry.extras.len(), 1, "{preset_id} should have 1 extra");
            assert_eq!(entry.extras[0].protocol, ProtocolId::Anthropic);
            assert_eq!(entry.extras[0].base_url, *expected_url);
            assert!(entry.extras[0].auth_header.contains("{api_key}"));
        }
    }

    #[test]
    fn catalog_extras_openai_is_empty() {
        // single-protocol presets have no extras
        let entry = load_all().into_iter().find(|e| e.meta.id == "openai").unwrap();
        assert!(entry.extras.is_empty(), "openai should have no extras");
    }
}
