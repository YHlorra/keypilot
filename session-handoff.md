# Session Handoff

> Per AGENTS.md §8 / §12 — 正式 session 交接。
> 目的: 让下次 session / 下个 Agent 能 `./init.sh` 起来就直接干活, 不需要问"上次搞到哪了"。

**Last updated**: 2026-06-30
**Last commit**: `4250896 stage-13.1: UsagePage dashboard UX fix batch (4 bugs)`
**Branch state**: `main`, working tree **dirty** (3 .rs files modified by stage-13.2; cargo tauri dev 后台仍在跑 PID 38284 + cargo 33364; `tauri-dev-live.{err,out}` 新的未跟踪 dev 日志)

---

## 一句话定位

KeyPilot V0.1 已交付(stage-9 完成 sign-off)。
**当前 session (2026-06-30 stage-13.2)**:opencode 数据导入 3 个串联 bug 修复 — 路径硬编码 / 100-row 阈值闸 / JSON model 解包。**未 commit**,working tree dirty。E2E 验证:用户 5.95GB opencode db 现在能 import,1166 行进 `%APPDATA%\com.keypilot.app\keypilot.db`。

## 下次 session 起手

1. **跑 `./init.sh`** 验证环境 (AGENTS.md §1 强制)。
2. **读 `progress.md`** — 顶部 "2026-06-30 (deepwork — stage-13.2)" entry 是本 session 完整记录。
3. **stage-13.2 状态**:`feature_list.json` 已加 stage-13.2 entry (status=done),但**代码未 commit**。决策点:
   - 立即 commit 锁住 3 个 fix + 5 个 test
   - 顺便做 stage-13.2 遗留(`pricing.json` 补 opencode model + `claude-code::derive_provider` 加 `kimi-` prefix)再一起 commit
4. **如不做 stage-13.2 commit**,下次 session 起手先 commit 现有 dirty 改动再继续。
5. **后续候选**(stage-13.2 commit 后):
   - `format-number-debt` — 1-2 个文件 dedupe
   - Sidebar Period 卡 cost 显示(还原 D1.c)
   - Task 2(托盘 popover 动画)
   - DESIGN.md UTF-16 → UTF-8 编码
   - V0.2 RFC 评估(用户拍板)
   - tray.png 极简版

## 当前环境状态

| 项 | 状态 |
|---|---|
| Rust 工具链 | ✅ `cargo check` PASS;`cargo test --lib` 129/129 PASS (5 新 stage-13.2 test) |
| Node 工具链 | ✅ `pnpm tsc --noEmit` 0 errors |
| WebUI 构建 | ✅ stage-13.2 无 webui 改动,`pnpm build` 应仍 PASS |
| Playwright | ✅ npx playwright test 1/1 PASS |
| Tauri 2 启动 | ✅ `cargo tauri dev` PASS;PID 38284 仍在跑,E2E 验证已通过 |
| SQLite db | `%APPDATA%\com.keypilot.app\keypilot.db`, **schema v6** (8 张表) |
| db 总行 | **20019**:18853 claude + 1166 opencode (was 3763 claude + 0 opencode) |
| Watcher | ✅ IncrementalImporter 监听 claude-code + codex jsonl;opencode SQLite 不走 watcher(全文件读) |
| Tray icon | ⚠️ 还是 595B 占位 |
| mmx CLI | ⚠️ 1.0.15 Authorization 头 bug |

## 关键文件 cheat-sheet

| 想看什么 | 看哪里 |
|---|---|
| Stage-13.2 工作笔记 + 决策 | `progress.md` 顶部 entry;`feature_list.json` stage-13.2 entry |
| Stage-13.2 实施 + 实测数据 | `progress.md` "端到端实测" 段 |
| opencode 多候选路径 + JSON model 解析 | `docs/quota-token-reference.md` §4.5 (本 session 已更新) |
| 共享 format util | `webui/src/lib/format.ts` |
| Token 用量 schema v6 | `src-tauri/src/database.rs` (`agent_file_cursor` 表) |
| 实时增量导入实现 | `src-tauri/src/services/incremental_import.rs` (~430 行) |
| opencode 解析逻辑 | `src-tauri/src/services/agent_parser_opencode.rs` (`candidate_paths` + `filter_db_files`) |
| opencode quota 窗口 | `src-tauri/src/services/opencode_go_limits.rs` (`discover_db_paths` 已修) |
| Periods IPC DTO | `src-tauri/src/commands/token_usage.rs` (`PeriodsSummaryResponse`) |
| `force_rescan_all` IPC | `src-tauri/src/commands/token_usage.rs` + `actions/token_usage.rs` |
| `token_usage_tick` 事件 | `webui/src/hooks/useUsageTick.ts` + `webui/src/App.tsx` |
| Token 用量架构 / schema / IPC 详解 | `docs/quota-token-reference.md` §4.6 |
| V0.1 验收清单 | `docs/v0.1-acceptance.md` |
| OpenSpec 归档状态 | `openspec/changes/` 只剩 `archive/` 子目录(无 active) |

