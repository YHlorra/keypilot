# V0.1 Spec Alignment — Spec Delta

> 格式: ADDED / MODIFIED / REMOVED。编号 `REQ-<DOMAIN>-<NNN>`。
> Domain: PROV(Provider/Adapter) / QUOTA(额度查询) / COPY(复制 UI) / SCHEMA(SQLite)

---

## ADDED

### REQ-PROV-001: ProviderKind 范围 — 3 enum + 3 实现 + Custom

```
enum ProviderKind { OpenAI, Anthropic, DeepSeek, Custom }
```

- 5 种未来 provider(Gemini / Mistral / Moonshot / Zhipu)的 enum 变体 **不在 V0.1 出现**
- 实施: `src-tauri/src/provider/mod.rs` match 必须穷尽 4 分支(3 + Custom)
- 理由: YAGNI("100 家 LLM 来了就适配 100 家",V0.2+ 范围)
- 拍板: 2026-06-24(用户决策)
- 覆盖: `MVP-范围.md 必做第 22 行` / `技术方案.md §4.3`

### REQ-PROV-002: 3 个官方 preset seed

首次启动(`lib.rs::run()`)seed 3 条 provider 行:

| name | kind | base_url | api_key | is_preset | id |
|---|---|---|---|---|---|
| `OpenAI` | OpenAI | `https://api.openai.com` | `""` | 1 | `preset-openai` |
| `Anthropic` | Anthropic | `https://api.anthropic.com` | `""` | 1 | `preset-anthropic` |
| `DeepSeek` | DeepSeek | `https://api.deepseek.com` | `""` | 1 | `preset-deepseek` |

- 触发: 首次启动 + `meta.preset_seeded != "1"`
- 完成后: `meta.preset_seeded = "1"` 写入
- 用户删除 preset 后: 不重建(尊重用户意图)
- 拍板: 2026-06-24(用户决策,"学习 cc-switch")

### REQ-PROV-003: schema.is_preset 列

```sql
ALTER TABLE providers ADD COLUMN is_preset INTEGER NOT NULL DEFAULT 0;
```

- 0 = custom(用户添加)
- 1 = official(首次启动 seed)
- migration: `schema_version` 从 1 → 2
- 影响: Stage 1 database.rs 实现 migration 框架

### REQ-PROV-004: validate_key 实现

每个 ProviderAdapter 实现 `validate_key(base_url, api_key) -> Result<(), ValidateError>`:

| Provider | 端点 | 成功 | 失败 | 边界 |
|---|---|---|---|---|
| OpenAI | `GET {base}/v1/models` | 200 | 401/403 | — |
| DeepSeek | `GET {base}/user/balance` | 200 | 401/403 | — |
| Anthropic | `POST {base}/v1/messages` | 200/201 | 401/403 | 400 = Ambiguous |

公共:
- timeout 15s
- Authorization 头按 provider 类型:`Bearer`(OpenAI/DeepSeek) / `x-api-key`(Anthropic)
- Anthropic 必须 header: `anthropic-version: 2023-06-01`
- Anthropic 必须 body: `{"model": "claude-sonnet-4-20250514", "max_tokens": 1, "messages": [{"role": "user", "content": "."}]}`

`ValidateError` enum:
```rust
enum ValidateError {
    InvalidKey,
    Ambiguous(&str),  // Anthropic 400 case
    Network(String),
}
```

### REQ-QUOTA-001: OpenAI fetch_quota 算法

抄自 `references/tier2-tech/openai-balance/cmd/root.go` L1-163。

```
Step 1: GET {base}/dashboard/billing/subscription
  → 200: { hard_limit_usd: f64, plan: { title: String, ... }, access_until: i64 }
  → 401/403: is_valid=false, invalid_message="Authentication failed"
  → 404: QuotaError::NoSubscription → UI 走"手动输入"路径

Step 2: usage → 3-month window 循环(因 OpenAI API 限制)
  起点 = 2000-01-01
  while start < now:
    end = min(start + 3 months, now)
    GET {base}/dashboard/billing/usage?start_date=start&end_date=end
    → 200: { total_usage: f64 }  // cents
    total_cents += total_usage
    start = end

Step 3: 计算
  total_usd = sub.hard_limit_usd
  used_usd = total_cents / 100.0  // cents → USD
  remaining_usd = total_usd - used_usd

Step 4: 输出
  QuotaSnapshot {
    total: Some(total_usd),
    used: Some(used_usd),
    remaining: Some(remaining_usd),
    unit: "USD",
    plan_name: Some(sub.plan.title),
    is_valid: true,
    source: QuotaSource::Api,
    fetched_at: now,
  }
```

注意:
- `base_url` 用 `https://api.openai.com`(**不带** `/v1`)
- `total_usage` 单位是 **cents**(参考 Go 代码 L149 `hardLimit - usage/100` 验证)
- 向上取整(`math.Ceil`)消除浮点误差(openai-balance L147)

### REQ-QUOTA-002: DeepSeek fetch_quota 算法

抄自 `references/tier1-direct/cc-switch/src-tauri/src/services/balance.rs` L68-134。

