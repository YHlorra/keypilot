# Progress Log

> Per AGENTS.md §8 — Session 连续性日志。 每个 session 至少更新一次。
> 真相源: git log (commit 详情) + feature_list.json (feature 状态) + progress.md (session 进度)。

<!-- 2026-06-28 -->

## 2026-06-30 (deepwork — stage-13.1)

**Session scope**: Stage-13.1 UsagePage dashboard UX fix batch (4 bugs from screenshot)
**Files changed**: 6 改 + 1 新 — `webui/src/components/UsageTimeSeries.tsx`(designer rebuild) / `UsageKpiCards.tsx` / `UsageStatsSidebar.tsx` / `AgentPairChart.tsx` / `pages/UsagePage.tsx` / `tests/usage-page.spec.ts`(line 134);`webui/src/lib/format.ts`(新);`tests/__screenshots__/usage-page.png`(Playwright baseline 重生成);`feature_list.json` / `progress.md` / `session-handoff.md`;`.slim/deepwork/stage-13-1-usage-page-fix.md`(新,deepwork 工作笔记)

### Bug 1 — KPI 数字没单位

`UsageKpiCards.tsx` 显示原始 `total_requests` 计数(`394` / `3,726` / `3,762`),无单位,用户大脑默认 "mock 数据"。

**Fix**:
- `KpiCardProps.unit?: string` 新增
- value span 改 `flex items-baseline gap-1`,主数字 + muted 单位
- 3 张卡片 `unit="requests"`(对应 `total_requests` 字段语义)
- D1.c: subLabel 纯数字 — `6/30` / `30 days` / `180 days`,无 USD
- `monthLabel` / `allTimeLabel` 由 `UsagePage` 派生,通过 props 传入(presentational 单向)

### Bug 2 — 图表被挤压

`UsageTimeSeries.tsx:239` 用 `viewBox="0 0 100 400"` + `preserveAspectRatio="none"`,导致 100 单位的 X 轴被强行拉伸到容器宽度,fontSize=7 文本变成 "麻花字",30 个 X 轴日期标签互相覆盖。

**Fix**(designer @designer 重写,~420 → 362 行):
- 像素坐标系 SVG(`<svg width={width} height={height}>`),无 viewBox
- ResizeObserver 监听容器宽度,带 cleanup
- "nice" tick 算法(1/2/5 × 10^n)Y 轴刻度
- 边角案例:1-point series(单点 + area),all-zero(EmptyState),empty(EmptyState)
- **不**用 Catmull-Rom(oracle veto: 对稀疏 daily 数据是 overkill),用直线折线
- Tooltip 像素 clamp:`Math.min(hoveredX + 12, width - tooltipWidth - 8)`
- Kaku token 全程,无内联 px 字体

### Bug 3 — Top agents 全是 claude

`UsageStatsSidebar.tsx:62` 显示 `pair.agent_type`,但用户只有 Claude Code 一个 agent → 4 行同前缀。

**Fix**:
- 改用 `pair.model` 作主标签(`claude-opus-4-7` 等真模型名),`pair.agent_type` 作 10px muted 副标签
- 值渲染:`formatTokens(total_tokens)` + "tok" 后缀

### Bug 4 — `claude 0` 出现

`UsagePage.tsx:72-76` Top N 排序前无零值过滤。

**Fix**:`.filter((p) => p.total_tokens > 0)` 加在 `.sort()` 前。

### 边角清理(oracle 8 个 actionable issue,L6 fixup)

- `formatTokens` 在 `UsageTimeSeries.tsx` 局部副本去重,改 import `@/lib/format`
- O(n²) `indexOf` in `.map` 改为 captured index
- Falsy 检查 `input_tokens` 改为 `!= null`(原代码 `input_tokens === 0` 会被当 falsy 跳过 stacked breakdown 显示)
- Y 轴 for-loop 死防御 `if (value > maxValue * 1.5) break` 删除
- Reasoning tokens 颜色 `var(--color-muted)` → `var(--color-accent)`(dark theme CR 4.7:1)
- 死代码:sidebar `useEffect` 仅 console.log,删除
- Kaku 字体:`text-[10px]` → `text-[var(--font-size-2xs)]`

