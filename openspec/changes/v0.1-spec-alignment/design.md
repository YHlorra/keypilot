# V0.1 Spec Alignment — Design

> 实现细节 + 参考项目引用 + 代码骨架。
> 对应 `spec.md` 的 REQ-XXX-NNN。

---

## 1. OpenAI Quota Algorithm(REQ-QUOTA-001)

### Reference Source

`references/tier2-tech/openai-balance/cmd/root.go` L1-163(Go 完整实现)。

### Key Lines from Reference

| Line | 内容 | KeyPilot 含义 |
|---|---|---|
| L39 | `const limitUrl = "https://api.openai.com/dashboard/billing/subscription"` | base_url 硬编码,不带 `/v1` |
| L40 | `const usageUrl = "https://api.openai.com/dashboard/billing/usage"` | 同上 |
| L88 | `request.Header.Add("Authorization", "Bearer "+apiKey)` | Bearer auth |
| L90-95 | usage 端点需要 `start_date` / `end_date` query param | 必传 |
| L123 | `hardLimit := (*limit)["hard_limit_usd"].(float64)` | 已是 USD 单位 |
| L131-136 | `for startTime.Before(time.Now()) { startTime = endTime; endTime = endTime.AddDate(0, 3, 0) }` | **3-month 窗口循环** |
| L140 | `usage += (*res)["total_usage"].(float64)` | total_usage 单位是 **cents** |
| L147 | `usage = math.Ceil(usage)` | 向上取整消除浮点误差 |
| L149 | `fmt.Println(hardLimit - usage/100)` | hard_limit - usage/100(cents → USD) |

### Rust 翻译

```rust
// src-tauri/src/provider/openai.rs
use async_trait::async_trait;
use chrono::{Datelike, Months, NaiveDate, Utc};
use serde::Deserialize;
use std::time::Duration;

use super::adapter::{ProviderAdapter, QuotaError, QuotaSnapshot, QuotaSource, ValidateError};
use super::ProviderKind;

pub struct OpenAIAdapter;

#[derive(Deserialize)]
struct SubResp {
    hard_limit_usd: f64,
    plan: SubPlan,
    access_until: i64,
}

#[derive(Deserialize)]
struct SubPlan {
    title: String,
}

#[derive(Deserialize)]
struct UsageResp {
    total_usage: f64,  // cents
}

#[async_trait]
impl ProviderAdapter for OpenAIAdapter {
    fn kind(&self) -> ProviderKind { ProviderKind::OpenAI }
    fn default_base_url(&self) -> &'static str { "https://api.openai.com" }

    async fn validate_key(
        &self,
        base_url: &str,
        api_key: &str,
    ) -> Result<(), ValidateError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| ValidateError::Network(e.to_string()))?;

        let resp = client
            .get(format!("{base_url}/v1/models"))
            .header("Authorization", format!("Bearer {api_key}"))
            .send()
            .await
            .map_err(|e| ValidateError::Network(e.to_string()))?;

        match resp.status().as_u16() {
            200 | 201 => Ok(()),
            401 | 403 => Err(ValidateError::InvalidKey),
            s => Err(ValidateError::Network(format!("unexpected status {s}"))),
        }
    }

    async fn fetch_quota(
        &self,
        base_url: &str,
        api_key: &str,
    ) -> Result<QuotaSnapshot, QuotaError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| QuotaError::Network(e.to_string()))?;

        // ── Step 1: subscription → hard_limit_usd ──
        let sub: SubResp = client
            .get(format!("{base_url}/dashboard/billing/subscription"))
            .header("Authorization", format!("Bearer {api_key}"))
            .send()
            .await
            .map_err(|e| QuotaError::Network(e.to_string()))?
            .error_for_status()
            .map_err(|e| match e.status() {
                Some(s) if s.as_u16() == 404 => QuotaError::NoSubscription,
                _ => QuotaError::Auth(e.to_string()),
            })?
            .json()
            .await
            .map_err(|e| QuotaError::Parse(e.to_string()))?;

        let total_usd = sub.hard_limit_usd;
        let plan_name = sub.plan.title;

        // ── Step 2: usage → 3-month window iteration ──
        let mut total_cents = 0.0_f64;
        let mut start = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let now_date = Utc::now().date_naive();

        while start < now_date {
            let end = start
                .checked_add_months(Months::new(3))
                .unwrap_or(now_date)
                .min(now_date);

            let usage: UsageResp = client
                .get(format!("{base_url}/dashboard/billing/usage"))
                .query(&[
                    ("start_date", start.format("%Y-%m-%d").to_string()),
                    ("end_date", end.format("%Y-%m-%d").to_string()),
                ])
                .header("Authorization", format!("Bearer {api_key}"))
                .send()
                .await
                .map_err(|e| QuotaError::Network(e.to_string()))?
                .error_for_status()
                .map_err(|e| QuotaError::Auth(e.to_string()))?
                .json()
                .await
                .map_err(|e| QuotaError::Parse(e.to_string()))?;

            total_cents += usage.total_usage;
            start = end;
        }

        // 向上取整(openai-balance/cmd/root.go L147)
        let used_usd = (total_cents / 100.0).ceil();
        let remaining_usd = total_usd - used_usd;

        Ok(QuotaSnapshot {
            provider_id: 0,  // service layer 填
            provider_kind: ProviderKind::OpenAI,
            total: Some(total_usd),
            used: Some(used_usd),
            remaining: Some(remaining_usd),
            unit: "USD".into(),
            plan_name: Some(plan_name),
            is_valid: true,
            invalid_message: None,
            fetched_at: Utc::now().timestamp(),
            source: QuotaSource::Api,
        })
    }
}
```