```
Step 1: GET {base}/user/balance
  → 200: { 
           balance_infos: [{ currency, total_balance, granted_balance, topped_up_balance }, ...],
           is_available: bool 
         }
  → 401/403: is_valid=false, invalid_message="Authentication failed"

Step 2: 取 balance_infos[0]
  currency = balance_infos[0].currency  // "CNY" or "USD"
  total_balance = balance_infos[0].total_balance  // 已在正确单位

Step 3: 输出
  QuotaSnapshot {
    total: None,        // DeepSeek API 不返回总额度
    used: None,         // DeepSeek API 不返回已用
    remaining: Some(total_balance),
    unit: currency,
    plan_name: Some(currency),  // cc-switch 用 currency 当 plan_name
    is_valid: is_available,
    invalid_message: is_available ? None : Some("Insufficient balance"),
    source: QuotaSource::Api,
    fetched_at: now,
  }
```

注意:
- `total_balance` 已是正确单位(DeepSeek 不像 OpenAI 用 cents 陷阱)
- cc-switch 模式:15s timeout, Bearer auth, Accept: application/json

### REQ-QUOTA-003: Anthropic fetch_quota 显式 Unsupported

**事实链**:
- Anthropic **不提供** "API key + 直接 usage" 的端点(API key 只能调 `/v1/messages` 计费按 token)
- cc-switch 走 OAuth 路径: `https://api.anthropic.com/api/oauth/usage` 需要 access_token
- access_token 来源: `~/.claude/.credentials.json` 或 macOS Keychain
- **KeyPilot §3.1 死命令:不读不写 CLI config** → OAuth 路径不能抄

**实现**:
```rust
async fn fetch_quota(&self, _base_url: &str, _api_key: &str) -> Result<QuotaSnapshot, QuotaError> {
    Err(QuotaError::Unsupported(
        "Anthropic 需要手动输入额度 (OAuth 路径不兼容 1Password 范式)"
    ))
}
```

**UI fallback**: UI 收到 `QuotaError::Unsupported` → 走"手动输入"路径(Stage 7 modal)。

### REQ-COPY-001: Copy 功能最小化

- Copy 按钮只复制"原始明文 key"或"明文 key + base_url 合并"
- **不实现** format 智能转换(砍 B 选项)
- **不实现** 30s auto-clear(V0.1)
- 用户自行复制粘贴,KeyPilot 不掺和格式

拍板: 2026-06-24(用户决策,"用户自己复制自己粘贴就行")

### REQ-SCHEMA-001: schema_version 1 → 2 migration

```sql
-- 旧版 (v1)
CREATE TABLE providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
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

-- 新版 (v2) — 增量 ALTER
ALTER TABLE providers ADD COLUMN is_preset INTEGER NOT NULL DEFAULT 0;
UPDATE meta SET value = '2' WHERE key = 'schema_version';
```

migration 框架要求: `database.rs::open()` 启动时检查 `schema_version`,缺失列则 ALTER,版本号递增。

---

## MODIFIED

### REQ-PROV-005 (改自 `MVP-范围.md` 必做第 22 行)

**原**:
> 3 家 Provider 预置(OpenAI / DeepSeek / Anthropic)

**改为**:
> 3 家 Provider 预置(OpenAI / Anthropic / DeepSeek),首次启动 seed(`is_preset=1`),允许用户添加同 kind 自定义 provider,uuid 区分,name 字段自由不强制唯一。

### REQ-QUOTA-004 (改自 `MVP-范围.md` 必做第 9-10 行)

**原**:
> 额度查询 - OpenAI: 用 subscription + usage 接口间接计算(因 credit_grants 已被封)
> 额度查询 - DeepSeek: 用 /user/balance 接口(官方支持)

**改为**:
> 额度查询 - OpenAI: 算法见 `openspec/changes/v0.1-spec-alignment/spec.md REQ-QUOTA-001`(基于 `openai-balance` 参考项目)
> 额度查询 - DeepSeek: 算法见 `openspec/changes/v0.1-spec-alignment/spec.md REQ-QUOTA-002`(基于 `cc-switch/services/balance.rs::query_deepseek`)

### REQ-QUOTA-005 (改自 `MVP-范围.md` 必做第 11 行)

**原**:
> 额度查询 - Anthropic: 暂用"手动输入额度"占位(Console API 需 OAuth,V0.1 不做)

**改为**:
> 额度查询 - Anthropic: `fetch_quota` 返回 `QuotaError::Unsupported`,UI 走手动输入路径(Stage 7)。详细见 `openspec/changes/v0.1-spec-alignment/spec.md REQ-QUOTA-003`。

---

## REMOVED

### REQ-PROV-006 (从 `技术方案.md §4.3` 移除)

**移除内容**:
- ProviderKind enum 5 种未来变体: `Gemini`, `Mistral`, `Moonshot`, `Zhipu`
- 这 5 种的 `default_base_url` 表行

**理由**: YAGNI — 实施时不写,避免 5 个 stub 文件(违反 §13.1 "no .todo / unimplemented!()" 铁律)

**影响**:
- `技术方案.md §4.3` 改为只列 3 + Custom
- `webui/src/lib/format.ts` 的 `PROVIDER_SCHEMAS` 改为 4 项(Stage 3 实施时跟进)

### REQ-COPY-002 (从设计中砍)

**移除内容**:
- "Format as Data" 抽象(Provider × Shell format 矩阵)
- 4 种 format: bash / PowerShell / .env / 明文
- 30s auto-clear 安全补丁
- CopyButton.tsx 的 format dropdown UI

**理由**: 用户决策 — "用户自己复制自己粘贴就行"

**影响**:
- `Stage 3` 简化 CopyButton.tsx(只复制明文 + 合并,不实现 format dropdown)
- 不引入新 IPC / 新 TS 文件

---