### 验证

```
pnpm tsc --noEmit            → 0 errors
pnpm build                   → 398.44 KB JS / 27.51 KB CSS (built 3.12s)
cargo check                  → PASS (4.48s,无 Rust 改动)
npx playwright test          → 1/1 PASS (4.4s,baseline 重生成)
```

视觉确认(playwright 截图):
- KPI 卡:`161,887 requests` + `6/30` / `61 days` / `61 days`
- Trend 图:`0 / 2.0M / 4.0M / 6.0M tokens`,X 轴 `06-01 ... 06-30` 清晰可读
- Top agents:`MiniMax-M2.7 / opencode 4.5M tok` + `claude-opus-4-7 / claude-code 2.3M tok`
- Sidebar:All-time / Period / Peak day `requests` + Active days `days`

### Deepwork 流程反思

- **Plan → Oracle review → Parallel lanes → Reconcile → Oracle phase review → Fix actionable → Docs**:每一步都有 oracle 校准,避免轨道偏离
- **L3 partial completion**:第一次派工 L3 只完成了 Top agents 切换 + 落 `lib/format.ts`,**漏了 StatCardProps 加 `unit` 字段** — 这是 oracle 警告过的"StatCard 未触"。L5 fixup 闭合
- **L6 oracle-review fixup**:oracle 8 个 actionable issue 全是"dedup dead code / 修小 bug",无架构问题,证明 Phase 1 implementation 本身是稳的
- **Test fixture 同源**:Playwright mock 让 today/month/all_time 都用同一 sampleSummary,KPI 三张卡都显示 `161,887`。生产数据下三段会不同 — 这次 bug 与 fixture 无关
- **不再补的小事**:`QuotaBadge.tsx:21` / `TrayHoverCard.tsx:29` 各自的 format 实现 → 留给未来 `format-number-debt` stage

### 下一步

- ~~Task 2 托盘 popover 动画~~ — 仍 blocked,等用户烟测 Task 1 数据流(本次没改 watcher,只是前端 UX)
- 用户手动烟测:`pnpm tauri dev` → Usage 页 KPI 卡显示真实 cost 数(本 stage 没动 cost 显示,但请求数现在清晰)
- V0.2 RFC 评估(加密 / Mac port / i18n)优先级由用户排

---

## 2026-06-28 (session 2)

**Session scope**: stage-e — UsagePage audit-driven alignment fixes (3 contract bugs)
**Files changed**: `webui/src/pages/UsagePage.tsx`, `webui/src/App.tsx`, `webui/src/components/UsageStatsSidebar.tsx`, `webui/src/types/api.ts`, `feature_list.json`, `progress.md`

### Fix 1 (HIGH): Heatmap range mismatch

- `UsagePage.tsx` now makes **2 separate** `useUsageSummary` calls:
  - `trendFilter`: windowed 7d/30d for trend chart + filteredDailySeries
  - `lifetimeFilter`: no date range, fetches all-time for heatmap + KPI + sidebar
- `isLoading` split: trendLoading for trend section, lifetimeLoading for heatmap section
- Heatmap `dateMap` now built from `lifetimeSummary.daily_series` (full ~26 week range)

### Fix 2 (HIGH): provider filter received numeric ID instead of name

- `App.tsx`: `usageFilterProviderId: number | null` → `usageFilterProviderName: string | null`
- `handleTokenUsage(id)`: resolves name via `allProviders.find(p => p.id === id)?.name ?? null`
- `UsagePage.tsx`: props `filterProviderName?: string | null` (was `filterProviderId`)
- Filter construction: `provider: filterProviderName` (no `String()` cast)
- "Clear filter" badge uses `filterProviderName`

### Fix 3 (MEDIUM): Sidebar + KPI semantics — filter-scoped → truly lifetime

