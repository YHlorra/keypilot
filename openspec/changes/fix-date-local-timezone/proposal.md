# fix-date-local-timezone — Proposal

## Problem

整个 keypilot 没有任何"系统时区"的统一概念:

- **Rust 写入 / 聚合路径**:`from_timestamp_millis(epoch).format("%Y-%m-%d")` 把 epoch 当 UTC 解读
  - `src-tauri/src/database.rs:412-415` 写入 `daily_agent_model_usage.date`
  - `src-tauri/src/services/token_usage.rs:66-70` `iso_date()`
  - `src-tauri/src/services/token_usage.rs:1143-1144` `recompute_costs affected_dates`
- **Rust 查询路径**:`get_periods_summary` 的 today/month 边界用 `Local::now()` 算(本地午夜 epoch),SQL filter 用 `iso_date()` 得到的 **UTC** 日期串 → 本地 epoch 与 UTC 日期串不在同一时区
- **前端日期格式化**:`toISOString().split("T")[0]` 永远 UTC;跨本地午夜偏移 1 天
- **DB 桶语义**:`daily_agent_model_usage.date` 列存 UTC 桶,跟用户心智模型(本地日历日)错位

**触发表征(2026-07-01,UTC+8 装机):**
- Trend chart 只画 2 点连直线(06-30 + 07-01)
- `AvgDayCard` 显示错的日均(除以 30 而非实际有数据的 1 天)
- 本月数据包含前一日 UTC 16:00-24:00 的事件
- 跨时区用户 / 跨本地午夜打开详情面板会少 1 天

## Proposed Solution

系统时区定 **Local**(桌面应用由 OS 决定)。所有 epoch↔date-string 转换走单一 helper:

```rust
// src-tauri/src/timeutil.rs (NEW)
pub fn local_date_str(epoch_ms: i64) -> String                    // "YYYY-MM-DD" in Local
pub fn local_date_to_epoch(s: &str, exclusive: bool) -> Result<i64, AppError>
```

```ts
// webui/src/lib/format.ts (add)
export function formatLocalDate(date: Date): string                // "YYYY-MM-DD" in browser Local
```

DB 升 schema **v7**:`daily_agent_model_usage.date` / `daily_model_usage.date` 列改成本地日期。回填策略:从 `token_usage_records.occurred_at` 重新 aggregate(实测 173 行,一次性 < 1s,无需 truncate)。

旧 `iso_date` / `iso_date_to_epoch` 函数删除,改名以明示 TZ 意图。

## Scope

**P0(必须):**
- `database.rs:412-415` 写入按 Local
- `services/token_usage.rs:66-70 iso_date` → `timeutil::local_date_str`
- `services/token_usage.rs:511-576 get_periods_summary` boundary/bucket 同 TZ
- schema_v7 migrate + 回填

**P1(应做):**
- `services/token_usage.rs:1143-1144 recompute affected_dates`
- `services/opencode_go_limits.rs:183-213 monthly_used`(line 358 `Utc::now` → `Local::now`)
- `webui/.../UsageHeatmapCalendar.tsx:31`
- `webui/.../UsageDetailPanel.tsx:56-64 getDateRange`
- `webui/.../UsageKpiCards.tsx:79 AvgDayCard`(改数据源或改标签二选一,见 design §决策点)
- `services/deepseek_balance_history.rs:130-160` 核验 row ts 来源后改

**P2(契约注释):**
- `provider/openai.rs:93` OpenAI 账单 UTC 注释
- `auto_import.rs:66/119` 游标 Utc 注释
- `incremental_import.rs:400/446` 游标 Utc 注释
- `commands/token_usage.rs:242` `iso_date_to_epoch` 重命名 + 注释

## Out of Scope

- `token_usage_records.occurred_at` / `recorded_at`(本身 epoch ms,无 TZ 概念)
- 已知外部 UTC 契约(`provider/openai.rs` 的 OpenAI billing、按文件游标的 wall-clock seconds)
- IPC shape(只改 helper 行为,**不增/不删** Tauri command)
- 用户可见 UI 文案(此 fix 不引入新页面)

## Why Now

- 已爆(2026-07-01 趋势图 + AvgDayCard)
- 同一根因扩散到:trend chart / AvgDayCard / OpenCode 月度用量 / 详情面板 / 热力图,每月 1 号全中
- 只做"前端用 all_time 替换"是绕开症状,DB 旧 UTC 桶数据仍然错;只有 schema_v7 换桶 + 回填才彻底
- Phase 1(纯 helper 替换 + 测试)能修 UI 90% 表现;Phase 2(migrate)是 DB 根本修

## Risk

| 风险 | 缓解 |
|------|------|
| 数据迁移:旧 UTC 桶必须重算 Local 桶 | 用 `token_usage_records.occurred_at` 重新 GROUP BY `local_date_str(occurred_at)`,不依赖旧 date 列 |
| 跨时区用户:跨 TZ 装机后看到日期校正 | changelog 明示;v6 表留 30 天回滚保险 |
| 性能:record_usage 多一次 chrono 转换 | QPS ≤ 10,无可测影响 |
| 向后兼容:`recompute_costs` 接受 date string 现在按 Local | 仅 caller(详情面板同步改用 `formatLocalDate`) |
| 既有测试:`cargo test` 部分 case 隐式假设 UTC | 全部用例改显式 Local,新增跨午夜测试 |

## Rollout

1. **P1**:helper + Rust call sites(无 DB 改动)→ `pnpm tsc --noEmit && cargo check && cargo test` 全绿
2. **P2**:schema_v7 migrate 函数 + boot 自动调用 + 回填 → 本地 DB verify `month.daily_series` 在 Local 桶里
3. **P3**:前端 helper + 3 个 call sites → Playwright 全绿
4. **P4**:sibling fix(AvgDayCard 语义、opencode_go_limits、deepseek)→ `cargo test` 全绿
5. **P5**:TZ-intent 注释 → grep `Utc::now|format\(.*%Y|toISOString` 复查
6. **P6**:TDD 6 个 case → Rust 3 + TS 3
7. **P7**:`rg 'from_timestamp_millis\|format\(.*%Y\|toISOString'` 全检 + `./init.sh` 全绿

## Open Decisions(等用户拍板)

- **Q1**:系统时区 Local 已建议,需用户确认
- **Q4**:`AvgDayCard` 语义 — ✅ **已决定 = 本月至今(MTD, Month-to-Date)**
  - 路径 B:数据源保持 `month?.daily_series`(不切到 `allTime`),标签改 `AVG / DAY (MTD)`,分母改 `month.daily_series.length`(除以实际已记录天数,不再除以 30)
  - 详见 `design.md` §Path B 实施 + `tasks.md` T4.1
