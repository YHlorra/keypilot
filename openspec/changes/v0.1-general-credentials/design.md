# V0.1 General Credentials — Design

> **目标**: 把 `spec.md` 的 14 REQ 落到具体代码骨架 + 数据库迁移 + 文件清单。
> **范围**: Stage 1-4 实施所需的 schema / 错误 / 类型 / IPC 接口 / adapter 骨架。

---

## 1. Stage 1 Schema(DROP + 重建,v3)

> ⚠️ V0.1 尚未发布,无真实用户数据,采用 DROP 重建,避免 v1→v2→v3 兼容代码膨胀。
> 若 Stage 1 后已有用户数据,需补 RFC 评估 v2→v3 兼容迁移(本设计不涵盖)。

### 1.1 最终 schema(`src-tauri/src/database.rs`)

```sql
-- ─── meta ───────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
INSERT OR IGNORE INTO meta (key, value) VALUES ('schema_version', '3');
INSERT OR IGNORE INTO meta (key, value) VALUES ('preset_seeded', '0');
INSERT OR IGNORE INTO meta (key, value) VALUES ('theme', 'auto');

-- ─── categories ─────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0,
    sort_index INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 首次启动 seed 默认 category(id=1)
INSERT OR IGNORE INTO categories (id, name, is_default, sort_index, created_at, updated_at)
VALUES (1, '凭证', 1, 0, strftime('%s','now'), strftime('%s','now'));

-- ─── providers ──────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS providers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    preset TEXT,                              -- 'openai'|'deepseek'|'anthropic'|'github'|'postgres'|NULL
    is_preset INTEGER NOT NULL DEFAULT 0,    -- 沿用 v0.1-spec-alignment REQ-PROV-003
    category_id INTEGER NOT NULL,
    pinned INTEGER NOT NULL DEFAULT 0,
    notes TEXT,
    icon TEXT,
    icon_color TEXT,
    sort_index INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS idx_providers_category ON providers(category_id, sort_index);
CREATE INDEX IF NOT EXISTS idx_providers_preset ON providers(preset);

-- ─── provider_fields ───────────────────────────────────────────
CREATE TABLE IF NOT EXISTS provider_fields (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id INTEGER NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,                     -- V0.1 全部明文,无加密
    visibility TEXT NOT NULL DEFAULT 'visible',  -- 'visible'|'masked'(V0.1 二态,'private' 推迟 V0.2)
    sort_index INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_pf_provider ON provider_fields(provider_id, sort_index);

-- ─── quota_cache ───────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS quota_cache (
    provider_id INTEGER PRIMARY KEY,
    snapshot_json TEXT NOT NULL,
    fetched_at INTEGER NOT NULL,
    FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
);
```

### 1.2 关键 DDL 触发顺序(`lib.rs::run()`)

```rust
fn run() {
    setup_db(&app)?;                  // Stage 1
    seed_default_category(&db)?;     // 创建 "凭证" category
    seed_preset_providers(&db)?;     // 首次启动 seed 5 个 preset
    setup_window_with_theme(&app)?;  // 读 meta.theme 决定初值
    build_tray(&app)?;               // Stage 5
}
```

