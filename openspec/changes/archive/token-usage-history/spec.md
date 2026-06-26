# Token Usage History — Spec

## REQ-TOKEN-001: 数据模型

### REQ-TOKEN-001.1: token_usage_records 表

| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | TEXT PK | `req_{uuid}` |
| `occurred_at` | TEXT (ISO8601 UTC) | 请求开始时间 |
| `finished_at` | TEXT (ISO8601 UTC) | 请求结束时间 |
| `latency_ms` | INTEGER | 端到端延迟 |
| `provider` | TEXT NOT NULL | `openai` / `anthropic` / `deepseek` / `minimax` / `bedrock` / `vertex` / `custom` |
| `model` | TEXT NOT NULL | 具体模型名,如 `gpt-4o` / `claude-3-5-sonnet-20241022` |
| `agent_type` | TEXT | 调用方 agent,如 `claude-code` / `codex` / `opencode` / `cursor` / `manual` / `unknown` |
| `user_id` | TEXT DEFAULT 'default' | 归因用户 |
| `session_id` | TEXT | 多轮对话 session |
| `observation_type` | TEXT DEFAULT 'generation' | `generation` / `embedding` / `tool` |
| `status` | TEXT DEFAULT 'success' | `success` / `error` / `timeout` / `cancelled` |
| `error_code` | TEXT | 可选,provider 错误码 |
| `cache_hit` | INTEGER DEFAULT 0 | 整包 cache 命中 |
| `usage_details` | TEXT (JSON) | token 类型细分,见 §2.1 |
| `cost_details` | TEXT (JSON) | USD 成本细分,见 §2.2 |
| `pricing_version` | TEXT | 定价表版本 |
| `messages` | TEXT (JSON) | 可选,默认 NULL |
| `response` | TEXT (JSON) | 可选,默认 NULL |
| `tags` | TEXT (JSON array) | 可选手工标签 |

### REQ-TOKEN-001.2: usage_details 结构

```json
{
  "input": 12000,
  "output": 800,
  "cache_read_input_tokens": 4000,
  "cache_creation_input_tokens": 0,
  "reasoning_tokens": 0
}
```

- `input` + `cache_read` + `cache_creation` + `output` + `reasoning_tokens` 五类
- 所有字段可选,缺失 = 0
- 不存 `total` (query 时 `SUM(input + cache_read + cache_creation + output + reasoning)`)

### REQ-TOKEN-001.3: cost_details 结构

```json
{
  "input": 0.000024,
  "output": 0.000012,
  "cache_read_input_tokens": 0.0000012,
  "total": 0.0000372
}
```

- `total` = SUM(all fields), 可选
- 精度: Decimal(18,8), 存为 REAL, display 时 round 到 4 位

### REQ-TOKEN-001.4: 归约表

| 表名 | 唯一键 | 用途 |
|---|---|---|
| `daily_agent_model_usage` | (date, agent_type, model, provider) | 日粒度 agent pair 聚合 |
| `daily_model_usage` | (date, model, provider) | 日粒度模型聚合 |

归约表字段:

```text
date            TEXT    (YYYY-MM-DD)
agent_type      TEXT
model           TEXT
provider        TEXT
input_tokens    INTEGER DEFAULT 0
output_tokens   INTEGER DEFAULT 0
cache_read_tokens INTEGER DEFAULT 0
cache_write_tokens INTEGER DEFAULT 0
reasoning_tokens INTEGER DEFAULT 0
total_tokens    INTEGER DEFAULT 0
total_cost_usd  REAL DEFAULT 0
request_count   INTEGER DEFAULT 0
```

## REQ-TOKEN-002: IPC 接口

| IPC 命令 | 方向 | 说明 |
|---|---|---|
| `record_usage` | invoke | 单条记录 (来自 proxy 或手动) |
| `list_usage_records` | invoke | 分页查询,支持 date / agent_type / model / status filter |
| `get_usage_summary` | invoke | 聚合: agent pair 条形图 + 时间序列 + 总览 |
| `import_usage` | invoke | JSONL / CSV 批量导入 |
| `get_pricing` | invoke | 返回 pricing 表 (Top 50 models) |

