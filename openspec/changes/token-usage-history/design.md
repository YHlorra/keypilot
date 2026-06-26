# Token Usage History — Design

## 1. 架构总览

```
┌──────────────┐     import_usage / record_usage      ┌─────────────────┐
│ Agent Logs   │ ─────────────────────────────────────▶ │  Rust Backend   │
│ (JSONL/CSV)  │                                       │  (IPC layer)    │
└──────────────┘                                       └────────┬────────┘
                                                             │
                                                    ┌────────▼────────┐
                                                    │   SQLite        │
                                                    │   token_usage   │
                                                    │   _records      │
                                                    │   + rollups     │
                                                    └────────┬────────┘
                                                             │
                                                             │ invoke
                                                    ┌────────▼────────┐
                                                    │   Frontend      │
                                                    │   /usage page   │
                                                    │   Bar + Line +  │
                                                    │   Heatmap       │
                                                    └─────────────────┘
```

## 2. 后端设计

### 2.1 数据层 (database.rs)

新增 3 个表,在 `schema_version` 迁移中添加:

```sql
-- 事件表
CREATE TABLE IF NOT EXISTS token_usage_records (
    id                    TEXT PRIMARY KEY,
    occurred_at           TEXT NOT NULL,
    finished_at           TEXT,
    latency_ms            INTEGER,
    provider              TEXT NOT NULL,
    model                 TEXT NOT NULL,
    agent_type            TEXT DEFAULT 'unknown',
    user_id               TEXT DEFAULT 'default',
    session_id            TEXT,
    observation_type      TEXT DEFAULT 'generation',
    status                TEXT DEFAULT 'success',
    error_code            TEXT,
    cache_hit             INTEGER DEFAULT 0,
    usage_details         TEXT NOT NULL DEFAULT '{}',
    cost_details          TEXT DEFAULT NULL,
    pricing_version       TEXT,
    messages              TEXT DEFAULT NULL,
    response              TEXT DEFAULT NULL,
    tags                  TEXT DEFAULT '[]'
);

CREATE INDEX IF NOT EXISTS idx_token_usage_occurred ON token_usage_records(occurred_at);
CREATE INDEX IF NOT EXISTS idx_token_usage_agent_model ON token_usage_records(agent_type, model, occurred_at);

-- 归约表: agent pair 日粒度
CREATE TABLE IF NOT EXISTS daily_agent_model_usage (
    date               TEXT NOT NULL,
    agent_type         TEXT NOT NULL,
    model              TEXT NOT NULL,
    provider           TEXT NOT NULL,
    input_tokens       INTEGER DEFAULT 0,
    output_tokens      INTEGER DEFAULT 0,
    cache_read_tokens  INTEGER DEFAULT 0,
    cache_write_tokens INTEGER DEFAULT 0,
    reasoning_tokens   INTEGER DEFAULT 0,
    total_tokens       INTEGER DEFAULT 0,
    total_cost_usd     REAL DEFAULT 0,
    request_count      INTEGER DEFAULT 0,
    PRIMARY KEY (date, agent_type, model, provider)
);

-- 归约表: 模型日粒度
CREATE TABLE IF NOT EXISTS daily_model_usage (
    date            TEXT NOT NULL,
    model           TEXT NOT NULL,
    provider        TEXT NOT NULL,
    input_tokens    INTEGER DEFAULT 0,
    output_tokens   INTEGER DEFAULT 0,
    cache_read_tokens INTEGER DEFAULT 0,
    cache_write_tokens INTEGER DEFAULT 0,
    reasoning_tokens INTEGER DEFAULT 0,
    total_tokens    INTEGER DEFAULT 0,
    total_cost_usd  REAL DEFAULT 0,
    request_count   INTEGER DEFAULT 0,
    PRIMARY KEY (date, model, provider)
);
```

迁移: 在 `database.rs` 的 `migrate()` 中添加 version bump (当前 → v4).

