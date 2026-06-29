# KeyPilot 额度测试 + Token 使用量计算 参考方案

> 决策日期: 2026-06-28
> 真相源: `keypilot-dev/` 实际代码 (基于 CCswitch 架构的 Rust/Tauri 本地化实现)
> 适用范围: keypilot-dev 自身的扩展、对齐、补全
> 关注重点: ① Token 用量与成本计算  ② SQLite Schema + 缓存策略

---

## 0. 一句话定位

KeyPilot 把 CCswitch 的"额度查询 + 用量统计"能力,翻译成 **Rust trait + SQLite 单一数据源 + pricing.json 静态价格表** 的三层架构:

```
┌────────────────────────────────────────────────────────────────┐
│  Frontend (React/TS)                                           │
│  useQuota / useUsage / QuotaBadge / UsageDetailPanel / ...      │
└──────────────────────────┬─────────────────────────────────────┘
                           │  Tauri #[command]
┌──────────────────────────▼─────────────────────────────────────┐
│  Command Layer  (commands/quota.rs, commands/token_usage.rs)   │
│  薄包装: IPC DTO ↔ Rust 业务类型转换                            │
└──────────────────────────┬─────────────────────────────────────┘
                           │
┌──────────────────────────▼─────────────────────────────────────┐
│  Service Layer  (services/*.rs)                                │
│  TokenUsageService  /  PricingService  /  AgentParser           │
│  auto_import orchestrator                                       │
└──────────────────────────┬─────────────────────────────────────┘
                           │
┌──────────────────────────▼─────────────────────────────────────┐
│  Adapter Layer  (provider/*.rs)                                │
│  ProviderAdapter trait + 5 个实现 (OpenAI/DeepSeek/...)       │
└──────────────────────────┬─────────────────────────────────────┘
                           │
        ┌──────────────────┴──────────────────┐
        ▼                                     ▼
   SQLite (单一真相源)                  pricing.json (静态价格表)
   schema v4, 7 张表                   include_str!() 编译期内联
```

---

## 1. SQLite Schema 详解 (当前 v4)

### 1.1 表清单

| 表名 | 角色 | 行级粒度 |
|---|---|---|
| `meta` | KV 元数据 (schema_version / preset_seeded / theme / last_auto_import) | 全局 key-value |
| `categories` | 凭证分组 | 1 row = 1 分类 |
| `providers` | 凭证主表 | 1 row = 1 provider |
| `provider_fields` | 任意 KV 字段 (base_url / api_key / host / port...) | 1 row = 1 字段 |
| `quota_cache` | 额度查询缓存 | 1 row = 1 provider 的最近一次快照 |
| `token_usage_records` | Token 用量原始记录 | 1 row = 1 次 LLM 调用 |
| `daily_agent_model_usage` | 按 (date, agent, model, provider) 日汇总 | 1 row = 1 日 1 组合 |
| `daily_model_usage` | 按 (date, model, provider) 日汇总 (跨 agent) | 1 row = 1 日 1 组合 |

### 1.2 Quota 相关表

#### `quota_cache` — 额度查询缓存

```sql
CREATE TABLE IF NOT EXISTS quota_cache (
    provider_id    INTEGER PRIMARY KEY,        -- 1:1 关联 providers.id
    snapshot_json  TEXT    NOT NULL,           -- QuotaSnapshot 序列化 JSON
    fetched_at     INTEGER NOT NULL,           -- unix epoch 秒
    source         TEXT    NOT NULL DEFAULT 'auto',  -- 'auto' | 'manual'
    FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
);
```

