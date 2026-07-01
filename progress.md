# Progress Log

> Per AGENTS.md §8 — Session 连续性日志。 每个 session 至少更新一次。
> 真相源: git log (commit 详情) + feature_list.json (feature 状态) + progress.md (session 进度)。

## 2026-07-02 — Over-engineering execution (Phase 0-7.5 complete)

**Goal**: Execute audit findings from prior session (5,000-5,500 LOC removal target).

**Result**:
- Rust dead code: 8 files deleted (provider/agent_source.rs + agent_sources/{claude_oauth,codex_rpc,cursor_probe,mod}.rs + postgres.rs + services/{currency,opencode_go_limits}.rs = 2,447 lines)
- Rust structural: 5 DB methods deleted, 2 types deleted, QuotaSnapshot derives Default, helpers added (timeutil::now_secs/now_millis, From<tauri::Error>, validate_key default), IncrementalImporter shutdown+Drop removed, test_connection action removed
- Rust deps: tokio-postgres removed, base64 still as transitive (reqwest chain)
- WebUI dead code: 9 files deleted (ManualQuotaModal/ImportModal/TrayHoverCard/UsageDetailPanel/QuotaBadge/AgentPairChart + ui/card + ui/toast + useActions)
- WebUI structural: dead IPC wrappers deleted (testConnection/pinProvider/etc), dead hooks deleted (useQuota/usePricing/etc), dead types deleted (Limit*/MoneyAmount etc)
- WebUI deps: date-fns dropped, two-mode Intl.RelativeTimeFormat helper added

**Net**: ~4,781 lines removed across 37 Rust files + 305 lines removed across 11 WebUI files = ~5,086 LOC

**Verification**:
- cargo check: PASS
- cargo test: 121/121 PASS (89 lib + 32 e2e)
- pnpm tsc --noEmit: PASS
- pnpm build: PASS
- Hard constraints: 4 encryption crates grep PASS, fs::write whitelist PASS, .and_utc() PASS, toISOString().split PASS