### 2.2 Service 层 (services/token_usage.rs)

**职责**: 纯业务逻辑,无 Tauri 依赖。

```rust
pub struct TokenUsageService { /* 依赖 Database */ }

impl TokenUsageService {
    pub async fn record_usage(&self, req: UsageRecordInput) -> Result<UsageRecord, AppError>;
    pub async fn list_records(&self, filter: UsageFilter, page: usize, per_page: usize) -> Result<Vec<UsageRecord>, AppError>;
    pub async fn get_summary(&self, filter: UsageFilter) -> Result<UsageSummary, AppError>;
    pub async fn import_jsonl(&self, content: &str, source_hint: Option<&str>) -> Result<ImportResult, AppError>;
    pub async fn import_csv(&self, content: &str) -> Result<ImportResult, AppError>;
    pub async fn refresh_daily_rollups(&self, date: &str) -> Result<(), AppError>;
}
```

**成本计算**:  ingest 时查内置 pricing 表,计算 cost_details. 无定价的 model → cost_details = null.

**归约刷新**: `record_usage` 后同步 upsert 到 `daily_agent_model_usage` + `daily_model_usage`. 导入批量结束后 bulk refresh.

### 2.3 IPC 层 (commands/token_usage.rs)

5 个 handler,注册到 `lib.rs`:

```rust
#[tauri::command]
async fn record_usage(state: State<'_, Arc<Mutex<AppState>>>, input: UsageRecordInput) -> Result<UsageRecord, AppError>

#[tauri::command]
async fn list_usage_records(state: State<'_, Arc<Mutex<AppState>>>, filter: UsageFilter) -> Result<PaginatedResponse<UsageRecord>, AppError>

#[tauri::command]
async fn get_usage_summary(state: State<'_, Arc<Mutex<AppState>>>, filter: UsageFilter) -> Result<UsageSummary, AppError>

#[tauri::command]
async fn import_usage(state: State<'_, Arc<Mutex<AppState>>>, content: String, format: ImportFormat, source_hint: Option<String>) -> Result<ImportResult, AppError>

#[tauri::command]
async fn get_pricing(state: State<'_, Arc<Mutex<AppState>>>) -> Result<Vec<PricingEntry>, AppError>
```

### 2.4 定价表 (services/pricing.rs)

内置 JSON,编译时嵌入:

```rust
const BUILTIN_PRICING: &str = include_str!("../../../data/pricing.json");
```

`data/pricing.json` 结构:

```json
{
  "version": "2026-06-25-lite-50",
  "models": [
    {
      "model": "gpt-4o",
      "input_cost_per_token": 2.5e-6,
      "output_cost_per_token": 1.0e-5,
      "cache_read_cost": 2.5e-7,
      "supports_reasoning": false
    }
  ]
}
```

Top 50 models,来源 LiteLLM pricing DB,手动筛选高频模型.

## 3. 前端设计

### 3.1 路由

新增 `/usage` 页面,在 `App.tsx` 中加 tab 或独立 route.

### 3.2 组件

| 组件 | 路径 | 职责 |
|---|---|---|
| `UsagePage` | `webui/src/pages/UsagePage.tsx` | 主容器,tab 切换 |
| `AgentPairChart` | `webui/src/components/AgentPairChart.tsx` | 条形图,按 total_tokens DESC |
| `UsageTimeSeries` | `webui/src/components/UsageTimeSeries.tsx` | 折线图,日粒度 |
| `UsageHeatmap` | `webui/src/components/UsageHeatmap.tsx` | 小时 × agent_type grid |
| `UsageDetailPanel` | `webui/src/components/UsageDetailPanel.tsx` | 点击 bar 后的 detail |
| `ImportModal` | `webui/src/components/ImportModal.tsx` | JSONL/CSV 导入 |
| `useUsage` | `webui/src/hooks/useUsage.ts` | TanStack Query hooks |

### 3.3 可视化方案

