# spec — fix-date-local-timezone

本文件定义系统时区契约。所有章节为 MUST 级别除非显式标 MAY / MUST NOT / EXCEPTION。

## REQ-DATE-LOCAL-001: 系统时区

**MUST** — 整个 keypilot 使用 **Local 时区**作为系统时区(由 OS `TZ` 环境变量 / Windows 时区设置决定,不硬编码)。

**MUST NOT** — 任何 Rust 模块不得使用 `chrono::DateTime::from_timestamp_millis(epoch).format("%Y-%m-%d")` 这种把 epoch 隐式按 UTC 解读的写法;统一过 `timeutil::local_date_str`。

**MUST NOT** — 任何 TS 模块不得使用 `date.toISOString().split("T")[0]` 作为本地日期串;统一过 `formatLocalDate(date)`。

**EXCEPTION** — 外部 API 契约要求 UTC 时保留:
- `provider/openai.rs:93` OpenAI 账单查询(API 按 UTC 日期收)
- `services/auto_import.rs:66/119` 游标 wall-clock seconds
- `services/incremental_import.rs:400/446` 游标 wall-clock seconds
- `services/category.rs:56` / `services/provider.rs:143/205` 用 `Utc::now().timestamp()` 拿秒级 wall-clock

所有 EXCEPTION 路径必须有显式注释 `// intentional Utc: <理由>`。

## REQ-DATE-LOCAL-002: DB 桶语义

**MUST** — `daily_agent_model_usage.date` / `daily_model_usage.date` 列表示 **Local 时区的 calendar day**(不是 UTC 日)。

**Schema**: v6 → v7。新表用 Local 桶;旧 v6 表保留 30 天回滚保险。

**Migration**: `migrate_to_v7()` 在 Tauri `setup()` hook 检测 `schema_version == 6` 时自动调用一次。

**回填**:从 `token_usage_records.occurred_at` GROUP BY `strftime('%Y-%m-%d', occurred_at/1000, 'unixepoch', 'localtime')` 重新聚合,**不依赖旧 date 列**。

## REQ-DATE-LOCAL-003: 后端 helper

```rust
// src-tauri/src/timeutil.rs
pub fn local_date_str(epoch_ms: i64) -> String;
    // 合约: Local 时区的 'YYYY-MM-DD';对 epoch<0 返回 "1970-01-01"

pub fn local_date_to_epoch(s: &str, exclusive: bool) -> Result<i64, AppError>;
    // 合约: 将 'YYYY-MM-DD' 解释为 Local TZ 当日;exclusive=true 返回次日 00:00
    // 错误: NaiveDate parse 失败 → AppError::TokenUsageInvalidFormat
```

旧函数 `iso_date` / `iso_date_to_epoch` 删除。

## REQ-DATE-LOCAL-004: 前端 helper

```ts
// webui/src/lib/format.ts
export function formatLocalDate(date: Date): string;
    // 合约: 浏览器 Local 时区的 'YYYY-MM-DD'
    // 边界:参数必须是合法 Date;否则返回 '1970-01-01'(可选,见 tasks.md)
```

## REQ-DATE-LOCAL-005: 回归测试矩阵

**MUST** — 所有 Rust unit test 必须 `init.sh` 已设 `TZ='Asia/Shanghai'`,否则 TC-01/02/03/06 在 UTC CI 上为 false green。

| ID | 输入 | 期望 | 仓库位置 |
|----|-----|------|---------|
| TC-01 | epoch = absolute value pinned(不用 `Local::now()`) | `local_date_str(1782837000000)` → `"2026-07-01"` | Rust unit test |
| TC-02 | `local_date_to_epoch("2026-07-01", true)` 应等于 `local_date_to_epoch("2026-07-02", false)` | 边界对齐 | Rust unit test |
| TC-03 | `get_periods_summary` 写 epoch = Local 2026-07-01 00:30+08:00 | `month.daily_series` dates ∈ `{"2026-07-01"}`(无 `"2026-06-30"`) | Rust unit test |
| TC-04 | 详情面板 `getDateRange("30d")` 在本地 00:30 触发 | `end_date=today`,`start_date=today - 30d` | TS Playwright |
| TC-05 | `AvgDayCard` MTD 路径(Q4=B,1 天 mock) | avg = today_tokens / 1(不除以 30) | TS Playwright |
| TC-05b | `AvgDayCard` MTD 路径(全 15 天 mock) | avg = sum(month) / 15 | TS Playwright |
| TC-06 | `recompute_costs("2026-06-30"..)` 命中 gap 内 epoch | Local 00:00 起的 row 被刷新 | Rust unit test |
| TC-07 | `migrate_to_v7()` 跑后 `daily_agent_model_usage.date` 各桶 | 全部日期落在 Local 区(用 `schema_version == 7` 路径) | Rust integration test |

