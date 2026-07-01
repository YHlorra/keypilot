# fix-date-local-timezone — design

## 时区策略

桌面应用 = **`Local` 是系统时区**。

- DB `daily_*_usage.date` 列存 Local 解释的日期串(`%Y-%m-%d`,代表 Local TZ 的 calendar day)
- 所有 epoch↔date 转换过单一 helper
- 外部已知 UTC 契约保留:`OpenAI` billing API 调用(`provider/openai.rs`)、按文件游标的 wall-clock seconds(`auto_import.rs`, `incremental_import.rs`)—— **显式注释,不重构**

## 单点 helper

```rust
// src-tauri/src/timeutil.rs (NEW)
use chrono::{Local, NaiveDate, TimeZone};

/// Epoch ms → Local 系统时区的 "YYYY-MM-DD" 字符串。
/// Local 是 OS TZ,桌面应用系统时区。
/// 非法 epoch 返回 "1970-01-01"(见 REQ-DATE-LOCAL-003)。
pub fn local_date_str(epoch_ms: i64) -> String {
    Local.timestamp_millis_opt(epoch_ms).single()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".into())
}

/// "YYYY-MM-DD" 字符串 → Local 当日 00:00 的 epoch ms。
/// exclusive=true 时返回次日 00:00(用于 half-open `[from, to)` 区间)。
/// ponytail: 真午夜的 Local wall-clock 永不歧义(DST 跳变在 02–03 时区),
///           所以 .single() 在本 codebase 内是 total,.latest() YAGNI。
pub fn local_date_to_epoch(s: &str, exclusive: bool) -> Result<i64, AppError> {
    let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| AppError::TokenUsageInvalidFormat(format!("invalid date '{s}': {e}")))?;
    let target = if exclusive { date.succ_opt() } else { Some(date) };
    let target = target.ok_or_else(|| AppError::TokenUsageInvalidFormat("date overflow".into()))?;
    Ok(target
        .and_hms_opt(0, 0, 0).unwrap()        // and_hms_opt is total for valid NaiveDate
        .and_local_timezone(Local).single()
        .ok_or_else(|| AppError::TokenUsageInvalidFormat(format!("local datetime '{s}' ambiguous")))?
        .timestamp_millis())
}
```

```ts
// webui/src/lib/format.ts (add)
export function formatLocalDate(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}
```

## 数据流

### 写入(record_usage / auto_import / incremental_import)

```
occurred_at: i64  (epoch ms, 无 TZ 概念)
   │
   ▼ local_date_str(occurred_at)
date: "YYYY-MM-DD"  (Local TZ 解释)
   │
   ▼
INSERT INTO daily_agent_model_usage (date, agent_type, model, provider, ...)
```

### 读取(get_periods_summary)

```
now_local        = Local::now()
month_start_local = now_local.date_naive().with_day(1).and_hms_opt(0, 0, 0)
month_start_ms   = month_start_local.timestamp_millis()
   │
   ▼ local_date_str(month_start_ms)            ← 与写入路径同 TZ
"2026-07-01"
   │
   ▼ SQL
WHERE date >= "2026-07-01" AND date <= "2026-08-01"
```

### 重算(recompute_costs affected_dates)

```
for row in affected_rows:
    affected_dates.insert(local_date_str(*occurred_at));
```

## schema_v7 迁移

**目标:** 把 `daily_agent_model_usage.date` / `daily_model_usage.date` 的语义从 UTC 桶换成 Local 桶。
**重要:** v6 与 v7 表结构完全一致(PK `(date, agent_type, model, provider)` / `(date, model, provider)`),DDL 不变,只是 bucket 语义切换。

**策略:在 `database.rs::migrate()` 加 `else if current == "6"` 分支,从 `token_usage_records.occurred_at` 重 aggregate(实测 173 行,一次性 < 1s)。**