**Deferred**:
- Phase 5 IPC DTO collapse — needs serde round-trip spike first (structural drift: Langfuse fields, ISO8601 vs epoch, total_cost_usd vs total_cost)
- provider.test_and_refresh action — borderline, kept
- cva for button.tsx only — borderline, kept
- executeAction generic dispatch — borderline, kept
- _by_state duplication — oracle-verified misguided (Tauri State<'_, T> vs &AppState distinct)
- sha2 dep — kept for hash stability (deepseek account_key persisted, SipHash would orphan data)
- once_cell dep — kept (one-time init pattern)
- uuid dep — kept (1 req-id site)

**Commits**:
- 1231cd6 Phase 3b (Rust: 8 dead files + helpers + structural, 37 files, 4781 deletions)
- e355b5d Phase 4+6 (WebUI: dead wrappers + date-fns drop, 11 files, 305 deletions)

---

## 2026-07-01 (stage-13.3 — UsagePage UI 收紧 + 设计稿定稿)

**Session scope**: 用户截图报告 UsagePage 5 个 UI bug → 创建冻结设计稿 `docs/usage-page.html` → 按稿实施到 4 个 React 组件 + 页面布局。**用户拍板定稿**(4 组件 = 热力图 / 趋势 / token 使用 / 请求数,与设计稿一致)。

**Files changed**: 7 改 + 1 新
- `docs/usage-page.html`(新) — 冻结设计稿,自包含 HTML,所有 token/尺寸/组件/结构 inline 标注
- `webui/src/components/UsageHeatmapCalendar.tsx` — 单 grid 重写(`auto repeat(26, minmax(0, 1fr))`,gap 5px,`aspect-square` cells,15/35/55/80/100% 强度)
- `webui/src/components/UsageTimeSeries.tsx` — HEIGHT 200,新 PADDING `{t:16 r:16 b:28 l:44}`,9px 坐标轴,删除 "tokens" 标注,stroke 1.5
- `webui/src/components/TokensLeaderboard.tsx` — grid 布局,11/9px 字号,36px 进度条
- `webui/src/components/UsageKpiCards.tsx` — 加第 4 张卡 "Avg / day (30d)" + 真实计算
- `webui/src/pages/UsagePage.tsx` — 删 `<h1>Usage</h1>` 标题块,body grid `1fr 230px`,内容 `max-w-[1600px] mx-auto`,删 `onClearFilter` 死代码
- `webui/src/App.tsx` — 删 `handleClearUsageFilter` 死函数

### 5 个原始 bug → 修复

| # | Bug | 修复 |
|---|---|---|
| 1 | 双底部滚动条 | `UsagePage.tsx:132` 移除 `overflow-y-auto`(单滚动上下文) |
| 2 | 窗口太窄放不下 | 内容 `max-width: 1600px` 配合窗口 1200×760 自适配 |
| 3 | 右侧滚动条消失 | 同上(单一滚动上下文,WebView2 stable gutter) |
| 4 | 顶部白色 "Usage" 字 | 删 `<h1>Usage</h1>` 标题块,LeftRail 承担标识 |
| 5 | 趋势图不可见 | HEIGHT 320→200,PADDING 重排,移除冗余 "tokens" 标注 |

### 设计稿定稿过程

1. 初版(spec 700px max-width)— 用户确认"短了",调整到 1600px 上限
2. 热力图单元格长方形 — 加 `aspect-square` + `aspect-ratio: 1`,cells 永正方形
3. 星期标签不对齐 — 改单 grid(day-labels 和 cells 共享 row indices)
4. 单元格"分开"— 改用 5px gap 的参考样式
5. 多余的 AgentPairChart 组件 — 删除,与设计稿 4 组件对齐
6. 侧栏 240→230(用户反馈"还多 10px")

### 验证

```
pnpm tsc --noEmit           → 0 errors
cargo check                 → PASS (无 Rust 改动)
Vite hot-reload             → 窗口内目视确认 4 组件,1200×760 无溢出
拖动窗口到 1800×1000        → cells 等比放大,1600px 上限封顶,不破结构
```

### 设计稿 vs 实现 漂移(已修)

- 规范 NOTES 一处写 `max-w-[1680px]`,CSS 写 `1600px`,代码 1600px → 一致
- 侧栏一处 NOTES 写 300px,实现 230px(经 240→230 收紧)→ 一致
- KPI 3 张卡 vs 4 张卡 → 加第 4 张 "Avg / day (30d)"

### 设计稿 vs 实现 漂移(未修,设计决策待用户)

- **章节标题字号** — 规范要 10px UPPERCASE,实际混用 14px 句首大写 + 12px UPPERCASE。三种样式并存于一个视图
- **圆角 token** — 规范 `--radius: 3px`,`globals.css` `--radius-sm: 8px`,代码 `rounded-sm` → 全局 token 不一致
- **Tailwind gap 近似** — `gap-4` (16px) vs 规范 14px;`gap-3` (12px) vs 规范 10px。差异 ≤ 2px,可忽略

### 死代码(留待清理,非本 session scope)

- `webui/src/components/AgentPairChart.tsx` — 不再被引用,但文件保留(数据层 `agent_pairs` 仍可用)
- `webui/src/components/UsageStatsSidebar.tsx` — 上一 stage 残留,本 stage 未检查引用
- `tauri-dev-live.{err,out,pid}` `tauri-dev.{err,out}` — dev server 临时日志
- `webui/{capture,inspect,capture-empty}.mjs` — 调试脚本,需确认是否仍需

### 下次 session 起手

1. 跑 `./init.sh`
2. 读 `feature_list.json` stage-13.3 entry
3. 决策点:
   - **立即 commit** 本次改动(working tree dirty,10 webui 改 + 1 docs 新)
   - 顺手清死代码:删 `AgentPairChart.tsx` + 检查 `UsageStatsSidebar.tsx` 引用 + 删 `tauri-dev-*` 日志
   - 解决章节标题字号不一致(10px UPPERCASE vs 14px 句首大写)— 设计决策
   - 重设 `globals.css --radius` 锁 3px 或 8px(全局)— 设计决策
4. 后续候选(用户拍板):
   - V0.2 RFC 路线(加密 / Mac port / i18n)
   - pricing.json 补 6 个 opencode model(沿 stage-13.2 遗留)
   - `claude-code::derive_provider` 加 `kimi-` prefix
   - format-number-debt

---

## 2026-06-30 (deepwork — stage-13.2)

**Session scope**: opencode 数据导入 3 个串联 bug 修复批(用户报告 "opencode 的数据仍然没有加入")
**Files changed**: 4 改 — `src-tauri/src/services/agent_parser_opencode.rs` / `opencode_go_limits.rs` / `auto_import.rs` / `token_usage.rs`;`progress.md` / `session-handoff.md` / `feature_list.json` / `docs/quota-token-reference.md`

### Bug A (HIGH) — `OpencodeParser::default_path()` 硬编码 LOCALAPPDATA

opencode Go CLI v1.17+ 在 Windows 上用 XDG 路径 `~/.local/share/opencode/opencode.db` (跟 token-monitor `discoverDbPaths` 一致),但代码只看 `%LOCALAPPDATA%\opencode\`。用户实测 5.95GB db 全在 XDG,`is_available()` 永远 false → 0 行进 DB。

**Fix**(`agent_parser_opencode.rs`):
- `candidate_paths()` 扫两个候选(`%LOCALAPPDATA%\opencode\opencode*.db` + `~/.local/share/opencode\opencode*.db`),glob 接受 `opencode[-<channel>].db` 变体,拒 WAL/SHM 边文件
- 抽 `filter_db_files(base_dir)` 纯函数(env-independent test)
- `opencode_go_limits.rs::discover_db_paths()` 同样改,删 `cfg!(target_os = "windows")` 分支(opencode Go CLI 跨平台都用 XDG)
- `parse()` 遍历所有 candidate,partial fail 进 `sample_errors` 不 abort 整批

### Bug B (HIGH 实际 blocker) — `scan_and_import_if_empty` 100-row 阈值闸

`if db_has_records(svc, 100) return empty` — 3763 行 `claude` OAuth 凭据永久挡门,opencode parser 永远轮不到。**这是用户看到"opencode 没数据"的真正原因**,不是 path。

**Fix**(`auto_import.rs`):
- `db_has_records` 标 `#[allow(dead_code)]` 保留(可能未来 per-agent-type gate 复用)
- `scan_and_import_if_empty` 直接调 `scan_and_import`,FNV-1a dedup 兜底
- 新增 `scan_and_import_if_empty_runs_when_db_already_populated` 预填 3763 行验证 gate 不会再回来

### Bug C (MEDIUM) — opencode Go `session.model` 是 JSON 不是 slash 字符串

opencode Go v1.17+ 把 model 存为 `'{"id":"kimi-k2.7-code","providerID":"opencode-go","variant":"max"}'`,parser 之前当 string 存,UI 显示整段 JSON。

**Fix**(`token_usage.rs::parse_opencode_db_records`):
- 检测到 `model.starts_with('{')` → `serde_json::from_str` → 抽 `providerID` + `id`
- 失败回退原行为;legacy `vendor/model` slash 仍支持
- 2 个 test: `import_opencode_db_unwraps_json_model` + `import_opencode_db_handles_invalid_json_model`

### 验证

```
cargo check --lib    → 0 errors
cargo test --lib     → 129/129 PASS (5 新: filter_db_files × 4 + JSON model × 2 + gate regression × 1 - 1 弃 env-dep candidate_paths_*)
pnpm tsc --noEmit    → 0 errors
```

### 端到端实测 (`cargo tauri dev`)

`%APPDATA%\com.keypilot.app\keypilot.db` 自动 import:
- 总行:18853 claude + 1166 opencode = 20019
- `last_auto_import` meta: 15091 imported / 5569 skipped / 0 errors
- opencode 1165 行扫到(WHERE `tokens_input > 0 OR tokens_output > 0` 过滤零 token 行),1 真插 + 1164 dedup(罕见同 `(model, occurred_at, input, output)` 碰撞 — FNV-1a 行为正常)

`(model, provider)` 分布:
- `MiniMax-M2.7 / minimax-cn-coding-plan` × 539
- `MiniMax-M3 / minimax-cn-coding-plan` × 321
- `unknown / opencode` × 216 (model 字段 null)
- `deepseek-v4-flash-free / opencode` × 47
- `deepseek-v4-pro / opencode-go` × 21
- `step-3.7-flash / stepfun` × 7
- `kimi-k2.7-code / opencode-go` × 1 ✅(确认 JSON 解包工作)

### 风险 / 已知遗留(非本次 scope)

- 多 model 不在 `pricing.json` → `total_cost = 0` + `cost_details.pricing_missing_for` 标记。需补 `MiniMax-M2.7` / `MiniMax-M3` / `kimi-k2.7-code` / `step-3.7-flash` / `mimo-v2.5-pro` / `deepseek-v4-flash-free`。
- `claude-code::derive_provider`(`agent_parser_claude_code.rs:268`)前缀白名单不含 `kimi-` → provider = "unknown"。
- `scan_and_import_if_empty` 每次启动扫 5.95GB SQLite(1.3k 行 ≈ 10ms OK,涨到 100k+ 时需加 per-agent-type `max(time_created)` cursor)。ponytail 标记先不优化。

### 下一阶段

- pricing.json 补 opencode 用的 model
- `claude-code::derive_provider` 加 `kimi-` prefix(顺手)
- 提交 stage-13.2(commit message 待 user 拍节奏)

---

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

**Fix 1 (HIGH) Heatmap range**: `UsagePage` 拆 2 个 `useUsageSummary` 调用 — `trendFilter`(7d/30d 给图) + `lifetimeFilter`(全期给热力图/KPI/sidebar);`isLoading` 拆 `trendLoading` + `lifetimeLoading`。

**Fix 2 (HIGH) provider filter**: `App.tsx` `usageFilterProviderId: number` → `usageFilterProviderName: string`,resolve via `allProviders.find()`。

**Fix 3 (MEDIUM) sidebar/KPI lifetime**: KPI + sidebar 全部从 `lifetimeSummary.daily_series` 派生;bonus 4th sidebar stat `Period` 显示 `trendSummary.total_requests`。

**Fix 4 (LOW) snake_case 注释**:`webui/src/types/api.ts` line 172 加注释警告 `#[serde(rename_all = "camelCase")]` 静默破坏 IPC DTO,不改字段。

**验证**: tsc clean;build 400.61 KB (+0.14 KB);cargo test 25/25;em-dash / V0.X UI / BETA / INVITE-ONLY 0 matches。

**下一步**: 用 Stage e 格式 commit,本 scope 结束。

---

## 2026-06-28 (session 3)

**Session scope**: stage-f — Agent parser abstraction + startup auto-import
**Files changed**: 4 新 + 6 改 — `services/{agent_parser,agent_parser_opencode,agent_parser_claude_code,auto_import}.rs`(新);`services/{token_usage,mod}.rs` + `database.rs` + `lib.rs` + `Cargo.toml` + `webui/src/{App,pages/UsagePage,components/UsageTimeSeries}.tsx`;`webui/{playwright.config.ts,tests/usage-page.spec.ts,tests/__screenshots__/usage-page.png}`(新)。

**架构 (user mandate)**: "做好抽象层,加 agent 只改一解析函数,不修改前端/热力图"。`AgentParser` trait + `default_parsers()` factory — 加新 agent = 1 文件 + 1 行 factory。

**实现**:
- `AgentParser` trait `Send+Sync` (agent_type / display_name / default_path / is_available / parse)
- `default_parsers()` 出 `OpencodeParser` + `ClaudeCodeParser`,Codex 适配时加 1 行
- `OpencodeParser::parse` 委托 `parse_opencode_db_records`(零 SQL 重复)
- `ClaudeCodeParser::parse` glob `~/.claude/projects/**/*.jsonl`,只解析 `type:"assistant"` 行的 `message.usage`
- `auto_import::scan_and_import_if_empty` 在 `lib.rs::setup()` 跑(>100 行 skip),`last_auto_import` meta + `auto_import_completed` event
- TopBar 条件渲染 `currentPage === 'credentials'`(Usage 页隐藏)
- Playwright visual test infra:`addInitScript` mock Tauri IPC,baseline screenshot committed

**验证**: cargo test 28/28 (3 新);tsc clean;build 394.40 KB (-6.0 KB from prev);playwright 1/1;硬约束全过。

**Open**: vite dev server 当时没起,UI 端未实烟测;auto_import_completed 事件发了无 listener;Codex parser 未做。

---

## 2026-06-28

**Session scope**: stage-d — opencode.db import adapter for TokenUsageService
**Files changed**: `services/token_usage.rs` + `commands/token_usage.rs` + `lib.rs` + `actions/{mod,token_usage}.rs` + `feature_list.json` + `progress.md`

**完成**: `TokenUsageService::import_opencode_db` 读 `opencode.db` session 表(`Connection::open_with_flags` READ ONLY+NO_MUTEX,符合 §3.1 硬约束),model split on `/` derive provider,record_usage + FNV-1a dedup 兜底,WHERE 过滤 0 token 行;IPC `import_opencode_db` (spawn_blocking 包装);Action Registry `token_usage.import_opencode_db`;3 unit tests。

**验证**: cargo test 25/25;grep fs::write / 加密 crate 0 matches;tsc clean。

**下一步**: stage-13 (UsagePage UI hookup for opencode.db file picker) deferred。

---

## 2026-06-27

**Session scope**: Stage 12 incremental polish + icon generation + dev workflow fix
**Commit**: `d70013d` — `stage-12 polish: design tokens + app icon + SettingsModal UX + dev workflow`

**完成项**: 设计 token 系统深化 (`globals.css` + `tailwind.config.ts`,新增 `accent` / `destructive` / `input` / `ring` + 排版尺码 `--font-size-*` + `--line-height-*`);Button 6 个变体修复;SettingsModal useMutation + `formatMutationError`;TopRightActions 抽取;App 图标全套 (`cargo tauri icon`);tauri.conf.json `beforeDevCommand` 修复 (pnpm-workspace.yaml issue)。

**验证**: tsc 0 errors;cargo check 7.33s;WCAG AA contrast (17.9:1 light / 14.1:1 dark)。

**遗留**(非 scope):`DESIGN.md` 是 UTF-16 LE BOM 编码;`docs/index.html` 是 UI 设计预览 HTML 不在 neat-freak scope;tray.png 还是 595B 占位。

**工具链发现**:`mmx CLI 1.0.15` Authorization 头缺 `Bearer ` 前缀,所有 API 返 code 6。手动加前缀可绕开,**应上报维护者但不动用户 config**。

---

## 2026-06-28 (session 4)

**Session scope**: stage-g — Claude parser schema fix + auto-import observability + frontend toast
**Files changed**: 10 files across `src-tauri/src/{services,commands,lib}` and `webui/src/{types,lib,hooks,App}`

**Root cause**: Claude parser 写的是假设 schema `{agent, model, timestamp, usage}`,跟 Claude Code 实际 jsonl 不符(outer `type` classifier → 只 `assistant` 行带 `message.usage.*`)。385 真实文件全 fail,errors 静默吞,UI 显示 0 行。

**Fix**: `serde_json::Value` walk,只 `assistant` 行产记录;`ParseStats { files_scanned, lines_scanned, lines_matched, lines_parse_errored, sample_errors[<=3] }` 暴露失败原因;`get_last_auto_import` Tauri command(替代 dead `auto_import_completed` event)+ App.tsx mount 查 + toast iff `imported>0||errors>0`。

**Validation**: cargo test 30/30;tsc clean;build 395.03 KB (+0.63);playwright 1/1;real-world smoke 隐式覆盖 20453 行 import。

**遗留**: provider name 对 claude 行 = "unknown"(ponytail 推迟到 schema 稳);V0.2: derive provider from model prefix;Codex parser adapter。

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