---

## 2. DeepSeek Quota Algorithm(REQ-QUOTA-002)

### Reference Source

`references/tier1-direct/cc-switch/src-tauri/src/services/balance.rs` L68-134。

### Key Lines from Reference

| Line | 内容 | KeyPilot 含义 |
|---|---|---|
| L72-77 | `GET https://api.deepseek.com/user/balance` + Bearer + 15s | 标准 GET |
| L98-101 | `body.is_available` 默认 true | 账户可用性 |
| L104-127 | `balance_infos[].{currency, total_balance, granted_balance, topped_up_balance}` | 余额数组 |
| L110 | `total = parse_f64_field(info, "total_balance")` | 已在正确单位 |

### Rust 翻译

```rust
// src-tauri/src/provider/deepseek.rs
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

use super::adapter::{ProviderAdapter, QuotaError, QuotaSnapshot, QuotaSource, ValidateError};
use super::ProviderKind;

pub struct DeepSeekAdapter;

#[derive(Deserialize)]
struct DeepSeekResp {
    balance_infos: Vec<BalanceInfo>,
    is_available: Option<bool>,
}

#[derive(Deserialize)]
struct BalanceInfo {
    currency: String,
    total_balance: f64,
}

#[async_trait]
impl ProviderAdapter for DeepSeekAdapter {
    fn kind(&self) -> ProviderKind { ProviderKind::DeepSeek }
    fn default_base_url(&self) -> &'static str { "https://api.deepseek.com" }

    async fn validate_key(
        &self,
        base_url: &str,
        api_key: &str,
    ) -> Result<(), ValidateError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| ValidateError::Network(e.to_string()))?;

        let resp = client
            .get(format!("{base_url}/user/balance"))
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| ValidateError::Network(e.to_string()))?;

        match resp.status().as_u16() {
            200 | 201 => Ok(()),
            401 | 403 => Err(ValidateError::InvalidKey),
            s => Err(ValidateError::Network(format!("unexpected status {s}"))),
        }
    }

    async fn fetch_quota(
        &self,
        base_url: &str,
        api_key: &str,
    ) -> Result<QuotaSnapshot, QuotaError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| QuotaError::Network(e.to_string()))?;

        let body: DeepSeekResp = client
            .get(format!("{base_url}/user/balance"))
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| QuotaError::Network(e.to_string()))?
            .error_for_status()
            .map_err(|e| QuotaError::Auth(e.to_string()))?
            .json()
            .await
            .map_err(|e| QuotaError::Parse(e.to_string()))?;

        let is_available = body.is_available.unwrap_or(true);
        let (currency, total_balance) = body
            .balance_infos
            .first()
            .map(|info| (info.currency.clone(), info.total_balance))
            .unwrap_or_else(|| ("CNY".to_string(), 0.0));

        Ok(QuotaSnapshot {
            provider_id: 0,
            provider_kind: ProviderKind::DeepSeek,
            total: None,        // DeepSeek API 不返回
            used: None,         // DeepSeek API 不返回
            remaining: Some(total_balance),
            unit: currency,
            plan_name: Some(currency),
            is_valid: is_available,
            invalid_message: if !is_available {
                Some("Insufficient balance".into())
            } else {
                None
            },
            fetched_at: chrono::Utc::now().timestamp(),
            source: QuotaSource::Api,
        })
    }
}
```

---

## 3. Anthropic Validate Key(REQ-PROV-004)

### 为什么 POST /v1/messages + max_tokens=1

