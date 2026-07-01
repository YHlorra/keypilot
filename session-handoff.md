# Session Handoff

> Per AGENTS.md §8 / §12 — 正式 session 交接。
> 目的: 让下次 session / 下个 Agent 能 `./init.sh` 起来就直接干活, 不需要问"上次搞到哪了"。

**Last updated**: 2026-07-02
**Last commit**: `e355b5d Phase 4+6: WebUI over-engineering shrink + date-fns removal` (over-engineering execution COMPLETE)
**Branch state**: `main`, working tree **clean** (pending state artifact commit)

---

## 一句话定位

KeyPilot V0.1 已交付(stage-9 完成 sign-off)。
**当前 session (2026-07-01 stage-13.3)**:UsagePage UI 收紧 — 用户截图报 5 个 bug,创建冻结设计稿 `docs/usage-page.html`,按稿实施 4 组件布局。**用户拍板定稿**。Working tree dirty,**待 commit**。

## 下次 session 起手

1. **跑 `./init.sh`** 验证环境 (AGENTS.md §1 强制)。先修 CRLF: `sed -i 's/\r$//' init.sh`
2. **读 `progress.md`** — 顶部 "2026-07-02 — Over-engineering execution" entry 是本 session 完整记录。
3. **Over-engineering execution 已完成**:
   - Rust over-engineering: commit `1231cd6` (Phase 3b, 37 Rust files, 4781 deletions)
   - WebUI over-engineering: commit `e355b5d` (Phase 4+6, 11 webui files, 305 deletions)
   - 所有 cargo + pnpm tests pass
4. **下一步侯选**(按优先级):
   - Phase 5 IPC DTO collapse (deferred — needs serde round-trip spike first)
   - Review borderline items: provider.test_and_refresh / cva / executeAction
   - Add sha2/once_cell/uuid to "considered but kept" doc for future audits
   - Clean `from_models pub(crate)` warning with `#[cfg(test)]`
5. **后续候选**:
   - pricing.json 补 6 个 opencode model
   - `claude-code::derive_provider` 加 `kimi-` prefix
   - format-number-debt
   - V0.2 RFC 评估(用户拍板)

## 当前环境状态

| 项 | 状态 |
|---|---|
| Rust 工具链 | ✅ `cargo check` PASS;`cargo test` 121/121 PASS |
| Node 工具链 | ✅ `pnpm tsc --noEmit` 0 errors;`pnpm build` PASS |
| WebUI 构建 | ✅ `pnpm build` PASS |
| Playwright | ⚠️ 未跑(over-engineering 无 UI 行为变更) |
| Tauri 2 启动 | ✅ 冷启动 → Vite + Rust 编译 + 窗口弹出 |
| SQLite db | `%APPDATA%\com.keypilot.app\keypilot.db`, **schema v6** (8 张表) |
| Over-engineering | ✅ 37 Rust files + 11 WebUI files deleted (~5,086 LOC) |
| Dep cleanups | ✅ tokio-postgres removed; date-fns removed |

## 关键文件 cheat-sheet

| 想看什么 | 看哪里 |
|---|---|
| **设计稿(冻结)** | `docs/usage-page.html` — 打开浏览器看 |
| 本 session 工作笔记 + 决策 | `progress.md` 顶部 entry;`feature_list.json` stage-13.3 entry |
| 热力图单 grid 实现 | `webui/src/components/UsageHeatmapCalendar.tsx` |
| 趋势图新尺寸 | `webui/src/components/UsageTimeSeries.tsx` (HEIGHT=200, PADDING 重排) |
| 4 组件布局 | `webui/src/pages/UsagePage.tsx` (body grid 1fr 230px) |
| KPI 第 4 张卡 | `webui/src/components/UsageKpiCards.tsx` (`AvgDayCard`) |
| 共享 format util | `webui/src/lib/format.ts` |
| 实时增量导入 | `src-tauri/src/services/incremental_import.rs` |
| Token 用量架构 / IPC 详解 | `docs/quota-token-reference.md` §4.6 |
| V0.1 验收清单 | `docs/v0.1-acceptance.md` |
| OpenSpec 归档状态 | `openspec/changes/` 只剩 `archive/` |

## 不要踩的坑