```rust
// src-tauri/src/database.rs (new fn on Database)
pub fn migrate_to_v7(&self) -> Result<(), AppError> {
    // ponytail: idempotency guard — v6→v7 已完成,二次启动直接返回。
    // 由 migrate() 调度层保证一次,但加这一层防御更稳。
    if self.schema_version()? >= 7 {
        return Ok(());
    }

    let conn = self.conn();
    let tx = conn.unchecked_transaction().map_err(AppError::Database)?;

    // 1. 旧表数据全清(同事务内,RENAME 是为调试可读性)
    tx.execute("ALTER TABLE daily_agent_model_usage RENAME TO daily_agent_model_usage_v6", [])
        .map_err(AppError::Database)?;
    tx.execute("ALTER TABLE daily_model_usage         RENAME TO daily_model_usage_v6", [])
        .map_err(AppError::Database)?;

    // 2. 重建 schema(与 v6 DDL 完全一致 —— 见 schema_version 5/6 的 setup_schema)
    tx.execute_batch(DAILY_AGENT_MODEL_USAGE_DDL).map_err(AppError::Database)?;
    tx.execute_batch(DAILY_MODEL_USAGE_DDL)         .map_err(AppError::Database)?;

    // 3. 从 token_usage_records 重 aggregate 到 Local 桶
    //    strftime(..., 'localtime') 是 SQLite 内置,等价 chrono::Local,无需 Rust 循环
    let mut stmt = tx.prepare(
        "SELECT
            strftime('%Y-%m-%d', occurred_at/1000, 'unixepoch', 'localtime') AS date_local,
            agent_type, model, provider_name,
            COUNT(*) AS req_count,
            SUM(input_tokens) AS in_t, SUM(output_tokens) AS out_t,
            SUM(input_tokens + output_tokens
                + cache_creation_input_tokens + cache_read_input_tokens) AS total_t,
            SUM(total_cost) AS cost
         FROM token_usage_records
         GROUP BY 1, 2, 3, 4"
    ).map_err(AppError::Database)?;
    let rows = stmt.query_map([], |row| { ... })?
        .collect::<Result<Vec<_>, _>>().map_err(AppError::Database)?;
    drop(stmt);
    for (date, agent, model, provider, count, inp, out, total, cost) in rows {
        tx.execute(
            "INSERT INTO daily_agent_model_usage (date, agent_type, model, provider, request_count, input_tokens, output_tokens, total_tokens, total_cost)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![date, agent, model, provider, count, inp, out, total, cost],
        ).map_err(AppError::Database)?;
    }

    // 4. 同理 daily_model_usage(无 agent_type 列)
    let mut stmt2 = tx.prepare("SELECT strftime(...) AS date_local, model, provider_name, COUNT(*), ... GROUP BY 1, 2, 3")
        .map_err(AppError::Database)?;
    // (analogous loop)
    drop(stmt2);

    // 5. 单事务 commit — 任一步失败整体 rollback,DB 留在 v6
    tx.commit().map_err(AppError::Database)?;
    Ok(())
}
```

**调度**:通过 `database.rs::migrate()` 既有 `else if current == "6"` 分支调 `self.migrate_to_v7()`,然后 `UPDATE meta SET value = '7'`。**不**改 `lib.rs::setup()` — 现有 `setup()` 已经在调 `db.migrate()`,多加一个分支足够。

**v6 表保留**:30 天回滚保险(实测 173 行,几百 KB,可忽略)。清理机制:暂无自动任务;手动 `DROP TABLE daily_agent_model_usage_v6` SQL 命令(可由用户在 keypilot TTY 内手动执行,或将来加 `cleanup_v6_backups` tauri 命令,本 PR 不在范围内)。

**回填策略取舍:**
- ❌ ALTER … UPDATE 不能用 SQL 函数(`date` 列已存 UTC 字符串,UPDATE 后还是 UTC 串)
- ❌ `INSERT INTO … SELECT DISTINCT date, …` 直接搬旧 UTC 桶 → 新 Local 桶语义错位
- ✅ 从 `token_usage_records.occurred_at`(`i64` epoch)重算 `strftime(..., 'localtime')` → 干净 Local 桶,no row loss,no row duplication
- ✅ 单事务 `unchecked_transaction` 自动 rollback on failure
- ✅ Idempotency guard 在 helper 内 + migrate() 调度层双重防御

**TEST 矩阵**(`database.rs::tests`):
- F.1 Empty DB
- F.2 单条 epoch `1782809400000` (Local Shanghai `2026-07-01 00:30`,UTC `2026-06-30 16:30`) → 断言 date = `"2026-07-01"`
- F.3 多条 row 同一 UTC 桶(epoch `1782809400000` + `1783010400000`)→ 两行都进 Local `"2026-07-01"`(单 PK 行,SUM 正确)
- F.4 SUM 守恒:seed N 行已知 input/output/cost,aggregate 后 SUM 等于 raw SUM
- F.5 dual-table 覆盖:`daily_agent_model_usage` 与 `daily_model_usage` 都正确 re-aggregate
- F.6 idempotency:第二次调 `migrate_to_v7()` 不 crash、不 clobber
- F.7 不用 `Local::now()`,全部绝对 epoch — 跨 CI TZ 稳定

## 决策点

### Q1: 系统时区 Local

✅ **默认 Local**(理由:桌面应用,用户心智模型本地,只需保留已知 UTC 外部契约)。

### Q4: AvgDayCard 语义

✅ **已决定 = 本月至今(路径 B)**