- Anthropic **没有** `/v1/models` 公开端点(参考搜索无结果)
- POST `/v1/messages` 是唯一"能验证 key 又几乎不花钱"的方式
- 1 input + 1 output token ≈ $0.00006(Sonnet 价格,可忽略)
- 200 = valid, 401 = invalid, **400 = ambiguous**(可能 key 有效但请求被安全过滤拒)

### 实现

```rust
// src-tauri/src/provider/anthropic.rs
use async_trait::async_trait;
use serde_json::json;
use std::time::Duration;

use super::adapter::{ProviderAdapter, QuotaError, QuotaSnapshot, ValidateError};
use super::ProviderKind;

pub struct AnthropicAdapter;

#[async_trait]
impl ProviderAdapter for AnthropicAdapter {
    fn kind(&self) -> ProviderKind { ProviderKind::Anthropic }
    fn default_base_url(&self) -> &'static str { "https://api.anthropic.com" }

    async fn validate_key(
        &self,
        base_url: &str,
        api_key: &str,
    ) -> Result<(), ValidateError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| ValidateError::Network(e.to_string()))?;

        let resp = client
            .post(format!("{base_url}/v1/messages"))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&json!({
                "model": "claude-sonnet-4-20250514",
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "."}]
            }))
            .send()
            .await
            .map_err(|e| ValidateError::Network(e.to_string()))?;

        match resp.status().as_u16() {
            200 | 201 => Ok(()),
            401 | 403 => Err(ValidateError::InvalidKey),
            400 => Err(ValidateError::Ambiguous(
                "Key valid but request rejected (possible safety filter)"
            )),
            s => Err(ValidateError::Network(format!("unexpected status {s}"))),
        }
    }

    async fn fetch_quota(
        &self,
        _base_url: &str,
        _api_key: &str,
    ) -> Result<QuotaSnapshot, QuotaError> {
        // Anthropic 直接 API key 路径不支持 quota 查询
        // OAuth 路径需要 ~/.claude/.credentials.json,违反 §3.1
        Err(QuotaError::Unsupported(
            "Anthropic 需要手动输入额度 (OAuth 路径不兼容 1Password 范式)"
        ))
    }
}
```

---

## 4. Provider Duplicate + Preset Pattern(REQ-PROV-002/003)

### Reference Source

`references/tier1-direct/cc-switch/src-tauri/src/database/dao/providers.rs` L180-310

### cc-switch 模式摘要

- **主键** `(id, app_type)` — id 是 user-provided 字符串
- **`save_provider` UPSERT** — id 存在则 UPDATE,否则 INSERT
- **删除 = 硬删** + `provider_endpoints` 表 CASCADE
- **`init_default_official_providers` 一次性 flag**(`settings.official_providers_seeded=true`)
- **`is_official_seed_id()`** — 官方 seed 的 id 固定,删了不重建

### KeyPilot 简化

| 维度 | cc-switch | KeyPilot V0.1 |
|---|---|---|
| 主键 | `(id, app_type)` | `id TEXT`(uuid v4) |
| 唯一性约束 | id + app_type | id 唯一 |
| name 字段 | 不唯一 | 不唯一 |
| category 字段 | enum(7 种) | **删除**(用 is_preset 0/1 二元) |
| Seed 标记 | `is_official_seed_id()` 查 id | `is_preset=1` 字段 |
| Seed 重置 flag | `settings.official_providers_seeded` | `meta.preset_seeded` |
| 删除 | 硬删 + CASCADE | 同(quota_cache 也 CASCADE) |

### Seed 实现(Stage 2 落地)

```rust
// src-tauri/src/services/provider.rs (Stage 2)

use crate::database::{Database, NewProviderRow};
use crate::provider::ProviderKind;

pub fn init_default_providers(db: &Database) -> Result<usize, AppError> {
    // 一次性 flag 检查
    if db.get_meta("preset_seeded")?.as_deref() == Some("1") {
        return Ok(0);
    }

    let presets = [
        NewProviderRow {
            id: "preset-openai".into(),
            name: "OpenAI".into(),
            kind: ProviderKind::OpenAI,
            base_url: "https://api.openai.com".into(),
            api_key: "".into(),
            notes: None,
            tags: None,
            icon: None,
            icon_color: None,
            sort_index: 0,
            is_preset: 1,  // ← 新增字段
        },
        NewProviderRow {
            id: "preset-anthropic".into(),
            name: "Anthropic".into(),
            kind: ProviderKind::Anthropic,
            base_url: "https://api.anthropic.com".into(),
            api_key: "".into(),
            notes: None,
            tags: None,
            icon: None,
            icon_color: None,
            sort_index: 1,
            is_preset: 1,
        },
        NewProviderRow {
            id: "preset-deepseek".into(),
            name: "DeepSeek".into(),
            kind: ProviderKind::DeepSeek,
            base_url: "https://api.deepseek.com".into(),
            api_key: "".into(),
            notes: None,
            tags: None,
            icon: None,
            icon_color: None,
            sort_index: 2,
            is_preset: 1,
        },
    ];

    let mut count = 0;
    for row in presets {
        // 防御: 用户可能手动删了某 preset,这次启动补上(但只对未 seed 过的状态生效)
        // 用户主动删后,seeded flag 已经是 1,不会进到这里
        db.insert_provider(row)?;
        count += 1;
    }

    db.set_meta("preset_seeded", "1")?;
    Ok(count)
}
```