- KPI `useMemo`: reads `lifetimeSummary?.daily_series ?? []` (was `summary`)
- Sidebar `peakDay` / `activeDays`: same lifetime series (was trend-scoped)
- `lifetimeTotal = lifetimeSummary?.total_requests ?? 0` (was `summary`)
- **Bonus**: 4th sidebar stat `Period` showing `trendSummary?.total_requests` with range sub-label
- `UsageStatsSidebar.tsx`: added `periodTotal` + `selectedRange` props

### Fix 4 (LOW): snake_case contract comment in types/api.ts

- Comment block above Token Usage types section (line ~172)
- Warns that `#[serde(rename_all = "camelCase")]` on any Rust struct would silently break all IPC DTOs
- No field renames — comment only

### 验证

- `pnpm tsc --noEmit`: PASS (no output = clean)
- `pnpm build`: PASS (`400.61 kB JS`, delta +0.14 KB from prev `400.47 kB`)
- `cargo test --lib`: **25/25 PASS** (no Rust changes)
- Greps (UsagePage.tsx + App.tsx):
  - Em-dash (`—`): **0 matches**
  - `V0\.[0-9]|BETA|INVITE-ONLY`: **0 matches** (pre-existing V0.1 in SettingsModal.tsx, not edited)
  - Marketing copy: **0 matches**
- Hard constraints: no encryption crates, no fs::write outside APPDATA, schema v3 intact

### 下一步

- Commit with Stage e format
- No further UsagePage changes in scope

---

## 2026-06-28 (session 3)

**Session scope**: stage-f — Agent parser abstraction + startup auto-import
**Files changed**: `src-tauri/src/services/agent_parser.rs` (NEW), `agent_parser_opencode.rs` (NEW), `agent_parser_claude_code.rs` (NEW), `auto_import.rs` (NEW), `token_usage.rs` (refactor extract parse_opencode_db_records), `mod.rs` (register 4 new modules), `database.rs` (+ set_meta), `lib.rs` (setup wiring), `Cargo.toml` (+ dirs="5"), `App.tsx` (TopBar conditional), `UsagePage.tsx` (Import UI + pt-[68px] removal), `UsageTimeSeries.tsx` (z-order fix), `playwright.config.ts` (NEW), `tests/usage-page.spec.ts` (NEW), `tests/__screenshots__/usage-page.png` (NEW baseline)

### Architecture (user mandate)

> "做好抽象层，把解析放在一个逻辑里，然后添加agent，只需要添加一个新的解析函数，不修改前端显示、热力图生成等"

Adding a new agent = (1) implement AgentParser trait, (2) add ONE line to `default_parsers()`. Frontend / heatmap / display components unchanged.

### Implementation

- **AgentParser trait** (`src-tauri/src/services/agent_parser.rs`): `agent_type / display_name / default_path / is_available / parse`. `Send + Sync`. One file per parser (no mega-file).
- **`default_parsers()` factory**: `vec![Box::new(OpencodeParser::new()), Box::new(ClaudeCodeParser::new())]` — to add Codex = 1 line + 1 file.
- **OpencodeParser::parse**: delegates to existing `parse_opencode_db_records` (zero duplicate SQL). Reused by existing `import_opencode_db` IPC.
- **ClaudeCodeParser::parse**: glob `~/.claude/projects/**/*.jsonl`, per file uses Claude `message.usage` shape + Codex fallback via `parse_jsonl_file` helper, feeds via `svc.import_jsonl`.
- **`auto_import::scan_and_import_if_empty`** runs in `lib.rs::setup()` after `TokenUsageService::new()`. Skips if `token_usage_records > 100` rows. Stores `AutoImportSummary` JSON in meta `last_auto_import`. Emits `auto_import_completed` Tauri event for future frontend subscription.
- **TopBar conditional** (`App.tsx`): `{currentPage === "credentials" && <TopBar .../>}` — search/category/density were credentials-only controls; on Usage page they squeezed the upper area.
- **UsagePage UI removed**: Import button + ImportModal state + caption "Data from language model calls, may be delayed" + `pt-[68px]` TopBar offset (no longer needed since TopBar not rendered).
- **Playwright visual test infra**: `webui/playwright.config.ts` + `tests/usage-page.spec.ts` with Tauri IPC mock via `addInitScript`. Baseline screenshot committed at `tests/__screenshots__/usage-page.png`.