| 维度 | 选 B 的理由 |
|------|-----------|
| 数据源 | 保持 `month?.daily_series`(与旁边"This Month" KPI 同源,语义一致) |
| 标签 | `AVG / DAY (30D)` → `AVG / DAY (MTD)`(Month-to-Date) |
| 分母 | `30` → `sorted.length`(已记录的天数) |
| subLabel | `vs prior 30d` → `N day(s) so far`(动态显示 MTD 跨度) |
| 移除 | `prior30` 的 rolling 比较(不是 rolling 30d,无"前 30 天"概念) |
| 验收用 | 月首 1 号 day=1,avg = today_tokens / 1(不再被 30 稀释) |

### Path B 实施细节

```typescript
// webui/src/components/UsageKpiCards.tsx AvgDayCard 重写
const AvgDayCard = React.memo(function AvgDayCard({
  dailySeries,
}: {
  dailySeries?: { date: string; total_tokens?: number }[];
}) {
  const { avg, deltaLabel } = useMemo(() => {
    if (!dailySeries || dailySeries.length === 0) {
      return { avg: 0, deltaLabel: "" };
    }
    const sorted = [...dailySeries].sort((a, b) => b.date.localeCompare(a.date));
    const days = sorted.length;
    const sum = sorted.reduce((s, d) => s + (d.total_tokens ?? 0), 0);
    return {
      avg: sum / days,                                  // ponytail: 修复 /30 稀释;path B 锁定
      deltaLabel: `${days} day${days === 1 ? "" : "s"} so far`,
    };
  }, [dailySeries]);

  return (
    <KpiCard
      label="AVG / DAY (MTD)"
      value={avg}
      formattedValue={formatTokens(avg)}
      subLabel={deltaLabel}
    />
  );
});
```

## 边界用例对比表

| 场景 | 旧行为(UTC 桶) | 新行为(Local 桶) |
|------|----------------|-----------------|
| 本地 `2026-07-01 02:00+08:00` 写入(= UTC `2026-06-30 18:00`) | `date="2026-06-30"` | `date="2026-07-01"` |
| `get_periods_summary` month 边界 | filter 用 `"2026-06-30"` / `"2026-07-31"` UTC 串 | filter 用 `"2026-07-01"` / `"2026-08-01"` Local 串 |
| `recompute_costs affected_dates` | UTC 串 | Local 串 |
| 详情面板 30d(本地 `00:30` 打开) | `end_date="2026-06-30"`(UTC 截断) | `end_date="2026-07-01"` |
| 跨 TZ 装机用户 | 旧 UTC 桶 = 装机时 TZ 那一刻的 UTC | 回填后 = 当前 TZ 的 Local(可能"日期校正") |

## 路径顺序(已和 Phase 对齐)

```
P1  helper + Rust call sites (无 DB 改动)
       ▼ test green
P2  schema_v7 migrate + boot 自动调 + 回填
       ▼ local DB verify (人工 spot-check 3-5 行)
P3  前端 helper + 3 个 call sites
       ▼ playwright green
P4  sibling fix (AvgDayCard 语义 / opencode_go_limits / deepseek)
       ▼ cargo test 全绿
P5  TZ-intent 注释 (4 处)
P6  TDD 回归 (Rust 3 + TS 3)
P7  blast search + ./init.sh
```

## 测试矩阵(`spec.md` 展开)

| ID | 输入 | 期望 |
|----|-----|------|
| TC-01 | epoch = local 2026-07-01 00:30+08:00 = 1782809400000 | `local_date_str` → `"2026-07-01"` |
| TC-02 | `get_periods_summary` 在 local 2026-07-01 00:30 | `month.daily_series` 只有 07-01,**不**含 06-30 |
| TC-03 | `recompute_costs from="2026-06-30"` | 命中 epoch 落在 `2026-06-30 00:00 local` 起的记录 |
| TC-04 | 详情面板 30d(本地 `00:30`) | `end_date="2026-07-01"`,`start_date="2026-06-01"` |
| TC-05 | `AvgDayCard` MTD (Q4=路径 B) | avg = sum(month.daily_series) / month.daily_series.length;月首 1 号 day=1 时 avg = today_tokens(不再除以 30) |
| TC-06 | OpenAI billing 调用 | 仍发送 UTC 日期字符串,行为不变 |
| TC-07 | schema_v7 migrate 后 `month.daily_series` | 各桶日期落在 Local 区(UTC+8 装机,7/1 00:30 写入的行归到 `2026-07-01`) |

## Anti-pattern 监督清单(P7 跑)

```bash
# 在 src-tauri/ 跑
rg -n 'from_timestamp_millis.*\.format|iso_date\b|iso_date_to_epoch\b'
rg -n 'Utc::now\(\)\.timestamp' services/

# 在 webui/src 跑
rg -n '\.toISOString\(\)\.split\("T"\)\[0\]'
rg -n 'new Date\([^)]*\).*86400'

# 期望命中应仅:
# - timeutil.rs 新 helper 内部
# - P5 加注释的 4 处已知 UTC 契约
```
