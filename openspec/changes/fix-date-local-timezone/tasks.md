# tasks — fix-date-local-timezone

每个任务自带**验收条件**(RED→GREEN TDD 模式)。Phase 内可并行,Phase 间串行。

---

## Phase 1 — helper + Rust call sites(无 DB 改动)

### T1.0 钉 CI TZ(RED 闸门前提)

- 文件:`init.sh` §1 顶部加 `export TZ='Asia/Shanghai'`
- 验收:在 UTC CI 机器上跑 `./init.sh`,`cargo test` 仍能触碰到 Local 时区分支
- ⚠️ 必须在 T1.1 测试前就位,否则 TC-01/02/03/06 对 UTC CI 是 false green

### T1.1 `[RED]` 测试先行

- 文件:`src-tauri/src/services/token_usage.rs`(tests 块内,**不要写在 #![cfg(test)] 之外**)
- 内容:**显式 pin epoch 值,确保 UTC+8 CI 必 RED**
  ```rust
  #[test]
  fn local_date_str_local_midnight_crosses_utc() {
      // Local 2026-07-01 00:30+08:00 → epoch 1782809400000 = UTC 2026-06-30 16:30
      // buggy: from_timestamp_millis(...).format → "2026-06-30"
      // fixed: timeutil::local_date_str → "2026-07-01"
      assert_eq!(timeutil::local_date_str(1782809400000), "2026-07-01");
  }

  #[test]
  fn local_date_to_epoch_round_trip() {
      let epoch = timeutil::local_date_to_epoch("2026-07-01", false).unwrap();
      assert_eq!(timeutil::local_date_str(epoch), "2026-07-01");
      // exclusive=true → 次日 00:00,应等于 inclusive next-day
      let next_via_exclusive = timeutil::local_date_to_epoch("2026-07-01", true).unwrap();
      let next_day_inclusive = timeutil::local_date_to_epoch("2026-07-02", false).unwrap();
      assert_eq!(next_via_exclusive, next_day_inclusive);
  }
  ```
- 验收:`cargo test local_date_str_local` RED,`cargo check` 编译失败(`timeutil` 不存在)

### T1.2 `[GREEN]` 写 helper

- 文件:**新建** `src-tauri/src/timeutil.rs` + `src-tauri/src/lib.rs` 加 `pub mod timeutil;`(插在 `pub mod store;` 和 `pub mod tray;` 之间,目前是 :4 与 :5 之间)
- 内容:`local_date_str` + `local_date_to_epoch`(见 design.md §单点 helper,3 行精简版)
- imports:`chrono::{Local, NaiveDate, TimeZone}`
- 验收:`cargo test local_date_str_local` GREEN,`cargo test local_date_to_epoch_round_trip` GREEN

### T1.3 改 call sites(⚠️ 修 oracle 指出的 line range bug)

**删**:
- `services/token_usage.rs:66-70` 删 `iso_date` 函数定义
- `commands/token_usage.rs:242-256` 删 `iso_date_to_epoch` 函数定义

**替换为 `timeutil::local_date_str`**:
- `services/token_usage.rs:437, 441` (get_summary WHERE)
- `services/token_usage.rs:618, 622` (aggregate_client_models WHERE)
- `services/token_usage.rs:1143-1144` (recompute_costs affected_dates;**保留** `if let Some(...)` 的 silent-skip 语义 → 改 helper 不合理 → 直接用 `crate::timeutil::local_date_str`,无效 epoch 写入 "1970-01-01" 是 no-op 可接受)
- **`database.rs:412-415`** (record_usage INSERT daily_agent_model_usage.date,**P0 移入 Phase 1**,无 schema 改动,仅 helper 替换)
- **`services/token_usage.rs:944-967`**(refresh_daily_rollups start 边界 — 原 `.and_utc()` 隐式把 Local 字符串当 UTC 解析,改用 `timeutil::local_date_to_epoch(date, false)?`;`+ 86400 * 1000` 保留字面量,代码库惯例。**call site 漏登记,validation 阶段发现并修复**)

**替换为 `timeutil::local_date_to_epoch`**:
- `commands/token_usage.rs:693-694` (recompute_costs_by_state)

**⛔ DO NOT TOUCH**:
- `commands/token_usage.rs:231 iso_to_epoch`(RFC3339 解析,IPC contract)
- `commands/token_usage.rs:283, 287`(`ipc_to_rust_filter` 调 `iso_to_epoch`,不是 `iso_date_to_epoch`)
- `commands/token_usage.rs:468`(record_usage IPC 调 `iso_to_epoch`,同上)

验收:
- `grep -rn 'iso_date\b' src-tauri/` 零命中(除非注释)
- `grep -rn 'from_timestamp_millis.*\.format' src-tauri/` 仅命中 `timeutil.rs` 内部
- `grep -rn 'iso_to_epoch\|from_timestamp_millis' src-tauri/` 仅命中 `commands/token_usage.rs:231,283,287,468 + 注释`
- `cargo test` 全绿(已有 191 个 unit test 不能 regress)

### T1.4 `[Regression guard]` 加时区用例

- `services/token_usage.rs::tests` 加 2 个:
  ```rust
  /// 写 epoch = Local 2026-07-01 00:30+08:00 = UTC 2026-06-30 16:30
  /// 期望 month.daily_series 第一项 date == "2026-07-01",且无 "2026-06-30"
  #[test]
  fn get_periods_summary_month_does_not_leak_yesterday() { ... }

  /// 用 gap 内的 epoch 验证 fix:
  /// epoch 1782782400000 = UTC 2026-06-29 20:00 = Local 2026-06-30 04:00+08:00
  /// buggy iso_date_to_epoch("2026-06-30", false) → UTC 2026-06-30 00:00 = 1782796800000
  ///   → 该 epoch 在 [1782796800000, …) 外,**不被刷新**
  /// fixed local_date_to_epoch("2026-06-30", false) → Local 2026-06-30 00:00 = 1782768000000
  ///   → 该 epoch 在 [1782768000000, …) 内,**被刷新**
  #[test]
  fn recompute_costs_respects_local_window() { ... }
  ```
- 验收:T1.1/T1.4 测试在 buggy code 上全 RED,在 fixed code 上全 GREEN,且在 `TZ='Asia/Shanghai'` 的 UTC CI 上也能触发 RED(关键:用绝对 epoch 而不是 Local::now())

### ✅ Phase 1 DoD

- `cargo test` 全绿(含 4 个新 case,既有 191 不 regress)
- `pnpm tsc --noEmit` 不动(本阶段不碰前端)
- `grep -rn 'iso_date\b' src-tauri/` 零命中
- `grep -rn 'from_timestamp_millis.*\.format' src-tauri/` 仅命中 `timeutil.rs` 内部
- `init.sh` 已含 `export TZ='Asia/Shanghai'`
- commit: `"P1: timeutil helper + Rust call sites (incl database.rs:412 P0 移入)"`

### ✅ Phase 1 DoD

- `cargo test` 全绿(含新 T1.1 / T1.4)
- `pnpm tsc --noEmit` 不动(本阶段不碰前端)
- `grep -rn 'iso_date\b' src-tauri/` 零命中
- commit: `"P1: timeutil helper + Rust call sites"`

---

## Phase 2 — schema_v7 migrate

### T2.0 调度层选型(ora-3 H2)

- **不在** `lib.rs::setup()` 加迁移逻辑 — 既有 `setup()` 已经调 `db.migrate()`。
- **在** `database.rs::migrate()` 加 `else if current == "6"` 分支 → 调 `self.migrate_to_v7()` → `UPDATE meta SET value = '7'`。
- 验收:迁移后 `meta` 表 `schema_version == "7"`。

### T2.1 `[RED]` migrate 测试

- 文件:`src-tauri/src/database.rs`(tests 块内)
- 内容:open v6 内存 DB,seed 几条 epoch 跨 06-30→07-01 local 的记录(用 epoch `1782809400000` = Local Shanghai 2026-07-01 00:30+08:00 = UTC 2026-06-30 16:30,这正是 incident 日期),跑 `migrate_to_v7()`,断言 `daily_agent_model_usage.date == "2026-07-01"`(Local 解释)
- 完整 test 矩阵见 design.md §schema_v7 迁移(7 个 F.1..F.7)
- 验收:`cargo test migrate_to_v7` RED

### T2.2 `[GREEN]` implement

- 文件:`src-tauri/src/database.rs` 加 `pub fn migrate_to_v7(&self) -> Result<(), AppError>`
- 内容见 design.md §schema_v7 迁移(re-aggregate via `strftime(..., 'unixepoch', 'localtime')`)
- 关键:DDL 内联,不另开 `.sql` 文件(v6 与 v7 DDL 完全一致)
- idempotency guard 在 helper 顶部(ora-3 M2)
- 验收:`cargo test migrate_to_v7` GREEN(F.2-F.7 全过)

### T2.3 `[RED→GREEN]` 集成 record_usage 走 v7

- Phase 1 已经把 `database.rs:412` (record_usage) 改成 `timeutil::local_date_str`,Phase 2 不需要再动这里。
- 验收:
  - 新建 integration test:写 epoch=1782809400000,断言 DB row date="2026-07-01"
  - `cargo test record_usage` 全绿

### T2.4 `[GREEN]` migrate 调度

- 不动 `lib.rs`。
- 加 `else if current == "6" { self.migrate_to_v7()?; ... UPDATE meta ... }` 到 `database.rs::migrate()`(跟现有 `current == "5"` 分支格式一致)
- 验收:cargo build 通过,在 v6 DB 上跑一次启动后 `schema_version == "7"` 且 `_v6` 表存在 30 天(本 PR 不实现清理命令)

### ✅ Phase 2 DoD

- `cargo test database` 全绿(含 F.1-F.7 新增 + 既有零 regress)
- 本地 DB 实际跑一次启动后 spot check 5 行(2026-06-30 那个旧 UTC 桶行 v6 后归 `2026-07-01` 或 `2026-06-30` 取决于 epoch,符合 Local 解释)
- commit: `"P2: schema_v7 Local bucket + idempotent migrate()"`

---

## Phase 3 — frontend mirror

### T3.1 `[GREEN]` 写 helper

- 文件:`webui/src/lib/format.ts` 加 `formatLocalDate(date: Date): string`(见 design.md)
- 验收:`pnpm tsc --noEmit` 通过

### T3.2 改 3 个 call sites

- `webui/src/components/UsageHeatmapCalendar.tsx:31`:`current.toISOString().split("T")[0]` → `formatLocalDate(current)`
- `webui/src/components/UsageDetailPanel.tsx:56-64`:
  - `getDateRange(range)` 改用 `formatLocalDate(start)` / `formatLocalDate(end)`
- `webui/src/components/ManualQuotaModal.tsx:42`:同样改
- 验收:
  - `grep -rn 'toISOString().split("T")' webui/src/` 仅命中 format.ts 注释
  - `pnpm tsc --noEmit` 0 错误
  - `pnpm playwright test` 全绿(已有 test 不 regress)

### T3.3 `[Regression guard]` 详情面板用例

- `webui/tests/usage-page.spec.ts` 加 test:`Trend chart rolls 30d from all_time on day 1 of a month` 已有,再加:详情面板 `end_date` 用本地日期断言(模拟 local 00:30,断言返回 today)
- 验收:对未实施 T3.2 的代码必 RED

### ✅ Phase 3 DoD

- `pnpm tsc --noEmit && pnpm playwright test` 全绿
- commit: `"P3: frontend formatLocalDate helper + 3 call sites"`

---

## Phase 4 — sibling fix

### T4.1 AvgDayCard(✅ Q4 = 本月至今 → 路径 B)

- 数据源保持 `<AvgDayCard dailySeries={month?.daily_series} />`(不改)
- 标签改 `AVG / DAY (30D)` → `AVG / DAY (MTD)`(Month-to-Date)
- 关键修复:`currentAvg = sum / sorted.length`(不再除以 30)
- 移除 `last30.length < 30` / `sorted.length < 60` / `prior30` 比较分支(本月至今日均,没有"前 30 天"概念)
- `subLabel` 改 `${days} day(s) so far`(动态显示 MTD 跨度)
- 文件:`webui/src/components/UsageKpiCards.tsx`
- 代码见 `design.md` §Path B 实施细节
- 验收:
  - TS unit(Playwright):mock `daily_series = [{date:"2026-07-01", total_tokens:1e7}]` → 渲染 `1 day so far`,avg = 1e7(不是 1e7/30)
  - TS unit:mock 30 天数据 → avg = sum/30,`30 days so far`
  - 端到端:`pnpm playwright test` 全绿

### T4.2 `opencode_go_limits.rs` monthly_used

- `services/opencode_go_limits.rs:358`:`chrono::Utc::now().timestamp_millis()` → `chrono::Local::now().timestamp_millis()`
- 测试:加 mock 验证 `rows[]` 的 ts(原 UTC-epoch)与新 `now_ms`(Local-epoch)对齐后,`monthly_used` 算正确
- 验收:`cargo test opencode_go_limits` 全绿

### T4.3 `deepseek_balance_history.rs`

- 核验:`sorted[i].0` 的 ts 来源(grep 调用栈)
- 若非 Local-epoch,改 `Local::now()`;若已是,加 helper 调用一致性 + 注释
- 验收:同 T4.2

### ✅ Phase 4 DoD

- `cargo test opencode_go_limits deepseek_balance_history` 全绿
- AvgDayCard 修法路径 A 或 B 已定,前端回归绿
- commit: `"P4: sibling fixes (AvgDayCard / opencode_go_limits / deepseek)"`

---

## Phase 5 — TZ-intent 注释

### T5.1 P2 命中点注释

- `provider/openai.rs:93` — `// intentional Utc: OpenAI billing API expects UTC dates`
- `auto_import.rs:66/119` — `// intentional Utc: cursor wall-clock seconds for monotonic reorder`
- `incremental_import.rs:400/446` — 同上
- `commands/token_usage.rs:242`(若未删)— 重命名为 `local_date_to_epoch`,加 fn doc 注明 Local 解释

### T5.2 `[GREEN]` grep gate

- 在 `init.sh` §4 的 grep gate 加一条(可选):
  ```bash
  # forbidden: epoch → UTC date string pattern
  ! grep -rn 'from_timestamp_millis(.*)\.format("%Y-%m-%d' src-tauri/src
  ```
- 验收:`./init.sh` 全绿

### ✅ Phase 5 DoD

- 4 处注释就位
- `init.sh` 含新 gate
- commit: `"P5: TZ-intent 注释 + init.sh grep gate"`

---

## Phase 6 — TDD 回归(放大 spec TC-01..TC-07)

### T6.1 Rust TC-01..TC-03 + TC-06(共 4 test)

- T1.4 已建 TC-03,这里扩:
  - TC-01 round-trip
  - TC-02 epoch = Local 2026-07-01 00:30+08:00 → "2026-07-01"
  - TC-06 OpenAI 调用 snapshot
- 验收:`cargo test` 含 4 个新 case,均 GREEN

### T6.2 TS TC-04 TC-05

- TC-04:Playwright 模拟本地 00:30,断言 `getDateRange("30d").end_date == today`
- TC-05:AvgDayCard rolling 30d 用例(全 60 天 mock,断言 avg = sum/30)
- 验收:`pnpm playwright test` 5+/5+ 通过

### T6.3 TS TC-07(integration)

- `migrate_to_v7()` 后,前端通过 IPC 拿 `month.daily_series`,断言日期落在 Local 区(用 mock IPC 验证 schema_v7 contract)
- 验收:Playwright 跑通

### ✅ Phase 6 DoD

- `cargo test` 全绿(+4 case)
- `pnpm playwright test` 全绿(+3 case)
- commit: `"P6: regression test suite (TC-01..TC-07)"`

---

## Phase 7 — blast + 全绿

### T7.1 blast search

- `rg -n 'from_timestamp_millis|format\(.*%Y|toISOString|split\("T"\)\[0\]|slice\(-[0-9]+\)' src-tauri/ webui/src/`
- 期望:仅 helper 内部、P5 注释、或 `let _ = unused_format` 形式
- 人工 review 每条命中

### T7.2 `./init.sh` 全绿

- `cargo check && cargo test --manifest-path src-tauri/Cargo.toml && cd webui && pnpm tsc --noEmit && ./init.sh`
- 验收:所有现有的 8 项 gate(visibility/value/preset/category_id/agent_file_cursor/无加密 crate/notify-debouncer-full/fs::write 白名单)+ 新增 P5 P2 gate 全过

### T7.3 文档收尾

- 更新 `feature_list.json`(新 entry `fix-date-local-timezone`,status=done)
- 更新 `progress.md`(记录 Phase 完成状态)
- 更新 `session-handoff.md`(若需要)
- 同步 `AGENTS.md` → `CLAUDE.md`(若未同步)
- git commit + descriptive message

### ✅ Phase 7 DoD

- 全部 blast 命中已确认
- `./init.sh` 全绿
- 文档收尾
- commit: `"P7: docs + blast + init.sh green"`

---

## 决策清单

| # | 决策 | 状态 | 备注 |
|---|------|------|------|
| Q1 | 系统时区 | ✅ **Local**(OS 决定) | design.md §决策点 |
| Q2 | schema 迁移策略 | ✅ 重 aggregate,不 truncate | design.md §schema_v7 迁移 |
| Q3 | 单一 helper | ✅ 是(timeutil.rs) | design.md §单点 helper |
| Q4 | AvgDayCard 语义 | ✅ **本月至今(MTD, 路径 B)** | design.md §Path B 实施 |
| Q5 | 旧 v6 表保留期 | ✅ **30 天** | design.md §schema_v7 迁移 |

**Phase 1 立即可开工**(Q1-Q3 已定),**Phase 4 T4.1 已 unblock**(Q4 路径 B)。