## REQ-DATE-LOCAL-006: 行为兼容边界

**MUST** — 以下行为**不变**(对调用方透明):

| 接口 | 旧 | 新 | 调用方需要做的 |
|------|-----|----|---------------|
| `recompute_costs(from, to)` 接受 `"YYYY-MM-DD"` | 解释为 UTC 00:00 | 解释为 Local 00:00 | 调用方(详情面板)同步改用 `formatLocalDate` |
| `get_usage_periods_summary` 返回 IPC shape | `daily_series[i].date` 是 UTC 日 | `daily_series[i].date` 是 Local 日 | 前端不需改(只解读,不写回) |
| IPC 命令名 / 参数 / 返回类型 | — | 不变 | 无 |

## REQ-DATE-LOCAL-007: 不再接受的模式(Anti-pattern)

**MUST NOT** — 后续代码 review 拒绝以下写法:

```rust
// BAD
chrono::DateTime::from_timestamp_millis(epoch).unwrap().format("%Y-%m-%d").to_string()

// BAD
chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string()  // 除非 EXCEPTION 已注释

// BAD — `refresh_daily_rollups` 旧实现,混 Local 字符串 + UTC 解析
chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap().and_utc().timestamp_millis()
```

```ts
// BAD
date.toISOString().split("T")[0]

// BAD
new Date(epoch).toISOString().slice(0, 10)
```

```ts
// BAD — 假设末尾 N 条 = 最近 N 天
series.slice(-days)
```

```ts
// BAD — 用"本月"数据当"最近 N 天"窗口
const trend = monthSummary.daily_series.slice(-30);  // ← 本次 bug 元凶
```

`./init.sh` 的 grep gate 应在 Phase 7 加上对以上模式的硬编码拒绝(可选,见 tasks.md P7)。

## REQ-DATE-LOCAL-008: schema_v6 → v7 migration contract

**MUST** — `database.rs::migrate()` 在 `current == "6"` 时,调用 `migrate_to_v7()` 并把 `meta.schema_version` 改写为 `'7'`。

**MUST** — `migrate_to_v7()` 在单个 unchecked_transaction 内完成:
- 把旧 `daily_agent_model_usage` / `daily_model_usage` 表 RENAME 为 `*_v6`(保留 30 天回滚保险)
- 重建 DDL(与 v6 一致;只换 bucket 语义)
- 从 `token_usage_records.occurred_at` 用 `strftime('%Y-%m-%d', occurred_at/1000, 'unixepoch', 'localtime')` 重 aggregate
- `INSERT` 进新表

**MUST** — `migrate_to_v7()` 是幂等的:第二次调用必须 no-op(顶部 guard `if schema_version >= 7 { return Ok(()) }`)。

**MUST** — Migration **不丢行**:每行 `token_usage_records` 在新表有且仅有一行(由 Local date + 元组其它维度 + GROUP BY 保证)。

**MUST** — Migration **不改 SUM**:新表 `SUM(input_tokens)`, `SUM(output_tokens)`, `SUM(total_cost)`, `SUM(request_count)` 等于 raw `token_usage_records` 同口径 SUM(测试守恒)。

**SHOULD** — `_v6` 表保留 ≥ 30 天;手动 `DROP TABLE …_v6` SQL 命令清理(deferred to post-Phase 2)。

**测试矩阵**(`database.rs::tests`):
- F.1 Empty DB migrate → no crash, schema_version = 7
- F.2 单 epoch `1782809400000` (Local Shanghai `2026-07-01 00:30` / UTC `2026-06-30 16:30`) → 断言 date = `"2026-07-01"`
- F.3 多行同 UTC 桶不同 Local 桶 → 各自进正确 Local 桶,SUM 守恒
- F.4 SUM 守恒总测试(seed N 行已知 input/output/cost,aggregate 后 SUM 等于 raw SUM)
- F.5 dual-table 覆盖:`daily_agent_model_usage` + `daily_model_usage` 都对
- F.6 idempotency:第二次调 `migrate_to_v7()` no-op,不 crash 不 clobber
- F.7 不依赖 `Local::now()`,全部绝对 epoch — 跨 CI TZ 稳定