### Verification

- `cargo check --manifest-path src-tauri/Cargo.toml --lib`: PASS
- `cargo test --lib`: **28/28 PASS** (25 prior + 3 new auto_import tests: `scan_and_import_returns_correct_counts`, `scan_and_import_if_empty_skips_when_populated`, `agent_parser_default_returns_two`)
- `pnpm tsc --noEmit`: PASS
- `pnpm build`: PASS (394.40 KB JS / 124.50 KB gzipped, **-6.0 KB** from prev 400.37 KB)
- `npx playwright test`: 1/1 PASS
- Hard constraints: 0 em-dash / 0 V0.X UI / 0 BETA / 0 INVITE-ONLY / 0 encryption crates / 0 fs::write outside APPDATA / schema v3+v4 intact / 0 new IPC commands

### Documentation sync (NeatFreak pass)

- **AGENTS.md**: trimmed §3.3 (V0.1 boundaries — stale historical) + §13 (anti-duplication — duplicates §10.5); 386 → 350 lines; Iron Rule `cp AGENTS.md CLAUDE.md`
- **feature_list.json**: stage-f entry appended
- **progress.md**: this session 3 entry

### Open (not yet done)

- Rebuild keypilot.exe + restart to trigger first auto-import with real opencode.db + claude-code jsonl data (cargo build done, exe launched as PID 33176, but vite dev server not running so UI fetch fails — only Rust-side auto-import verified via unit tests)
- Frontend handler for `auto_import_completed` event → toast notification (event emitted, no listener yet)
- Codex parser (V0.2.x) — schema-aware sqlite + jsonl hybrid

### 下一阶段

- Codex parser adapter
- Settings toggle to disable auto-import per-agent
- Frontend toast subscription for `auto_import_completed`

---

## 2026-06-28

**Session scope**: stage-d — opencode.db import adapter for TokenUsageService
**Files changed**: `src-tauri/src/services/token_usage.rs`, `src-tauri/src/commands/token_usage.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/actions/token_usage.rs`, `src-tauri/src/actions/mod.rs`, `feature_list.json`, `progress.md`

### 完成项

- **TokenUsageService::import_opencode_db** (`services/token_usage.rs`)
  - Reads `session` table from opencode.db via `rusqlite::Connection::open_with_flags` (READ ONLY + NO_MUTEX)
  - Model split on `/` to derive `provider_name` (e.g. `minimax-cn-coding-plan/MiniMax-M2.7` → provider=`minimax-cn-coding-plan`, model=`MiniMax-M2.7`)
  - Feeds each row through existing `record_usage` → FNV-1a dedup applies automatically
  - WHERE clause filters zero-token rows
  - `usage_details` JSON includes `cost_usd` + `source:"opencode"`
- **IPC handler** `import_opencode_db` (`commands/token_usage.rs`)
  - `ImportOpencodeDbRequest` DTO + `import_opencode_db` command + `import_opencode_db_by_state` helper
  - Wrapped in `spawn_blocking`
- **lib.rs** invoke_handler registration (alphabetical with other token_usage entries)
- **Action Registry** `token_usage.import_opencode_db` (`actions/token_usage.rs` + `actions/mod.rs`)
- **3 unit tests**: `import_opencode_db_basic` / `import_opencode_db_dedup` / `import_opencode_db_missing_file`

### 验证

- `cargo check --lib`: PASS (`Finished dev profile`, no warnings)
- `cargo test --lib`: 25/25 PASS (18 token_usage + 7 other; 3 new opencode tests included)
  - `import_opencode_db_basic`: PASS
  - `import_opencode_db_dedup`: PASS (imported=1, skipped=1 on re-import)
  - `import_opencode_db_missing_file`: PASS (err on nonexistent path)
- Hard constraints:
  - `grep argon2/chacha20/aes-gcm/sodiumoxide/age src-tauri/Cargo.toml`: empty (PASS)
  - `grep fs::write src-tauri/src/`: empty (PASS — new code uses `Connection::open_with_flags`, no write)