## 2. Rust 类型(`src-tauri/src/types.rs`,Stage 1 新建)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Visible,
    Masked,
    // V0.1 不实现 Private:加密整体推迟到 V0.2 RFC
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Visible => "visible",
            Self::Masked => "masked",
        }
    }
    pub fn parse(s: &str) -> Result<Self, AppError> {
        match s {
            "visible" => Ok(Self::Visible),
            "masked" => Ok(Self::Masked),
            _ => Err(AppError::InvalidVisibility(s.into())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderField {
    pub id: i64,
    pub provider_id: i64,
    pub key: String,
    pub value: String,                    // V0.1 全部明文
    pub visibility: Visibility,
    pub sort_index: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: i64,
    pub name: String,
    pub preset: Option<String>,           // None = Custom
    pub is_preset: bool,
    pub category_id: i64,
    pub pinned: bool,
    pub notes: Option<String>,
    pub icon: Option<String>,
    pub icon_color: Option<String>,
    pub sort_index: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub fields: Vec<ProviderField>,       // 列表时随行带出
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub is_default: bool,
    pub sort_index: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    Auto,
}

impl Theme {
    pub fn as_str(&self) -> &'static str { /* ... */ }
    pub fn parse(s: &str) -> Result<Self, AppError> { /* ... */ }
}
```

## 3. AppError(`src-tauri/src/error.rs`)

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("invalid visibility: {0}")]
    InvalidVisibility(String),

    #[error("invalid theme: {0}")]
    InvalidTheme(String),

    #[error("provider not found: id={0}")]
    ProviderNotFound(i64),

    #[error("category not found: id={0}")]
    CategoryNotFound(i64),

    #[error("category is default and cannot be deleted: id={0}")]
    CategoryIsDefault(i64),

    #[error("provider {0} cannot be tested")]
    ProviderCannotTest(String),

    #[error("provider {0} does not support fetch_quota")]
    ProviderQuotaUnsupported(String),

    #[error("http error: {0}")]
    Http(String),
}

// Tauri command 序列化: { code: String, message: String }
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 2)?;
        let code = match self {
            Self::Database(_) => "DATABASE",
            Self::Io(_) => "IO",
            Self::Serde(_) => "SERDE",
            Self::InvalidVisibility(_) => "INVALID_VISIBILITY",
            Self::InvalidTheme(_) => "INVALID_THEME",
            Self::ProviderNotFound(_) => "PROVIDER_NOT_FOUND",
            Self::CategoryNotFound(_) => "CATEGORY_NOT_FOUND",
            Self::CategoryIsDefault(_) => "CATEGORY_IS_DEFAULT",
            Self::ProviderCannotTest(_) => "PROVIDER_CANNOT_TEST",
            Self::ProviderQuotaUnsupported(_) => "PROVIDER_QUOTA_UNSUPPORTED",
            Self::Http(_) => "HTTP",
        };
        s.serialize_field("code", code)?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}
```

## 4. ~~加密骨架~~ — **V0.1 移除**(用户决策"不加密")

V0.1 不实现 `crypto.rs`,所有字段明文存储。

- 依赖调整:`aes-gcm` / `base64` / `rand` **不引入** Cargo.toml
- V0.2 评估加密时再回来:可能落 SQLCipher 全文件 / 主密码 + argon2id / Windows DPAPI / 字段级 AES-GCM
- V0.1 期间威胁模型:依赖 Windows ACL + 用户 Windows 密码(透明告知用户,不误导)

## 5. Store / AppState(`src-tauri/src/store.rs`)

```rust
use std::sync::{Arc, Mutex};
use crate::database::Database;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        Self { db: Arc::new(Mutex::new(db)) }
    }
}
```

> **rev 2 修正 (Stage 1 实施发现,2026-06-24)**:`Arc<Database>` **不会编译** — `rusqlite::Connection` 是 `Send` 但 **不是 `Sync`**,Tauri 2 的 `manage()` 要求 managed state 是 `Send + Sync`。修正为 `Arc<Mutex<Database>>`(标准 rusqlite + Tauri 集成模式)。所有 `services/*.rs` 调用 `state.db.lock().unwrap()` 获取短期 lock;SQLite 操作同步快(< 1ms),Mutex contention 可忽略。Stage 2/3 实现时**所有 AppState 访问必须先 `.lock().unwrap()`**。若未来需要细粒度并发,Database 内部用 `tokio::sync::Mutex<Connection>`,但 V0.1 不需要。

## 6. Adapter trait(`src-tauri/src/provider/adapter.rs`,Stage 2)

```rust
use async_trait::async_trait;
use crate::error::AppError;

#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    fn preset(&self) -> &'static str;     // 'openai'|'deepseek'|'anthropic'|'github'|'postgres'
    fn can_test(&self) -> bool;           // 沿用 REQ-PROV-009,3 LLM=true, GitHub/Postgres=false
    fn can_fetch_quota(&self) -> bool;    // 沿用 REQ-QUOTA-004,3 LLM=true, GitHub/Postgres=true
    
    async fn test_connection(&self, base_url: &str, api_key: &str) -> Result<(), AppError>;
    async fn fetch_quota(&self, base_url: &str, api_key: &str) -> Result<crate::types::QuotaSnapshot, AppError>;
}

pub fn adapter_for(preset: &str) -> Option<Box<dyn ProviderAdapter>> {
    match preset {
        "openai" => Some(Box::new(crate::provider::openai::OpenAiAdapter)),
        "deepseek" => Some(Box::new(crate::provider::deepseek::DeepSeekAdapter)),
        "anthropic" => Some(Box::new(crate::provider::anthropic::AnthropicAdapter)),
        "github" => Some(Box::new(crate::provider::github::GitHubAdapter)),
        "postgres" => Some(Box::new(crate::provider::postgres::PostgresAdapter)),
        _ => None,  // Custom(preset=NULL)和未知 preset 返回 None;caller 决定如何处理 (ProviderCannotTest / ProviderQuotaUnsupported)
    }
}
```

> Custom 路径:`preset=NULL` 的 provider 不走 adapter_for,前端 UI 显示"自定义"徽章,所有 action(test / fetch_quota)都 disabled。

## 7. IPC 命令清单(Stage 2-3 实现,本设计先列接口)

```rust
// commands/provider.rs
// All commands use single-struct arg pattern (Phase 1.5 oracle 修正:update_provider 原本 `id + req` 改为单 struct)
// JS calls: invoke<Res>('cmd_name', req)  where req is the Request struct (id 嵌在 update/delete_request 里)
#[tauri::command] async fn list_providers(state) -> Result<Vec<Provider>, AppError>;
#[tauri::command] async fn get_provider(state, id: i64) -> Result<Provider, AppError>;
#[tauri::command] async fn add_provider(state, req: AddProviderRequest) -> Result<Provider, AppError>;
#[tauri::command] async fn update_provider(state, req: UpdateProviderRequest) -> Result<Provider, AppError>;  // Phase 1.5 oracle: id inside req
#[tauri::command] async fn delete_provider(state, id: i64) -> Result<(), AppError>;
#[tauri::command] async fn list_categories(state) -> Result<Vec<Category>, AppError>;
#[tauri::command] async fn add_category(state, req: AddCategoryRequest) -> Result<Category, AppError>;  // Phase 1.5 oracle: single struct
#[tauri::command] async fn delete_category(state, req: DeleteCategoryRequest) -> Result<(), AppError>;  // Phase 1.5 oracle: id+migrate_to inside req
#[tauri::command] async fn test_connection(state, id: i64) -> Result<(), AppError>;
#[tauri::command] async fn fetch_quota(state, id: i64) -> Result<QuotaSnapshot, AppError>;
#[tauri::command] async fn get_theme(state) -> Result<Theme, AppError>;  // Phase 1.5 oracle: Theme enum not String
#[tauri::command] async fn set_theme(state, theme: Theme) -> Result<(), AppError>;  // Phase 1.5 oracle: Theme enum not String
```

## 8. 前端契约(`webui/src/types/`,Stage 3 落地)

```typescript
// webui/src/types/api.ts (Stage 3 创建)
export type Visibility = 'visible' | 'masked';  // V0.1 二态,'private' 推迟 V0.2
export type Theme = 'dark' | 'light' | 'auto';

export interface ProviderField {
  id: number;
  provider_id: number;
  key: string;
  value: string;
  visibility: Visibility;
  sort_index: number;
}

export interface Provider {
  id: number;
  name: string;
  preset: string | null;
  is_preset: boolean;
  category_id: number;
  pinned: boolean;
  notes: string | null;
  fields: ProviderField[];
}

export interface Category {
  id: number;
  name: string;
  is_default: boolean;
}

export interface AppError {
  code: string;
  message: string;
}
```

## 9. Stage 1 文件清单(更新)

```
src-tauri/src/
├── main.rs                       (更新:window label / frontendDist 占位)
├── lib.rs                        (重写:setup_db → seed_default_category → seed_preset_providers)
├── database.rs                   (重写:categories + providers + provider_fields + quota_cache,schema v3)
├── store.rs                      (重写:AppState with Arc<Mutex<Database>>,rev 2 修正 Send+Sync)
├── error.rs                      (重写:AppError enum + Serialize for IPC)
├── types.rs                      (新建:Provider / ProviderField / Category / Visibility(二态) / Theme)
├── provider/
│   ├── mod.rs                    (Stage 2 创建,match 6 preset)
│   ├── adapter.rs                (Stage 2 创建,ProviderAdapter trait + adapter_for)
│   ├── openai.rs                 (Stage 2 创建)
│   ├── deepseek.rs               (Stage 2 创建)
│   ├── anthropic.rs              (Stage 2 创建)
│   ├── github.rs                 (Stage 2 创建)
│   └── postgres.rs               (Stage 2 创建,需 tokio-postgres)
├── services/
│   ├── provider.rs               (Stage 2 创建)
│   └── quota.rs                  (Stage 4 创建)
├── commands/
│   ├── provider.rs               (Stage 2 创建,12 IPC)
│   ├── quota.rs                  (Stage 4 创建)
│   └── tray.rs                   (Stage 5 创建)
└── tray.rs                       (Stage 5 创建)
```

> ❌ **删除** `crypto.rs`(V0.1 不加密)
> ❌ **不引入** `aes-gcm` / `base64` / `rand` 依赖

## 10. Stage 1 Cargo.toml

```toml
[dependencies]
tauri = "2"
tokio = "1"
rusqlite = "0.32"
serde = "1"
serde_json = "1"
thiserror = "1"
chrono = "0.4"
async-trait = "0.1"        # 新增:ProviderAdapter trait(Stage 2 提前引入)
# reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }  # Stage 2-4 加
```

> **硬约束 grep** 需追加(提交前):
> - `grep -E "^aes-gcm|^argon2|^chacha20|ChaCha20Poly1305" src-tauri/Cargo.toml` — **必须空**(V0.1 无加密 crate)