### REQ-TOKEN-002.1: get_usage_summary 返回结构

```json
{
  "total_tokens": 4800000,
  "total_cost_usd": 123.45,
  "total_requests": 156,
  "agent_pairs": [
    {
      "agent_type": "claude-code",
      "model": "claude-3-5-sonnet-20241022",
      "provider": "anthropic",
      "total_tokens": 4800000,
      "total_cost_usd": 89.50,
      "request_count": 120,
      "token_breakdown": {
        "input": 3200000,
        "output": 400000,
        "cache_read": 1200000
      }
    }
  ],
  "daily_series": [
    {
      "date": "2026-06-25",
      "total_tokens": 150000,
      "total_cost_usd": 3.50,
      "request_count": 12
    }
  ]
}
```

## REQ-TOKEN-003: 前端

### REQ-TOKEN-003.1: Usage 页面路由

- 路由: `/usage` (主窗口 tab 或独立页面)
- 默认视图: agent pair 条形图 (按 total_tokens DESC)

### REQ-TOKEN-003.2: 三种可视化

| 视图 | 组件 | 数据源 |
|---|---|---|
| Agent Pair 条形图 | shadcn Bar Chart | `agent_pairs[]` |
| 时间序列折线图 | shadcn Line Chart | `daily_series[]` |
| 单日热力图 | 自定义 grid | 按 hour × agent_type 聚合 |

### REQ-TOKEN-003.3: 交互

- 点击 agent pair bar → 弹出 detail panel (该 pair 的日序列 + token 构成 pie chart)
- 日期范围选择器: 7d / 30d / 90d / All
- 悬停 tooltip: tokens + cost USD + request count

## REQ-TOKEN-004: 导入

### REQ-TOKEN-004.1: 支持格式

| 格式 | 来源 | 示例 |
|---|---|---|
| JSONL | Claude Code `~/.claude/projects/**/*.jsonl` | 每行一个 JSON，含 `usage` 字段 |
| JSONL | Codex `~/.codex/sessions/**/*.jsonl` | 同上 |
| CSV | 手动导出 | `timestamp,provider,model,input_tokens,output_tokens` |

### REQ-TOKEN-004.2: agent_type 推断规则

```
如果文件路径包含 "claude" → agent_type = "claude-code"
如果文件路径包含 "codex" → agent_type = "codex"
如果文件路径包含 "opencode" → agent_type = "opencode"
否则 → agent_type = "unknown"
```

### REQ-TOKEN-004.3: 去重

- 导入时按 `(agent_type, model, occurred_at, input_tokens, output_tokens)` 五元组去重
- 重复记录跳过,返回 skipped count

## REQ-TOKEN-005: 定价

### REQ-TOKEN-005.1: 内置定价表

- Top 50 models (覆盖 OpenAI / Anthropic / DeepSeek / MiniMax / Google)
- 格式: LiteLLM `model_prices_and_context_window.json` 子集
- 字段: `model`, `input_cost_per_token`, `output_cost_per_token`, `cache_read_cost`, `cache_creation_cost`, `supports_reasoning`

### REQ-TOKEN-005.2: 成本计算

```
cost = (input × input_rate) + (output × output_rate) + (cache_read × cache_read_rate) + (cache_creation × cache_creation_rate) + (reasoning × reasoning_rate)
```

- 无定价的 model → cost = null, 前端显示 "—"
- pricing_version = 内置表 git commit hash

## REQ-TOKEN-006: 边界

- 不加密 usage 数据 (明文, SQLite ACL 保护)
- 最大保留 90 天 (TTL 删除)
- messages/response 默认 NULL (隐私保护)
- 不跨设备同步