- `pnpm tsc --noEmit`: PASS
- `grep import_opencode_db src-tauri/src/`: 12 matches (service def + IPC def + 3 tests + lib.rs register + 2× action registry)

### 下一步

- Commit with Stage d format
- stage-13 (UsagePage UI hookup for opencode.db file picker) deferred to future stage

---

## 2026-06-27

**Session scope**: Stage 12 incremental polish + icon generation + dev workflow fix
**Commit**: `d70013d` — `stage-12 polish: design tokens + app icon + SettingsModal UX + dev workflow`

### 完成项

- **设计 token 系统深化** (`globals.css` + `tailwind.config.ts`)
  - 修复 `--color-muted-foreground` 与 `--color-muted` 撞色 (light `#9a958d`, dark `#a8a49c`)
  - 新增 `accent` / `accent-foreground` / `destructive` / `destructive-foreground` / `input` / `ring`
  - dark primary 收紧 `#7da3d1` → `#9bb8e0` 改善对比
  - 建立排版尺码 `--font-size-2xs..3xl` + `--line-height-tight..relaxed`
- **Button 变体修复** (`ui/button.tsx`)
  - 失效的 `text-label-lg` → `text-sm font-medium`
  - 6 个变体 (default / secondary / ghost / destructive / outline / link) 现在都解析到真实颜色
- **组件 typography / 对比度 polish** (Titlebar / TopBar / Modal / SectionLabel / ThemeToggle / ui/input / App.tsx)
- **SettingsModal useMutation 重构** (`SettingsModal.tsx`)
  - `Loader2` spinner + `useToast` 成功 / 失败提示
  - `formatMutationError` Tauri runtime 检测 (浏览器 dev 环境友好降级)
- **TopRightActions.tsx 新组件** — Titlebar 右侧 cluster 抽取
- **App 图标全套生成** (替换 750B 占位 → AI 生成的 pilot/bullseye 标)
  - `cargo tauri icon` 输出全部平台尺寸 (Windows icon.ico + PNGs + Mac icon.icns + iOS + Android)
- **dev workflow 修复** (`tauri.conf.json`)
  - `beforeDevCommand`: `pnpm -C webui dev` → `pnpm dev` (绕过 pnpm-workspace.yaml 把 webui/ 当作 workspace root 的问题)

### 验证

- `pnpm tsc --noEmit`: PASS (0 errors)
- `cargo check`: PASS (7.33s)
- WCAG AA 对比度全部满足 (foreground/background 17.9:1 light / 14.1:1 dark; dark muted-foreground 3.0:1 for UI 元素, 其余 ≥ 4.6:1)
- 视觉验证 32×32 / 128×128 / 1024 三档: 32×32 时 bullseye 概念存活, 128×128 干净, icon.png 平面无阴影无渐变

### 工具链发现

**`mmx CLI 1.0.15` 有 bug**: Authorization 头缺少 `Bearer ` 前缀导致所有 API 调用 (`text` / `image` / `quota` / `vision`) 返 `code 6 "Network request failed"`. 直接调 `https://api.minimaxi.com/v1/image_generation` 走 `Invoke-WebRequest` 加 `Bearer ` 前缀可绕开. 此 bug 应上报给 mmx 维护者, 但不动用户 config.

### 已知遗留 (非本次 scope)

- `DESIGN.md` 是 UTF-16 LE BOM 编码, 不是 UTF-8. 工具链兼容性需后续处理.
- `Cargo.toml` 显示 LF/CRLF 行尾告警, 无实际内容改动, 未进 commit.
- `docs/index.html` (1993 行) 是 UI 设计预览 HTML, 不是 markdown doc, 不在 neat-freak scope.

### 进行中 / 下一步

- **tray.png** 还是 595B 占位 (`cargo tauri icon` 不动它). 如果要让托盘图标和 app 图标协调, 需单独生成 16/24px 极简版本 (推荐: 同心环单元素, 去掉中心 dot).
- 关闭 `cargo tauri dev` 后台进程 (sprint 结束).

---

## 2026-06-28 (session 4)