## 不要踩的坑

1. **不要写加密代码** — V0.1 决策明文存储,V0.2 评估升级方案。
2. **不要 fs::write 到 `%APPDATA%` 之外** — 唯一调用在 `lib.rs:28` (`app.path().app_data_dir()`);test 代码可用 `std::env::temp_dir()`。
3. **不要把 `get_usage_periods_summary` 改回返回 `crate::types::PeriodsSummary`** — 必须返回 `PeriodsSummaryResponse` IPC DTO。
4. **不要改 `notify-debouncer-full` 的 debounce window 低于 200ms** — 300ms 是单行 JSONL append 多次 Modify 事件的合并窗口。
5. **不要 sync AGENTS.md → CLAUDE.md 时改内容** — Iron Rule §0 要求两份内容完全相同(`cp AGENTS.md CLAUDE.md`)。
6. **不要把 openspec/ 的 active 改动自行归档** — 由 orchestrator 在确认 stage 全部 done 后做。
7. **`pnpm tauri dev` 别直接跑** — 用 `cargo tauri dev`(项目无根 `package.json`)。`tauri.conf.json:beforeDevCommand` 必须非空。
8. **mmx 1.0.15 不可信** — API key 必须手动加 `Bearer ` 前缀。
9. **Stage-13.1 KPI subLabel 决策 D1.c 已锁**(纯数字,无 USD)。改 KPI subLabel 时记住这是 user-facing decision。
10. **Stage-13.1 chart 已用像素坐标系** — 不要回退到 viewBox。
11. **🆕 Stage-13.2 gate 已拆** — `scan_and_import_if_empty` 现在总是调 `scan_and_import`,FNV-1a dedup 兜底。**不要**再加 `if db_has_records(...)` 之类的"DB 已满就 skip" gate — 上次那个 100-row 阈值把 opencode 永久挡门外了。如果未来要 per-agent-type 增量 cursor,直接在 `incremental_import.rs` 的 cursor 模型里加,不要碰 `auto_import.rs` 的 `scan_and_import`。

## 待用户决策 (优先级由用户排)

1. **立即 commit stage-13.2** — 3 文件 + 5 test 已就绪(working tree dirty)
2. **stage-13.2 commit + 顺手补 pricing.json** — 6 个 opencode model 加 `pricing.json`:`MiniMax-M2.7` / `MiniMax-M3` / `kimi-k2.7-code` / `step-3.7-flash` / `mimo-v2.5-pro` / `deepseek-v4-flash-free`。小,1 文件。
3. **stage-13.2 commit + 加 `kimi-` prefix 到 `claude-code::derive_provider`** — provider 派生从 "unknown" 改 "moonshot"(或 `kimi-` 用户指定厂商)。5 行。
4. **format-number-debt** — QuotaBadge + TrayHoverCard 各自 format impl dedupe
5. **Sidebar Period 卡 cost 显示** — Stage-13.1 D1.c 还原点
6. **Task 2(托盘 popover 动画)** — unblock,可开
7. **DESIGN.md UTF-16 → UTF-8 编码** — 历史遗留,1 命令
8. **tray.png 极简版**
9. **V0.2 RFC 路线** — 加密 / Mac port / i18n / 自动 refresh

## 不在 scope (per AGENTS.md §3.3)

- 故障转移 / 代理 / 账号池
- MCP 管理
- 跨平台(Mac/Linux 留给 V0.3+)
- 加密(V0.2 RFC)
- 导入 / 导出 / 同步(V0.2)
- 浏览器扩展(永久不做)
- 团队协作 / 多 Vault(永久不做)

## Stage-13.2 剩余债务 (out of scope, future)

1. `pricing.json` 补 6 个 opencode model — 影响 cost 显示,等用户拍板
2. `claude-code::derive_provider` 加 `kimi-` prefix — provider 字段修正
3. `scan_and_import` 5.95GB 全文件读 — 1.3k 行 ≈ 10ms OK;涨到 100k+ 时需 per-agent-type `max(time_created)` cursor
4. Stage-13.1 剩余:`format-number-debt` / `Sidebar Period cost` / `Tool × Model` 表 / `Task 2 popover`