**关键设计**:
- `provider_id` 是主键 → 每个 provider 至多 1 条缓存
- `source` 区分数据来源 (这是 v0.1-rev2 加的列,见 [database.rs L147-161](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/database.rs#L147-L161) 的幂等 ALTER)
- `snapshot_json` 存 `QuotaSnapshot` 全字段 (total / used / remaining / unit / level / reset_at)

### 1.3 Token Usage 相关表 (Migration v3 → v4)

完整 DDL 见 [src-tauri/data/migrations/v3_to_v4.sql](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/data/migrations/v3_to_v4.sql)。

#### `token_usage_records` — 原始记录表

```sql
CREATE TABLE IF NOT EXISTS token_usage_records (
    id                            TEXT    PRIMARY KEY,        -- FNV-1a 64 位十六进制 (见 §3.3)
    agent_type                    TEXT    NOT NULL,           -- 'claude-code' | 'opencode' | 'codex' | ...
    model                         TEXT    NOT NULL,           -- 'gpt-4o' / 'claude-opus-4' / ...
    provider_name                 TEXT    NOT NULL,           -- 'openai' / 'anthropic' / 'unknown'
    occurred_at                   INTEGER NOT NULL,           -- 调用发生时间 (epoch 秒,来自日志)
    recorded_at                   INTEGER NOT NULL,           -- 入库时间 (chrono::Utc::now())
    session_id                    TEXT,                       -- 可选 (Claude Code sessionId)
    request_id                    TEXT,                       -- 可选 (Claude Code uuid)
    -- 5 维 token 拆解 (KeyPilot 核心模型)
    prompt_tokens                 INTEGER DEFAULT 0,          -- 别名 input_tokens (向后兼容字段)
    completion_tokens             INTEGER DEFAULT 0,          -- 别名 output_tokens
    input_tokens                  INTEGER DEFAULT 0,          -- 真实 input
    output_tokens                 INTEGER DEFAULT 0,          -- 真实 output
    cache_read_input_tokens       INTEGER DEFAULT 0,          -- 命中缓存的输入
    cache_creation_input_tokens   INTEGER DEFAULT 0,          -- 写入缓存的输入
    reasoning_tokens              INTEGER DEFAULT 0,          -- o1/o3 推理 token
    total_tokens                  INTEGER DEFAULT 0,          -- 5 维求和 (服务端算)
    -- 5 维成本拆解 (PricingService 算)
    prompt_cost                   REAL    DEFAULT 0.0,
    completion_cost               REAL    DEFAULT 0.0,
    cache_read_cost               REAL    DEFAULT 0.0,
    cache_creation_cost           REAL    DEFAULT 0.0,
    reasoning_cost                REAL    DEFAULT 0.0,
    total_cost                    REAL    DEFAULT 0.0,
    currency                      TEXT    DEFAULT 'USD',
    pricing_version               TEXT,                       -- pricing.json 的 version 字段
    usage_details                 TEXT    DEFAULT '{}',       -- 原始 usage JSON (审计/调试)
    cost_details                  TEXT    DEFAULT NULL        -- 5 维成本 + pricing_missing_for JSON
);

CREATE INDEX IF NOT EXISTS idx_token_usage_occurred      ON token_usage_records(occurred_at);
CREATE INDEX IF NOT EXISTS idx_token_usage_agent_model  ON token_usage_records(agent_type, model, occurred_at);
```

**设计要点**:
- `id` 用 FNV-1a 哈希 → 跨导入幂等 (重复导入同一行 JSONL 不会重复计数)
- 同时存 `prompt_tokens` 和 `input_tokens` 是历史遗留 (兼容早期 Codex 命名),新代码用 `input_tokens`
- `total_tokens` 是服务端计算 (5 维 sum),不信任客户端传值
- `cost_details` JSON 比 5 个 REAL 列多了 `pricing_missing_for` 字段 → 前端可以提示"该模型未在价格表中"

#### `daily_agent_model_usage` — 按 agent/model/provider 日汇总

```sql
CREATE TABLE IF NOT EXISTS daily_agent_model_usage (
    date           TEXT    NOT NULL,           -- 'YYYY-MM-DD' (ISO 日期)
    agent_type     TEXT    NOT NULL,
    model          TEXT    NOT NULL,
    provider       TEXT    NOT NULL,
    request_count  INTEGER DEFAULT 0,
    input_tokens   INTEGER DEFAULT 0,
    output_tokens  INTEGER DEFAULT 0,
    total_tokens   INTEGER DEFAULT 0,
    total_cost     REAL    DEFAULT 0.0,
    PRIMARY KEY (date, agent_type, model, provider)
);
```

#### `daily_model_usage` — 跨 agent 的日汇总

```sql
CREATE TABLE IF NOT EXISTS daily_model_usage (
    date           TEXT    NOT NULL,
    model          TEXT    NOT NULL,
    provider       TEXT    NOT NULL,
    request_count  INTEGER DEFAULT 0,
    input_tokens   INTEGER DEFAULT 0,
    output_tokens  INTEGER DEFAULT 0,
    total_tokens   INTEGER DEFAULT 0,
    total_cost     REAL    DEFAULT 0.0,
    PRIMARY KEY (date, model, provider)
);
```

**为什么有两张日汇总表**:
- `daily_agent_model_usage` 保留 agent 维度 → 给"agent × model"配对图 ([AgentPairChart.tsx](file:///e:/Desktop/workspace/keypilot-dev/webui/src/components/AgentPairChart.tsx)) 用
- `daily_model_usage` 跨 agent 聚合 → 给"模型成本排行"用 (V0.2 计划)

### 1.4 Schema 版本与 Migration

**当前**: `meta.schema_version = '4'`

**升级流程** ([database.rs L166-181](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/database.rs#L166-L181)):

```rust
pub fn migrate(&self) -> Result<(), AppError> {
    let current: String = self.conn.query_row(
        "SELECT value FROM meta WHERE key = 'schema_version'", [], |row| row.get(0))?;
    if current == "3" {
        let sql = include_str!("../data/migrations/v3_to_v4.sql");
        self.conn.execute_batch(sql)?;
        self.conn.execute("UPDATE meta SET value = '4' WHERE key = 'schema_version'", [])?;
    }
    Ok(())
}
```

**幂等性**:
- 所有 DDL 用 `CREATE TABLE IF NOT EXISTS` / `CREATE INDEX IF NOT EXISTS`
- `quota_cache.source` 列用 `pragma_table_info` 探测后 `ALTER TABLE ADD COLUMN` (见 [database.rs L149-161](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/database.rs#L149-L161))
- 新增列时务必给 `DEFAULT` 值

---

## 2. 额度测试 (Quota) — Provider Adapter 模式

### 2.1 Trait 抽象

完整代码: [src-tauri/src/provider/adapter.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/provider/adapter.rs)

```rust
#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    fn preset(&self) -> &'static str;
    fn can_test(&self) -> bool;            // 是否支持 validate_key
    fn can_fetch_quota(&self) -> bool;     // 是否支持 fetch_quota (Anthropic=false)
    async fn validate_key(&self, base_url: &str, api_key: &str)
        -> Result<(), ValidateError>;
    async fn fetch_quota(&self, base_url: &str, api_key: &str)
        -> Result<QuotaSnapshot, QuotaError>;
}

/// 工厂函数 — preset 字符串 → 适配器实例
pub fn adapter_for(preset: &str) -> Option<Box<dyn ProviderAdapter>> {
    match preset {
        "openai"    => Some(Box::new(crate::provider::openai::OpenAiAdapter)),
        "deepseek"  => Some(Box::new(crate::provider::deepseek::DeepSeekAdapter)),
        "anthropic" => Some(Box::new(crate::provider::anthropic::AnthropicAdapter)),
        "github"    => Some(Box::new(crate::provider::github::GitHubAdapter)),
        "postgres"  => Some(Box::new(crate::provider::postgres::PostgresAdapter)),
        _ => None,
    }
}
```

**核心约束**:
- 每家 LLM 一个 struct,实现同一 trait → UI 层只关心 `QuotaSnapshot` 一种格式
- 工厂函数用 `match preset` 而非 URL 子串检测 (CCswitch 用 URL 子串,KeyPilot 改进点)

### 2.2 QuotaSnapshot — 统一返回结构

```rust
pub struct QuotaSnapshot {
    pub total:     Option<f64>,  // 总额度 (PostgreSQL 无)
    pub used:      f64,         // 已用 (必有)
    pub remaining: Option<f64>,  // 剩余 (PostgreSQL 无)
    pub unit:      String,      // 'USD' | 'CNY' | 'req' | 'GB' | 'token'
    pub level:     Option<String>,  // 'green' | 'amber' | 'red' | 'ruby' (UI 配色)
    pub reset_at:  Option<i64>, // 重置时间 epoch 秒 (PostgreSQL 无)
}
```

**各家 provider 形状对照** (见 [types.rs L94-116](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/types.rs#L94-L116) 注释):

| Provider | total | used | remaining | unit | reset_at | 备注 |
|---|---|---|---|---|---|---|
| OpenAI | hard_limit_usd | usage.total_usage/100 | total - used | USD | 无 | 3 月窗口迭代 |
| DeepSeek | balance | used | balance - used | CNY | 无 | `/user/balance` |
| Anthropic | — | — | — | — | — | `Unsupported` 错误,UI 走手动输入 |
| GitHub | 5000 | used | remaining | req | reset_at | `/rate_limit` |
| PostgreSQL | None | pg_database_size | None | GB | None | 只看已用 |

### 2.3 OpenAI 适配器示例 (重点)

完整代码: [src-tauri/src/provider/openai.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/provider/openai.rs)

```rust
async fn fetch_quota(&self, base_url: &str, api_key: &str) -> Result<QuotaSnapshot, QuotaError> {
    let client = reqwest::Client::new();
    let base = base_url.trim_end_matches('/');

    // Step 1: GET subscription → hard_limit_usd (已经是 USD,不是 cents)
    let sub: SubResp = client
        .get(format!("{}/dashboard/billing/subscription", base))
        .header("Authorization", format!("Bearer {}", api_key))
        .timeout(Duration::from_secs(10))
        .send().await?.json().await?;
    let hard_limit = sub.hard_limit_usd;

    // Step 2: 3 月窗口迭代 (从 2000-01-01 累计到现在)
    let mut total_cents: f64 = 0.0;
    let mut start = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
    let now_date = Utc::now().date_naive();

    while start < now_date {
        let end_raw = start.checked_add_months(Months::new(3)).unwrap_or(now_date);
        let end = if end_raw > now_date { now_date } else { end_raw };

        let usage: UsageResp = client
            .get(format!("{}/dashboard/billing/usage?start_date={}&end_date={}",
                base, start.format("%Y-%m-%d"), end.format("%Y-%m-%d")))
            .header("Authorization", format!("Bearer {}", api_key))
            .timeout(Duration::from_secs(10))
            .send().await?.json().await?;
        total_cents += usage.total_usage;
        start = end;
    }

    // cents → USD,ceil 消除浮点误差
    let used = (total_cents / 100.0).ceil();
    let remaining = hard_limit - used;

    // 颜色等级 (前端 QuotaBadge 用)
    let level = match remaining / hard_limit {
        r if r > 0.5  => "green",
        r if r > 0.2  => "amber",
        r if r > 0.05 => "red",
        _             => "ruby",
    };

    Ok(QuotaSnapshot { total: Some(hard_limit), used, remaining: Some(remaining),
        unit: "USD".into(), level: Some(level.into()), reset_at: None })
}
```

**算法来源**: OpenAI 在 4 月份封禁了 `credit_grants` 接口 (只允许浏览器 token),替代方案是 `subscription - usage` 间接算余额。**3 月窗口迭代** 是因为 OpenAI `/usage` 接口单次最多返回 3 个月数据。

### 2.4 fetch_quota 命令层 — 三阶段锁策略

完整代码: [src-tauri/src/commands/quota.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/commands/quota.rs)

```rust
pub async fn fetch_quota_by_state(state: &AppState, id: i64) -> Result<QuotaSnapshot, AppError> {
    // Phase A: 读 provider + 检查缓存 (同步 SQLite,短锁)
    let (preset, base_url, api_key, cached) = {
        let db = state.db.lock().unwrap();
        // SELECT preset, fields, cache_snapshot ...
        (preset, base_url, api_key, cached)
    }; // 锁在这里释放

    if let Some(snapshot) = cached { return Ok(snapshot); }

    // Phase B: 调用 adapter (async HTTP,无锁)
    let adapter = adapter_for(&preset)?;
    let snapshot = adapter.fetch_quota(&base_url, &api_key).await?;

    // Phase C: 写缓存 (同步 SQLite,短锁)
    {
        let db = state.db.lock().unwrap();
        // INSERT INTO quota_cache ... 'auto'
    }

    Ok(snapshot)
}
```

**为什么三阶段**:
- Phase A 和 C 的 SQLite 锁各只持有几毫秒
- Phase B 的 HTTP 请求可能 10-15 秒,不能持锁 → 否则 UI 全卡死
- Tauri 命令运行在 async runtime 上,SQLite 同步操作用 `tauri::async_runtime::spawn_blocking` 包裹 (见 [commands/token_usage.rs L159](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/commands/token_usage.rs#L159))

### 2.5 TTL + Manual Source 缓存策略

```rust
const QUOTA_CACHE_TTL_SECS: i64 = 900; // 15 分钟 (REQ-QUOTA-DISPLAY-001)

let cached: Option<QuotaSnapshot> = db.conn
    .query_row("SELECT snapshot_json, fetched_at, source FROM quota_cache WHERE provider_id = ?1",
        [id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
    .ok()
    .filter(|(_, fetched_at, source)| {
        source == "manual" || now - fetched_at < QUOTA_CACHE_TTL_SECS
    })
    .and_then(|(json, _, _)| serde_json::from_str(&json).ok());
```

**两条缓存路径**:

| source | 写入路径 | TTL | 用途 |
|---|---|---|---|
| `auto` | `fetch_quota` 命令成功后自动写入 | 15 分钟 | OpenAI/DeepSeek/GitHub/Postgres |
| `manual` | `set_manual_quota` 命令用户手输 | 永久 (直到覆盖) | Anthropic (无 quota API) |

**手动覆盖命令** ([commands/quota.rs L150-191](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/commands/quota.rs#L150-L191)):

```rust
#[tauri::command]
pub async fn set_manual_quota(state: State<'_, AppState>, req: SetManualQuotaRequest)
    -> Result<(), AppError>
{
    // 1. 验证 provider 存在 (避免孤儿行)
    // 2. INSERT INTO quota_cache ... 'manual'
    //    ON CONFLICT(provider_id) DO UPDATE SET source='manual', fetched_at=now
}
```

---

## 3. Token 用量与成本计算 (重点)

### 3.1 PricingService — 静态价格表

完整代码: [src-tauri/src/services/pricing.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/pricing.rs)

#### 价格表结构 ([src-tauri/data/pricing.json](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/data/pricing.json))

```json
{
  "version": "2026-06-25",
  "models": [
    {
      "model": "gpt-4o",
      "provider": "OpenAI",
      "input_price_per_1m": 2.50,            // USD / 1M tokens
      "output_price_per_1m": 10.00,
      "cache_read_price_per_1m": 1.25,        // 命中缓存的输入,价格 = 0.5 × input
      "cache_creation_price_per_1m": 2.50,   // 写入缓存的输入,价格 = 1.0 × input
      "reasoning_price_per_1m": null          // 该模型不收 reasoning 费
    }
    // ...
  ]
}
```

#### 加载方式 — 编译期内联

```rust
static PRICING_DATA: Lazy<PricingData> = Lazy::new(|| {
    let raw = include_str!("../../data/pricing.json");
    serde_json::from_str(raw).expect("Invalid pricing.json")
});
```

**为什么 `include_str!` 而非运行时读文件**:
- 编译期检查: pricing.json 损坏 → 编译失败 (fail-fast)
- 零运行时 IO: 不需要 `app.path()` 异步获取路径
- 单二进制部署: 打包后无需携带 pricing.json
- **代价**: 改价格表必须重新 `cargo build` (V0.1 可接受,因为定价变化不频繁)

#### 5 维成本计算公式

```rust
pub fn calculate_token_usage_cost(
    &self,
    model: &str,
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_creation_tokens: i64,
    reasoning_tokens: i64,
) -> Result<TokenUsageCostBreakdown, AppError> {
    let Some(entry) = self.lookup(model) else {
        // 未知模型 → 全 0,但记录 pricing_missing_for 标志
        return Ok(TokenUsageCostBreakdown {
            input_cost: 0.0, output_cost: 0.0, cache_read_cost: 0.0,
            cache_creation_cost: 0.0, reasoning_cost: 0.0, total_cost: 0.0,
            currency: "USD".into(),
            pricing_missing_for: Some(model.to_string()),
        });
    };

    let per_token = |price_per_1m: Option<f64>, tokens: i64| -> f64 {
        price_per_1m.map(|r| r * tokens as f64 / 1_000_000.0).unwrap_or(0.0)
    };

    let input_cost         = per_token(entry.input_price_per_1m,         input_tokens);
    let output_cost        = per_token(entry.output_price_per_1m,        output_tokens);
    let cache_read_cost    = per_token(entry.cache_read_price_per_1m,    cache_read_tokens);
    let cache_creation_cost = per_token(entry.cache_creation_price_per_1m, cache_creation_tokens);
    let reasoning_cost     = per_token(entry.reasoning_price_per_1m,     reasoning_tokens);

    let total_cost = input_cost + output_cost + cache_read_cost
                   + cache_creation_cost + reasoning_cost;

    Ok(TokenUsageCostBreakdown {
        input_cost, output_cost, cache_read_cost, cache_creation_cost,
        reasoning_cost, total_cost, currency: "USD".into(),
        pricing_missing_for: None,
    })
}
```

**核心公式**:

```
total_cost = (input_price    × input_tokens
            + output_price   × output_tokens
            + cache_read_price    × cache_read_tokens
            + cache_creation_price × cache_creation_tokens
            + reasoning_price     × reasoning_tokens) / 1_000_000
```

**为什么 5 维而不是 2 维 (input/output)**:
- Claude 系列有 `cache_read` (0.1×) 和 `cache_creation` (1.25×),不算就低估成本
- o1/o3 系列有 `reasoning_tokens`,价格单独计
- 透明性: 用户能在 [UsageDetailPanel](file:///e:/Desktop/workspace/keypilot-dev/webui/src/components/UsageDetailPanel.tsx) 看到每个维度多少钱

### 3.2 TokenUsageService — 7 大方法

完整代码: [src-tauri/src/services/token_usage.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/token_usage.rs)

| 方法 | 输入 | 输出 | 用途 |
|---|---|---|---|
| `record_usage` | id + UsageRecordInput | TokenUsageRecord | 单条记录 (自动算成本) |
| `list_records` | filter + page + per_page | Vec<TokenUsageRecord> | 分页查询 |
| `get_summary` | filter | UsageSummary (totals + agent_pairs + daily_series) | 看板汇总 |
| `import_jsonl` | content + source_hint | ImportResult | 批量导入 Claude/Codex JSONL |
| `import_csv` | content | ImportResult | 批量导入 CSV |
| `import_opencode_db` | db_path | ImportResult | 从 opencode.db 导入 |
| `refresh_daily_rollups` | date | () | 重算某天的日汇总 |
| `count_records` | — | u64 | 检查是否已有数据 (>100 跳过自动导入) |
| `recompute_costs` | from_epoch + to_epoch | RecomputeResult | 批量重算成本 + 联动 daily rollups |

#### `record_usage` 完整流程

```rust
pub fn record_usage(&self, id: &str, input: UsageRecordInput) -> Result<TokenUsageRecord, AppError> {
    // 1. 算成本 (5 维 + total + currency + pricing_missing_for)
    let cost = self.pricing.calculate_token_usage_cost(
        &input.model, input.input_tokens, input.output_tokens,
        input.cache_read_input_tokens, input.cache_creation_input_tokens,
        input.reasoning_tokens)?;

    // 2. 算 total_tokens (5 维 sum,服务端算不信任客户端)
    let total_tokens = input.input_tokens + input.output_tokens
                     + input.cache_read_input_tokens + input.cache_creation_input_tokens
                     + input.reasoning_tokens;

    // 3. 拼 cost_details JSON (比 5 个 REAL 列多了 pricing_missing_for)
    let cost_json = serde_json::json!({
        "currency": cost.currency,
        "input": cost.input_cost, "output": cost.output_cost,
        "cache_read": cost.cache_read_cost, "cache_creation": cost.cache_creation_cost,
        "reasoning": cost.reasoning_cost, "total": cost.total_cost,
        // 仅当模型不在 pricing.json 时加这字段
        "pricing_missing_for": cost.pricing_missing_for,
    });

    // 4. 持久化 (PK 冲突 → TokenUsageDuplicate 错误)
    let record = TokenUsageRecord { /* ... */ };
    match db.insert_token_usage(&record) {
        Ok(()) => Ok(record),
        Err(AppError::Database(rusqlite::Error::SqliteFailure(err, _)))
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            Err(AppError::TokenUsageDuplicate(id.to_string())),
        Err(e) => Err(e),
    }
}
```

### 3.3 FNV-1a 去重主键 (核心设计)

```rust
fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn deterministic_id(agent: &str, model: &str, occurred_at: i64, input: i64, output: i64) -> String {
    let key = format!("{agent}|{model}|{occurred_at}|{input}|{output}");
    format!("{:016x}", fnv1a_64(key.as_bytes()))
}
```

**为什么用 FNV-1a 而不是 UUID**:
- **跨导入幂等**: 同一行 JSONL 解析出的 (agent, model, ts, input, output) 一定相同 → 同一 ID → 第二次导入走 `ON CONFLICT` 跳过
- **零依赖**: FNV-1a 是 64 位哈希,纯算术,不需要 `uuid` crate (虽然项目其他地方用了 uuid)
- **冲突率**: 64 位哈希在 1000 万条记录下冲突概率约 0.0003% (生日悖论),可接受
- **可调试**: 16 位十六进制短,日志里好看

**冲突处理**: PK 冲突时返回 `AppError::TokenUsageDuplicate(id)`,调用方决定是 skip 还是 error (批量导入走 skip)。

### 3.4 cost_details JSON 结构

```json
{
  "currency": "USD",
  "input": 2.50,
  "output": 10.00,
  "cache_read": 1.25,
  "cache_creation": 2.50,
  "reasoning": 0.0,
  "total": 16.25,
  "pricing_missing_for": null      // 模型不在 pricing.json 时填模型名
}
```

**为什么除了 5 个 REAL 列还要 JSON**:
- `pricing_missing_for` 是元信息,不是数值,放列里类型不统一
- 前端 [UsageDetailPanel](file:///e:/Desktop/workspace/keypilot-dev/webui/src/components/UsageDetailPanel.tsx) 直接 `JSON.parse` 一次拿全部维度
- 未来加维度 (例如 `image_cost`) 不需要改 schema

---

## 4. 缓存与汇总策略

### 4.1 四层缓存模型

```
┌─────────────────────────────────────────────────────────────────┐
│  Layer 0: quota_cache 过期清理(启动时,7 天 TTL)               │
│  ─────────────────────────────────────────────                   │
│  应用启动时清理 source='auto' 且 fetched_at < now - 7天 的行    │
│  source='manual' 行不受影响,失败不阻塞启动                     │
└─────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────┐
│  Layer 1: quota_cache (15 min TTL)                              │
│  ─────────────────────────────────────────────                   │
│  存 QuotaSnapshot,减少对 LLM quota API 的调用                    │
│  manual source 永久有效,auto source 15 min 过期                │
└─────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────┐
│  Layer 2: token_usage_records (永久原始数据)                    │
│  ─────────────────────────────────────────────                   │
│  每次 LLM 调用一行,FNV-1a 去重                                 │
│  写入时即计算 5 维成本 + total_tokens                          │
└─────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────┐
│  Layer 3: daily_agent_model_usage + daily_model_usage (日汇总)  │
│  ─────────────────────────────────────────────                   │
│  UPSERT 模式,写入新 record 时同步累加                          │
│  refresh_daily_rollups 可重算某天                               │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 日汇总 UPSERT 模式

完整代码: [database.rs L297-322](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/database.rs#L297-L322)

```sql
INSERT OR REPLACE INTO daily_agent_model_usage
  (date, agent_type, model, provider, request_count, input_tokens, output_tokens, total_tokens, total_cost)
VALUES (
  ?1, ?2, ?3, ?4,
  COALESCE((SELECT request_count FROM daily_agent_model_usage
            WHERE date=?1 AND agent_type=?2 AND model=?3 AND provider=?4), 0) + 1,
  COALESCE((SELECT input_tokens   FROM daily_agent_model_usage
            WHERE date=?1 AND agent_type=?2 AND model=?3 AND provider=?4), 0) + ?5,
  -- output_tokens / total_tokens / total_cost 同样模式
);
```

**为什么不用触发器**:
- 触发器是隐式的,debug 困难
- COALESCE + INSERT OR REPLACE 显式可读
- 单次写入 ~5ms,可接受 (KeyPilot 不是高吞吐系统)

### 4.3 日汇总重算流程 (修复脏数据)

完整代码: [services/token_usage.rs L676-772](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/token_usage.rs#L676-L772)

```rust
pub fn refresh_daily_rollups(&self, date: &str) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction()?;

    // 1. 删掉该天的所有汇总行
    tx.execute("DELETE FROM daily_agent_model_usage WHERE date = ?1", [date])?;
    tx.execute("DELETE FROM daily_model_usage        WHERE date = ?1", [date])?;

    // 2. 从 token_usage_records 重算 (group by agent/model/provider)
    let mut stmt = tx.prepare(
        "SELECT agent_type, model, provider_name, COUNT(*),
                SUM(input_tokens), SUM(output_tokens), SUM(total_tokens), SUM(total_cost)
         FROM token_usage_records
         WHERE occurred_at >= ?1 AND occurred_at < ?2
         GROUP BY agent_type, model, provider_name"
    )?;
    // ... 写回 daily_agent_model_usage

    // 3. 同样方式重算 daily_model_usage (group by model/provider)
    // ...

    tx.commit()?;
}
```

**何时用**:
- 测试场景下手动改了 daily 表 → 调一次修复
- V0.2 计划: pricing.json 更新后批量重算所有历史成本

### 4.4 自动导入流程 (ParseStats 可观测性)

完整代码: [services/auto_import.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/auto_import.rs)

```rust
pub fn scan_and_import_if_empty(svc: &TokenUsageService) -> AutoImportSummary {
    if db_has_records(svc, 100) {
        return AutoImportSummary::empty();   // 已有数据,跳过
    }
    scan_and_import(svc)
}

pub fn scan_and_import(svc: &TokenUsageService) -> AutoImportSummary {
    let parsers = default_parsers();  // [OpencodeParser, ClaudeCodeParser]
    for parser in parsers {
        if !parser.is_available() { continue; }
        let outcome = parser.parse()?;     // 返回 (records, ParseStats)
        for input in outcome.records {
            let id = deterministic_id(&input);
            match svc.record_usage(&id, input) {
                Ok(_) => imported += 1,
                Err(TokenUsageDuplicate) => skipped += 1,  // 幂等
                Err(e) => errors.push(e),
            }
        }
    }
    // 结果写入 meta.last_auto_import,前端 mount 时读 → toast 提示
}
```

**ParseStats 关键设计** (见 [services/agent_parser.rs L18-40](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/agent_parser.rs#L18-L40)):

```rust
pub struct ParseStats {
    pub files_scanned: u32,
    pub lines_scanned: u32,
    pub lines_matched: u32,
    pub lines_parse_errored: u32,
    pub sample_errors: Vec<String>,  // 前 3 条错误,格式 "{file}:{line_no}: {reason}"
}
```

**为什么记录这些**:
- "imported: 0" 不告诉用户原因 (是文件不存在? 还是格式不对?)
- `files_scanned: 0` → 告诉用户路径错了
- `lines_parse_errored: 385` → 告诉用户日志格式变了,需要更新 parser
- `sample_errors` 上限 3 条 → 避免 385 文件的扫描日志爆掉 meta 表

### 4.5 AgentParser trait — 可插拔扩展

```rust
pub trait AgentParser: Send + Sync {
    fn agent_type(&self)   -> &'static str;    // 'claude-code' / 'opencode' / ...
    fn display_name(&self) -> &'static str;    // 'Claude Code' (UI 显示)
    fn default_path(&self) -> PathBuf;          // ~/.claude/projects/ 等
    fn is_available(&self) -> bool;             // 路径存在?
    fn parse(&self) -> Result<ParseOutcome, AppError>;
}

pub fn default_parsers() -> Vec<Box<dyn AgentParser>> {
    vec![
        Box::new(OpencodeParser::new()),
        Box::new(ClaudeCodeParser::new()),
        // 加新 agent 只需在这里加一行
    ]
}
```

**已有实现**:
- [OpencodeParser](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/agent_parser_opencode.rs): 读 `opencode.db` SQLite (`session` 表)
- [ClaudeCodeParser](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/agent_parser_claude_code.rs): walk `~/.claude/projects/**/*.jsonl`,只解析 `type:"assistant"` 行的 `message.usage`

**ClaudeCodeParser 关键逻辑** ([agent_parser_claude_code.rs L159-235](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/agent_parser_claude_code.rs#L159-L235)):

```rust
fn parse_assistant(v: &Value, ...) -> Option<UsageRecordInput> {
    let message = v.get("message")?;
    let model = message.get("model")?.as_str()?.to_string();
    let usage = message.get("usage")?;
    let ts_str = v.get("timestamp")?.as_str()?;
    let occurred_at = chrono::DateTime::parse_from_rfc3339(ts_str)?.timestamp_millis();

    Some(UsageRecordInput {
        agent_type: "claude-code".into(),
        model,
        provider_name: derive_provider(&model),  // 'claude-*' → anthropic, 'gpt-*' → openai
        occurred_at,
        session_id: v.get("sessionId").and_then(|x| x.as_str()).map(String::from),
        request_id: v.get("uuid").and_then(|x| x.as_str()).map(String::from),
        input_tokens:  usage.get("input_tokens").and_then(|x| x.as_i64()).unwrap_or(0),
        output_tokens: usage.get("output_tokens").and_then(|x| x.as_i64()).unwrap_or(0),
        cache_read_input_tokens: usage.get("cache_read_input_tokens")...,
        cache_creation_input_tokens: usage.get("cache_creation_input_tokens")...,
        // ...
    })
}
```

---

## 5. 数据流图 (端到端)

### 5.1 额度查询流程

```
用户点 ProviderCard "刷新额度" 按钮
         │
         ▼
useQuota.fetch(id)  →  invoke('fetch_quota', { id })
         │
         ▼
commands::quota::fetch_quota_by_state(state, id)
         │
         ├─ Phase A: lock db → 读 provider + quota_cache → unlock
         │            │
         │            ├─ 命中 (auto 未过期 / manual) → return cache
         │            └─ miss → 进入 Phase B
         │
         ├─ Phase B: adapter_for(preset).fetch_quota(base_url, api_key)
         │            │
         │            ├─ OpenAI: GET /subscription + 循环 GET /usage
         │            ├─ DeepSeek: GET /user/balance
         │            ├─ GitHub: GET /rate_limit
         │            ├─ PostgreSQL: SELECT pg_database_size()
         │            └─ Anthropic: Err(Unsupported) → UI 走 ManualQuotaModal
         │
         └─ Phase C: lock db → INSERT INTO quota_cache 'auto' → unlock
                      │
                      ▼
                返回 QuotaSnapshot → useQuota.setState → QuotaBadge 更新颜色
```

### 5.2 Token 用量导入流程

```
应用启动 (lib.rs setup)
         │
         ▼
scan_and_import_if_empty(svc)
         │
         ├─ svc.count_records() > 100? → 跳过
         │
         └─ scan_and_import(svc):
              │
              ├─ OpencodeParser.parse()
              │     └─ parse_opencode_db_records(db_path) → Vec<UsageRecordInput>
              │
              ├─ ClaudeCodeParser.parse()
              │     └─ walk_jsonl_dir(~/.claude/projects/**/*.jsonl)
              │         → 解析 type:"assistant" 行 → Vec<UsageRecordInput>
              │
              └─ for input in records:
                    id = fnv1a_64(agent|model|ts|in|out)
                    svc.record_usage(id, input)
                      │
                      ├─ PricingService.calculate_token_usage_cost (5 维)
                      ├─ INSERT INTO token_usage_records (PK 冲突 → Duplicate)
                      └─ UPSERT daily_agent_model_usage + daily_model_usage
                              │
                              ▼
                    写入 meta.last_auto_import = { entries, parse_stats, ... }
                              │
                              ▼
前端 App.tsx mount → invoke('get_last_auto_import') → 显示 toast
```

### 5.3 Usage 看板查询流程

```
用户打开 Usage 页面
         │
         ▼
useUsage.invoke('get_usage_summary', { filter })
         │
         ▼
commands::token_usage::get_usage_summary_by_state(state, filter)
         │
         ▼
TokenUsageService::get_summary(filter)
         │
         ├─ SELECT date, SUM(...) FROM daily_agent_model_usage GROUP BY date
         │   → daily_series (折线图)
         │
         ├─ SELECT agent, model, provider, SUM(...)
         │   FROM daily_agent_model_usage
         │   GROUP BY agent, model, provider
         │   ORDER BY SUM(total_tokens) DESC LIMIT 10
         │   → agent_pairs (配对图)
         │
         └─ totals: SUM(daily_series) → KPI cards
                   │
                   ▼
UsageKpiCards + UsageTimeSeries + AgentPairChart + UsageHeatmapCalendar 渲染
```

---

## 6. 现状评估 + 改进建议

### 6.1 已实现且成熟的部分 ✅

| 能力 | 状态 | 关键文件 |
|---|---|---|
| Provider Adapter trait + 5 实现 | ✅ | [provider/adapter.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/provider/adapter.rs) |
| Quota 15min TTL + manual override | ✅ | [commands/quota.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/commands/quota.rs) |
| 5 维成本计算 (input/output/cache_read/cache_creation/reasoning) | ✅ | [services/pricing.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/pricing.rs) |
| FNV-1a 跨导入幂等去重 | ✅ | [services/token_usage.rs L20-33](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/token_usage.rs#L20-L33) |
| Claude Code JSONL 自动解析 | ✅ | [services/agent_parser_claude_code.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/agent_parser_claude_code.rs) |
| opencode.db 自动导入 | ✅ | [services/token_usage.rs L162-233](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/token_usage.rs#L162-L233) |
| ParseStats 可观测性 (3 条错误样本) | ✅ | [services/agent_parser.rs L18-40](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/agent_parser.rs#L18-L40) |
| daily_agent_model_usage UPSERT 累加 | ✅ | [database.rs L299-310](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/database.rs#L299-L310) |
| 日汇总重算 (refresh_daily_rollups) | ✅ | [services/token_usage.rs L676-772](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/token_usage.rs#L676-L772) |
| quota_cache 启动时过期清理 (7 天 TTL) | ✅ | [database.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/database.rs) `purge_expired_quota_cache` + [lib.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/lib.rs) setup |
| Codex JSONL 解析 + cache_read/cache_creation/reasoning = 0 测试覆盖 | ✅ | [services/token_usage.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/token_usage.rs) `import_jsonl_codex_format_with_cache` |
| PricingService provider lookup(新模型不再需要改代码) | ✅ | [services/pricing.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/pricing.rs) `lookup_provider_by_model` + [services/agent_parser_claude_code.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/agent_parser_claude_code.rs) `derive_provider` |
| 历史成本重算命令 (pricing.json 升级后批量重算) | ✅ | [services/token_usage.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/token_usage.rs) `recompute_costs` + [commands/token_usage.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/commands/token_usage.rs) `recompute_costs` IPC |

### 6.2 可补强 / 待扩展的部分 ⚠️

#### G1. JSONL 行级解析没有 unit test 覆盖 Codex 格式 ✅ 已完成(见上表)

[services/token_usage.rs L491-521](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/token_usage.rs#L491-L521) 有 Codex 解析分支,但 `mod tests` 里只测了 Claude 格式。建议补:

```rust
#[test]
fn import_jsonl_codex_format_with_cache() {
    // Codex 不带 cache_read/cache_creation,确认这两个字段在记录里是 0
}
```

#### G2. pricing.json 升级后历史成本不重算 ✅ 已完成(见上表)

`record_usage` 用当时的 `pricing_version` 写入 `token_usage_records.pricing_version`。如果 pricing.json 涨价,历史记录保持旧价格 → 看板上的"今日成本"是新价,"历史成本"是旧价,无法对齐。

**建议**: 加一个 `recompute_costs_for_date_range(from, to)` 命令,在某天 pricing.json 更新后批量重算:

```rust
pub fn recompute_costs(&self, from: i64, to: i64) -> Result<u32, AppError> {
    // SELECT id, model, input_tokens, ... FROM token_usage_records WHERE occurred_at BETWEEN
    // 对每行: cost = pricing.calculate_token_usage_cost(...)
    // UPDATE token_usage_records SET ... WHERE id = ?
    // 同时 refresh_daily_rollups(每个受影响日期)
}
```

#### G3. `prompt_tokens` / `completion_tokens` 历史遗留列

[services/token_usage.rs L309-311](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/token_usage.rs#L309-L311) 同时写 `prompt_tokens = input_tokens` 和 `input_tokens = input_tokens`。这是为兼容 Codex 命名。

**建议**: V0.2 删除 `prompt_tokens` / `completion_tokens` 列,统一用 `input_tokens` / `output_tokens`。需要 migration v4 → v5。

#### G4. ClaudeCodeParser 的 `derive_provider` 模型前缀表硬编码 ✅ 已完成(见上表)

[services/agent_parser_claude_code.rs L241-259](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/agent_parser_claude_code.rs#L241-L259) 用 `if model.starts_with("claude-")` 推断 provider。新模型出现时(如 `gemini-*`)需要改代码。

**建议**: 把映射表抽到 `pricing.json` 的 `provider` 字段,直接 lookup。

#### G5. quota_cache 没有过期清理任务 ✅ 已完成(见上表)

`auto` source 15 min 后会自然 miss,但旧 row 不会被删除 (除非 provider 被 CASCADE 删除)。长期运行数据库会膨胀。

**建议**: 加一个启动时清理:

```rust
conn.execute(
    "DELETE FROM quota_cache
     WHERE source = 'auto' AND fetched_at < ?1",
    [now_secs() - 86400 * 7],  // 删 7 天前的 auto 缓存
)?;
```

#### G6. 没有汇率换算

`PricingService` 写死 `currency: "USD"`。DeepSeek 返回 CNY,前端 QuotaBadge 显示时要不要换算成 USD 做横向对比? 这是产品决策,目前 V0.1 没做。

#### G7. pricing.json 的 `include_str!` 编译期检查是双刃剑

如果用户在 release 包里想自己改价格表 (例如测 Beta 定价),必须重编译。

**建议 V0.2**: 双路径加载:
1. 优先 `app_data_dir/pricing.json` (运行时,允许用户改)
2. fallback `include_str!` (编译期,保证可启动)

---

## 7. 关键文件索引

### Rust 后端

| 文件 | 角色 | 行数 |
|---|---|---|
| [provider/adapter.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/provider/adapter.rs) | ProviderAdapter trait + 工厂 | ~80 |
| [provider/openai.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/provider/openai.rs) | OpenAI quota 算法 (subscription - usage) | ~150 |
| [provider/deepseek.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/provider/deepseek.rs) | DeepSeek quota | ~80 |
| [provider/anthropic.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/provider/anthropic.rs) | 返回 Unsupported | ~30 |
| [provider/github.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/provider/github.rs) | GitHub /rate_limit | ~80 |
| [provider/postgres.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/provider/postgres.rs) | pg_database_size | ~80 |
| [services/pricing.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/pricing.rs) | PricingService + 5 维成本公式 | ~120 |
| [services/token_usage.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/token_usage.rs) | TokenUsageService 6 方法 + FNV-1a + JSONL/CSV 解析 | ~1100 |
| [services/agent_parser.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/agent_parser.rs) | AgentParser trait + ParseStats | ~80 |
| [services/agent_parser_claude_code.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/agent_parser_claude_code.rs) | Claude Code JSONL 解析 | ~320 |
| [services/agent_parser_opencode.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/agent_parser_opencode.rs) | opencode.db 解析 | ~100 |
| [services/auto_import.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/services/auto_import.rs) | 启动时自动扫描 + 导入 | ~230 |
| [commands/quota.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/commands/quota.rs) | fetch_quota + set_manual_quota (三阶段锁) | ~190 |
| [commands/token_usage.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/commands/token_usage.rs) | record/list/summary/import IPC | ~530 |
| [database.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/database.rs) | SQLite schema v4 + 5 张表 DDL | ~600 |
| [types.rs](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/src/types.rs) | QuotaSnapshot / TokenUsageRecord / PricingEntry / ... | ~280 |
| [data/pricing.json](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/data/pricing.json) | 静态价格表 (Top 50 模型) | ~500 |
| [data/migrations/v3_to_v4.sql](file:///e:/Desktop/workspace/keypilot-dev/src-tauri/data/migrations/v3_to_v4.sql) | token_usage 三张表 DDL | ~56 |

### 前端 (React/TS)

| 文件 | 角色 |
|---|---|
| [hooks/useQuota.ts](file:///e:/Desktop/workspace/keypilot-dev/webui/src/hooks/useQuota.ts) | TanStack Query 包 fetch_quota |
| [hooks/useUsage.ts](file:///e:/Desktop/workspace/keypilot-dev/webui/src/hooks/useUsage.ts) | TanStack Query 包 usage summary/list |
| [components/QuotaBadge.tsx](file:///e:/Desktop/workspace/keypilot-dev/webui/src/components/QuotaBadge.tsx) | 额度徽章 (green/amber/red/ruby) |
| [components/ManualQuotaModal.tsx](file:///e:/Desktop/workspace/keypilot-dev/webui/src/components/ManualQuotaModal.tsx) | Anthropic 手动输入弹窗 |
| [components/UsageDetailPanel.tsx](file:///e:/Desktop/workspace/keypilot-dev/webui/src/components/UsageDetailPanel.tsx) | 5 维成本展开 |
| [components/UsageKpiCards.tsx](file:///e:/Desktop/workspace/keypilot-dev/webui/src/components/UsageKpiCards.tsx) | 总 token / 总成本 / 总请求数 |
| [components/UsageTimeSeries.tsx](file:///e:/Desktop/workspace/keypilot-dev/webui/src/components/UsageTimeSeries.tsx) | 折线图 |
| [components/UsageHeatmapCalendar.tsx](file:///e:/Desktop/workspace/keypilot-dev/webui/src/components/UsageHeatmapCalendar.tsx) | GitHub 风格热力图 |
| [components/AgentPairChart.tsx](file:///e:/Desktop/workspace/keypilot-dev/webui/src/components/AgentPairChart.tsx) | agent × model 配对图 |
| [components/ImportModal.tsx](file:///e:/Desktop/workspace/keypilot-dev/webui/src/components/ImportModal.tsx) | 手动导入 JSONL/CSV 弹窗 |
| [pages/UsagePage.tsx](file:///e:/Desktop/workspace/keypilot-dev/webui/src/pages/UsagePage.tsx) | 用量看板页 |

---

## 8. 一键验证 (硬约束)

参考 [AGENTS.md §10](file:///e:/Desktop/workspace/keypilot-dev/AGENTS.md) Sprint Contract:

```bash
# 编译
cargo check --manifest-path src-tauri/Cargo.toml
cargo test  --manifest-path src-tauri/Cargo.toml

# Schema v4 必有
grep "schema_version" src-tauri/src/database.rs
grep "source TEXT NOT NULL DEFAULT 'auto'" src-tauri/src/database.rs

# Token usage 三张表
grep "token_usage_records"        src-tauri/data/migrations/v3_to_v4.sql
grep "daily_agent_model_usage"    src-tauri/data/migrations/v3_to_v4.sql
grep "daily_model_usage"          src-tauri/data/migrations/v3_to_v4.sql

# 5 维成本公式
grep "calculate_token_usage_cost" src-tauri/src/services/pricing.rs
grep "reasoning_cost"             src-tauri/src/services/pricing.rs

# FNV-1a 去重
grep "fnv1a_64" src-tauri/src/services/token_usage.rs
grep "fnv1a_64" src-tauri/src/services/auto_import.rs

# Quota TTL
grep "QUOTA_CACHE_TTL_SECS" src-tauri/src/commands/quota.rs
grep "source == \"manual\"" src-tauri/src/commands/quota.rs
```

---

*最后更新: 2026-06-28 — 基于 keypilot-dev 当前实现整理*
