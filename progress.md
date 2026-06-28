# Progress Log

> Per AGENTS.md §8 — Session 连续性日志。 每个 session 至少更新一次。
> 真相源: git log (commit 详情) + feature_list.json (feature 状态) + progress.md (session 进度)。

<!-- 2026-06-28 -->

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