**条形图 (Agent Pair)**:

```
claude-code + claude-3-5-sonnet  ████████████████████ 4.8M  $89.50
codex     + gpt-4o              ██████████████       2.8M  $45.20
opencode  + minimax-M3          ████                 0.5M   $2.10
```

- 水平条形图,按 total_tokens DESC
- 每行: agent_type + model + 条形 + token 数 + cost USD
- 颜色: 按 provider (chrome=gray, status=grass/amber/red)

**折线图 (时间序列)**:

- 选 7d / 30d / 90d, X 轴 = date, Y 轴 = total_tokens
- 可选 stacked: input / output / cache_read / reasoning 分层

**热力图 (单日)**:

- X 轴 = hour (0-23), Y 轴 = agent_type
- 颜色深浅 = token 量,悬停显示具体数值

### 3.4 交互流程

```
UsagePage 加载
  → useUsage.getSummary({range: "30d"})
    → Rust: SELECT ... FROM daily_agent_model_usage WHERE date >= ...
    → 渲染 AgentPairChart + UsageTimeSeries

点击 AgentPairChart 中某 bar
  → 设置 selectedPair 状态
  → 渲染 UsageDetailPanel
    → useUsage.getRecords({agent_type, model, date_range})
    → 显示该 pair 的日序列 + token 构成 pie chart

点击 "导入"
  → 打开 ImportModal
  → 选择文件 + 格式
  → invoke import_usage(content, format, source_hint)
  → 显示导入结果 (imported / skipped / errors)
```

## 4. 导入格式

### 4.1 JSONL (Claude Code)

Claude Code 的 `~/.claude/projects/**/*.jsonl` 每行格式:

```json
{
  "timestamp": "2026-06-25T03:14:07Z",
  "model": "claude-3-5-sonnet-20241022",
  "usage": {
    "input_tokens": 12000,
    "output_tokens": 800,
    "cache_creation_input_tokens": 0,
    "cache_read_input_tokens": 4000
  },
  "sessionId": "abc123"
}
```

映射:
- `timestamp` → `occurred_at`
- `model` → `model`
- 路径包含 `claude` → `agent_type = "claude-code"`
- `usage` → `usage_details`
- `sessionId` → `session_id`

### 4.2 JSONL (Codex)

Codex 的 `~/.codex/sessions/**/*.jsonl` 每行格式:

```json
{
  "timestamp": "2026-06-25T03:14:07Z",
  "model": "gpt-4o",
  "usage": {
    "prompt_tokens": 12000,
    "completion_tokens": 800,
    "prompt_tokens_details": {
      "cached_tokens": 4000
    }
  },
  "conversationId": "def456"
}
```

映射:
- 路径包含 `codex` → `agent_type = "codex"`
- `prompt_tokens` → `input`
- `completion_tokens` → `output`
- `prompt_tokens_details.cached_tokens` → `cache_read_input_tokens`
- `conversationId` → `session_id`

### 4.3 CSV

```csv
timestamp,provider,model,agent_type,input_tokens,output_tokens,cache_read_tokens,cache_write_tokens,reasoning_tokens
2026-06-25T03:14:07Z,openai,gpt-4o,codex,12000,800,4000,0,0
```

## 5. 非功能需求

| 需求 | 目标 |
|---|---|
| 导入性能 | 10K 条/秒 (批量 transaction) |
| 查询延迟 | P95 < 200ms (归约表查询) |
| 保留期 | 90 天 TTL |
| 存储上限 | 预估 1M 条 ≈ 500MB SQLite |

## 6. 风险与缓解

| 风险 | 缓解 |
|---|---|
| 定价表过时 | 版本化,内置表手动更新; V0.3 自动拉 LiteLLM |
| 导入格式不兼容 | 提供错误详情,跳过不可解析行 |
| 隐私合规 | messages/response 默认 NULL,不存储 |
| 性能 (大导入) | 批量 upsert + 归约表异步刷新 |