### 用户加 custom(Stage 2)

```rust
// 用户点 "添加 Provider" → 弹窗填 name / kind / base_url / api_key
// 后端:
pub async fn add_provider(
    db: &Database,
    name: String,
    kind: ProviderKind,
    base_url: String,
    api_key: String,
) -> Result<Provider, AppError> {
    // 1. 校验 name 必填(空 → AppError::EmptyName)
    if name.trim().is_empty() {
        return Err(AppError::EmptyName);
    }

    // 2. 生成 uuid 作为 id
    let id = uuid::Uuid::new_v4().to_string();

    // 3. 插入
    let row = NewProviderRow {
        id,
        name,
        kind,
        base_url,
        api_key,
        notes: None,
        tags: None,
        icon: None,
        icon_color: None,
        sort_index: 999,  // custom 排后面
        is_preset: 0,     // custom 标记
    };
    db.insert_provider(row)
}
```

### UI 区分(Stage 3)

```tsx
// webui/src/components/ProviderList.tsx
{providers.map(p => (
  <div key={p.id} className="provider-row">
    {p.is_preset === 1 && <span className="badge">📌 官方</span>}
    <span className="name">{p.name}</span>
    <span className="kind">{p.kind}</span>
    {/* 复制 / 编辑 / 删除 按钮 */}
  </div>
))}
```

---

## 5. Schema Migration(REQ-SCHEMA-001)

```rust
// src-tauri/src/database.rs (Stage 1)

const CURRENT_SCHEMA_VERSION: i32 = 2;

pub fn open(db_path: &Path) -> Result<Database, AppError> {
    let conn = Connection::open(db_path)?;

    // 创建 v1 schema(如果全新)
    conn.execute_batch(SCHEMA_V1_SQL)?;

    // 检查 schema_version,跑 migrations
    let version: i32 = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get::<_, String>(0).map(|s| s.parse().unwrap_or(1)),
        )
        .unwrap_or(1);

    if version < 2 {
        conn.execute_batch("
            ALTER TABLE providers ADD COLUMN is_preset INTEGER NOT NULL DEFAULT 0;
            UPDATE meta SET value = '2' WHERE key = 'schema_version';
        ")?;
    }

    Ok(Database { conn: Arc::new(Mutex::new(conn)) })
}

const SCHEMA_V1_SQL: &str = "
CREATE TABLE IF NOT EXISTS providers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,  -- v1 用 INTEGER, v2 改为 TEXT 需 migration
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    base_url TEXT NOT NULL,
    api_key TEXT NOT NULL,
    notes TEXT,
    tags TEXT,
    icon TEXT,
    icon_color TEXT,
    sort_index INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
-- (其他 v1 表)
";
```

**注意**: v1 → v2 实际是 schema 微调(只加 `is_preset` 列),`providers` 表 id 类型保持 INTEGER(因为 v1 spec 已经定)。如果未来 id 改为 TEXT uuid,需要 v2 → v3 migration(可能涉及数据迁移)。

---

## 6. 实现顺序

按依赖关系:

```
Stage 1 (DB)
  └─ schema migration v1 → v2 (is_preset 列)
  └─ seed 逻辑基础设施 (但实际调用在 Stage 2)

Stage 2 (Provider 模型)
  ├─ ProviderKind enum 缩到 3 + Custom
  ├─ adapter.rs: ValidateError enum + ProviderAdapter trait 完整定义
  ├─ provider/{openai,deepseek,anthropic}.rs: 三个实现(含 validate_key)
  └─ services/provider.rs: init_default_providers() + add_provider()

Stage 3 (UI)
  ├─ components/ProviderList.tsx: preset 徽章
  └─ components/CopyButton.tsx: 简化(无 format dropdown)

Stage 4 (Quota)
  ├─ services/quota.rs: fetch_one / fetch_all / get_cache 三个 command
  ├─ provider/{openai,deepseek}.rs: fetch_quota 实现
  └─ quota_cache 表写: 每次 fetch_quota 后 upsert
```

详细任务分解见 `tasks.md`。