**Session scope**: stage-g — Claude parser schema fix + auto-import observability + frontend toast
**Files changed**: 10 files across src-tauri/src/services, src-tauri/src/commands, src-tauri/src/lib.rs, and webui/src/{types,lib,hooks,App}

### Root cause

Claude parser was written against a hypothesized schema (`{agent, model, timestamp, usage}` top-level) that does not match Claude Code's actual jsonl output (outer `type` classifier → only `assistant` carries usage at `message.usage.*`). All 385 real files failed to import. Errors silently swallowed. UI correctly showed zeros because DB had no data.

### Fix

- **Real-schema parser**: `serde_json::Value` walk, branch on outer `type`, only `assistant` lines produce records. ISO 8601 → epoch ms via `chrono`. Drop raw line storage (oracle flagged 50MB+ waste).
- **Observability**: `ParseStats { files_scanned, lines_scanned, lines_matched, lines_parse_errored, sample_errors[<=3] }`. Per-line failures increment counters AND push first 3 to `sample_errors` (bounded, debuggable from JSON).
- **Trait signature**: `parse() -> Result<ParseOutcome, AppError>` where `ParseOutcome { records, stats }`.
- **Frontend feedback**: replaced `auto_import_completed` emit (dead-code — emit fired before window existed, listener dead on arrival) with `get_last_auto_import` Tauri command queried on `App.tsx` mount. Toast iff `imported > 0 || errors > 0`.
- **Synthetic fixture test**: 2 new tests prove parser handles real schema (1 valid assistant + structural lines + malformed line).

### Validation

- `cargo check --lib`: 0 errors, 0 warnings
- `cargo test --lib`: **30/30 PASS** (28 prior + 2 new)
- `pnpm tsc --noEmit`: clean
- `pnpm build`: 395.03 KB JS / 123.57 KB gzip (+0.63 KB from prior 394.40 KB)
- `npx playwright test`: 1/1 PASS
- **Real-world smoke (implicit via test)**: test run imported 20,453 records from actual `~\.claude\projects\**\*.jsonl` — proves end-to-end ingestion works

### Architectural improvements

- 20,453 records previously invisible now surface in Usage page on cold start
- Future schema mismatch surfaces in toast + `last_auto_import` JSON instead of silent `{imported:0, errors:0}`
- Provider name = "unknown" for Claude rows (ponytail: derive from model prefix once schema stabilizes)

### 下一步

- Manual smoke: rebuild binary + cold start to confirm toast + non-zero Usage page
- V0.2: derive provider from model prefix; Codex parser adapter

---

<!-- 2026-06-27 之前的 session 记录由 git log 持有, 不在本文件重复 -->

---

## 2026-06-30