1. **不要写加密代码** — V0.1 决策明文存储。
2. **不要 fs::write 到 `%APPDATA%` 之外** — `lib.rs:28` 唯一调用。
3. **不要把 `get_usage_periods_summary` 改回返回 `crate::types::PeriodsSummary`** — 必须返回 `PeriodsSummaryResponse` IPC DTO。
4. **不要改 `notify-debouncer-full` debounce < 200ms**。
5. **不要 sync AGENTS.md → CLAUDE.md 时改内容** — Iron Rule §0:`cp` 不改。
6. **`pnpm tauri dev` 别直接跑** — 用 `cargo tauri dev`。
7. **mmx 1.0.15 不可信** — API key 手动加 `Bearer ` 前缀。
8. **Stage-13.1 KPI subLabel 决策 D1.c 已锁** — 纯数字,无 USD。
9. **Stage-13.1 chart 已用像素坐标系** — 不回退 viewBox。
10. **Stage-13.2 gate 已拆** — `scan_and_import_if_empty` 总是调 `scan_and_import`,FNV-1a dedup 兜底。**不要**加 `if db_has_records(...)` 之类"DB 已满就 skip" gate。
11. **🆕 Stage-13.3 设计稿冻结** — `docs/usage-page.html` 是唯一真相源。**不要再改设计稿**;有调整需求先和用户对齐再改稿。
12. **🆕 Stage-13.3 4 组件** — `<UsageHeatmapCalendar/>` / `<UsageTimeSeries/>` / `<TokensLeaderboard/>` / `<UsageKpiCards/>`。**不要**再加第 5 个(用户拍板"就是这 4 个")。
13. **🆕 AgentPairChart 死代码** — `webui/src/components/AgentPairChart.tsx` 不再被引用。删之前先 grep 确认无引用,不要误删。

## 待用户决策 (优先级由用户排)

1. **立即 commit stage-13.2 + stage-13.3** — 2 个未 commit 的工作,working tree 累计 ~13 改 + 1 新
2. **commit + 顺手清死代码** — `AgentPairChart.tsx` / `UsageStatsSidebar.tsx` / `tauri-dev-*` 日志 / `webui/capture*.mjs` `inspect.mjs`
3. **章节标题样式统一** — 10px UPPERCASE (规范) vs 14px 句首大写 (UsagePage h2) vs 12px UPPERCASE (sidebar/KPI)。定一个
4. **stage-13.2 遗留**:`pricing.json` 补 6 个 opencode model + `kimi-` prefix to `derive_provider`
5. **format-number-debt** — QuotaBadge + TrayHoverCard 各自 format impl dedupe
6. **V0.2 RFC 路线** — 加密 / Mac port / i18n / 自动 refresh

## 不在 scope (per AGENTS.md §3.3)

- 故障转移 / 代理 / 账号池
- MCP 管理
- 跨平台(Mac/Linux 留给 V0.3+)
- 加密(V0.2 RFC)
- 导入 / 导出 / 同步(V0.2)
- 浏览器扩展(永久不做)
- 团队协作 / 多 Vault(永久不做)

## Stage-13.3 剩余债务 (out of scope, future)

1. **章节标题样式统一** — 同一视图三种样式,设计决策
2. **圆角 token 统一** — 规范 3px vs `globals.css` 8px
3. **死代码清理** — `AgentPairChart.tsx` + `UsageStatsSidebar.tsx` + dev 日志 + 调试 mjs
4. **Playwright 视觉回归** — 4 组件新布局应跑 baseline 截图比对

---

## Next session start

Repo state at HEAD (post this session):
- Rust over-engineering execution: DONE in commit 1231cd6 (Phase 3b mega-commit absorbed Phase 1-3b work)
- WebUI over-engineering execution: DONE in commit e355b5d (Phase 4+6)
- Dep cleanups: tokio-postgres removed (Phase 2); date-fns removed (Phase 6)
- All cargo + pnpm tests pass (121 + tsc + build)

Open work:
1. Phase 5 IPC DTO collapse (deferred — needs serde round-trip spike first)
2. Review remaining borderline items: provider.test_and_refresh / cva / executeAction
3. Add sha2/once_cell/uuid to a "considered but kept" doc if relevant for future audits
4. The "from_models pub(crate)" warning could be cleaned with `#[cfg(test)]` (currently produces warning)

Run `./init.sh` (after fixing CRLF line endings with `sed -i 's/\r$//' init.sh`) to verify before any new work.

Audit artifacts:
- Audit plan: .slim/deepwork/over-engineering-execution.md
- This handoff: .slim/deepwork/session-handoff.md
- Audit history: prior session memory in knowledge base (search "over-engineering")