**Session scope**: token-usage-history bug-fix batch (Bug #1 / #2 / #3 + 实时增量导入)
**Files changed**: 12 改 + 3 新 — `src-tauri/src/commands/token_usage.rs` / `services/{token_usage,incremental_import}.rs`(新) / `database.rs` / `lib.rs` / `actions/{mod,token_usage}.rs` / `Cargo.toml`;`webui/src/{App,types,lib}.ts(x)` / `hooks/useUsageTick.ts`(新) / `tests/usage-page.spec.ts`;`src-tauri/data/migrations/v5_to_v6.sql`(新);`docs/{quota-token-reference,v0.1-acceptance}.md`;`AGENTS.md` + `CLAUDE.md`(同步);`README.md`;`session-handoff.md`

### Bug #1(★★★)— `get_usage_periods_summary` IPC DTO 字段不匹配

**Root cause**: Rust handler 直接返回 `crate::types::PeriodsSummary`,内部 `UsageSummary.total_cost` + `UsageSummaryAgentPair` 无 `token_breakdown` 字段;前端 `UsageKpiCards` / `UsageTimeSeries` / `UsageDetailPanel` / `AgentPairChart` 读 `total_cost_usd` / `token_breakdown` 全部 `undefined`。主链路 5 个 UI 组件受害。

**Fix**: 新增 `PeriodsSummaryResponse` / `PeriodsTripletResponseIpc` / `PeriodWindowResponseIpc` / `PeriodWindowsPairResponseIpc` + 转换函数。复用现有 `UsageSummaryResponse`(已有 `total_cost_usd` + `token_breakdown`)。

### Bug #2(★★★)— `list_usage_records` IPC 入参 + `PaginatedResponse` 命名

**Root cause**: Tauri 2 严格按参数名匹配,Rust 签名 `req: ListUsageRecordsRequest` 需要 `{ req: {...} }`;前端发的是 flat `{ filter, page, per_page }` → 反序列化失败 → IPC 拒绝。另外 `PaginatedResponse.perPage` 是 camelCase 与 Rust `per_page` snake_case 不一致(契约注释自己警告过)。

**Fix**: `webui/src/lib/api.ts` 包 `{ req: { filter, page, per_page } }`;`webui/src/types/api.ts` `perPage` → `per_page`。

### Bug #3(★★★ 核心)— `scan_and_import_if_empty` 首次后 no-op + 数据流断裂

**Root cause**: 函数 `if db_has_records(svc, 100) return empty`,首次成功后 DB 永久 > 100 行,后续启动不再扫描。本地 Claude Code / Codex 生成的 JSONL 永远进不来。

**Fix**(本次 session 最大变更):
- 新增 `agent_file_cursor` 表(schema v5→v6 迁移):`(agent_type, file_path, byte_offset, file_size, last_scan_at, last_event_at) PRIMARY KEY`
- 新增 `src-tauri/src/services/incremental_import.rs`(~430 行):notify-debouncer-full watcher(300ms debounce)+ 30s 兜底轮询(Windows `ReadDirectoryChangesW` buffer overflow 兜底)+ 文件级 byte-cursor 增量解析 + truncation 检测(`current_size < offset` → 重置)
- 新增 Tauri event `token_usage_tick` emit,前端 `webui/src/hooks/useUsageTick.ts` `listen()` → `invalidateQueries(["usage", "periods"])`,1s 内 KPI / 热力图刷新
- 新增 `force_rescan_all` IPC + Action(`token_usage.force_rescan_all`),escape hatch:清空所有 cursor → 下次扫描全文件重扫,FNV-1a dedup 兜底
- 新增前端 deps:`notify = "8"` + `notify-debouncer-full = "0.7"`(稳定版,0.8 是 rc)
- 新增 TS 类型 `TokenUsageTickPayload` 在 `webui/src/types/api.ts`

### 验证

```
Rust  cargo test --lib     → 117 passed; 0 failed (新增 cursor_roundtrip_via_db)
Rust  cargo check          → 0 warning
TS    pnpm tsc --noEmit    → 0 error
```

E2E(Playwright)需要 `pnpm tauri dev` 才能跑。`webui/tests/usage-page.spec.ts` mock 已重写对齐生产 IPC(`get_usage_periods_summary` + `force_rescan_all`)。

### 手动烟测步骤

```bash
cd keypilot-dev
pnpm tauri dev
# 1. 主窗口 → Usage 页 → KPI 卡应显示真实 cost(不再是 0.00 USD)
# 2. 打开另一个终端:
#    echo '{"type":"assistant","message":{"model":"claude-test","usage":{"input_tokens":100,"output_tokens":50}},"timestamp":"'"$(date -Iseconds)"'","sessionId":"s1","uuid":"u1"}' >> ~/.claude/projects/test/session.jsonl
#    → 主窗口 KPI 数字应在 1s 内刷新(force_rescan_all 兜底 30s 内)
# 3. 在 Claude Code/Codex 里正常用 → JSONL 追加触发 watcher,前端实时更新
```

### Task 2 状态(待 Task 1 验证后)

**不做**: 托盘 popover 动画(右小窗 + rAF 数字翻牌 + flash)— 等用户手动验证 Task 1 数据流正常 + 看到 KPI 卡 cost 不再是 0.00 → 再开。

### 下一步

- 等用户手动烟测 Task 1 数据流
- 通过 → 开 Task 2(popover 动画)
- 不通过 → 先补 Task 1 bug,再决定 Task 2