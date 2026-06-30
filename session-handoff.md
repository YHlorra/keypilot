# Session Handoff

> Per AGENTS.md §8 / §12 — 正式 session 交接。
> 目的: 让下次 session / 下个 Agent 能 `./init.sh` 起来就直接干活, 不需要问"上次搞到哪了"。

**Last updated**: 2026-06-30
**Last session work**: token-usage-history bug-fix batch (Bug #1/#2/#3 修复 + schema v5→v6 + 实时增量导入)
**Branch state**: `main`, working tree has 13 modified + 3 untracked (Bug-fix batch, 未 commit)

---

## 一句话定位

KeyPilot V0.1 已交付(stage-9 完成 sign-off)。当前主线是 **token-usage-history bug-fix batch + 后续 V0.1.x 补丁**。
本次 session 修了 Bug #1(periods IPC DTO 字段错位)、Bug #2(`list_usage_records` 入参 + `PaginatedResponse` 命名)、Bug #3(`scan_and_import_if_empty` no-op → `agent_file_cursor` + notify watcher + `token_usage_tick` 实时事件 + `force_rescan_all` IPC)。

## 下次 session 起手

1. **跑 `./init.sh`** 验证环境 (这是 AGENTS.md §1 强制的)。
2. **读 `progress.md`** — 看 2026-06-30 session entry 的"下一步"段。
3. **读 `feature_list.json`** — 检查 stage 状态(目前 stages 1-9 全 done,但没新增 bug-fix stage)。
4. **决定下一步**。候选:
   - **Task 2**: 托盘 popover 动画(等用户验证 Task 1 数据流正常后再开)。
   - **手动烟测 Task 1**: 见 `progress.md` 的"手动烟测步骤"段,验证 `pnpm tauri dev` 后 Usage 页 KPI 卡不再显示 "0.00 USD"。
   - **修 `DESIGN.md` UTF-16 → UTF-8 编码**(历史遗留)。
   - **启动 V0.2 RFC 评估**(加密方案 / 自动 refresh / i18n / Mac port)— 优先级由用户拍板。

## 当前环境状态

| 项 | 状态 |
|---|---|
| Rust 工具链 | ✅ `cargo check` PASS;`cargo test --lib` 117/117 PASS |
| Node 工具链 | ✅ `pnpm tsc --noEmit` 0 errors |
| SQLite db | `%APPDATA%\com.keypilot.app\keypilot.db`, **schema v6** (8 张表) |
| Watcher | ✅ `IncrementalImporter` 启动,监听 `~/.claude/projects/**/*.jsonl` + `~/.codex/sessions/**/*.jsonl`,300ms debounce + 30s fallback poll |
| Tray icon | ⚠️ 还是 595B 占位,后续 stage 修 |
| mmx CLI | ⚠️ 1.0.15 Authorization 头 bug,需用 `Invoke-WebRequest` 绕开 |

## 关键文件 cheat-sheet

| 想看什么 | 看哪里 |
|---|---|
| Token 用量 schema v6 | `src-tauri/src/database.rs` (`agent_file_cursor` 表 + cursor CRUD) |
| 实时增量导入实现 | `src-tauri/src/services/incremental_import.rs` (~430 行,新文件) |
| Periods IPC DTO(Bug #1 修复) | `src-tauri/src/commands/token_usage.rs` (`PeriodsSummaryResponse` + 转换函数) |
| `force_rescan_all` IPC(Bug #3 escape hatch) | `src-tauri/src/commands/token_usage.rs` + `src-tauri/src/actions/token_usage.rs` |
| `token_usage_tick` 事件监听 | `webui/src/hooks/useUsageTick.ts` (新) + `webui/src/App.tsx` 接入 |
| Token 用量架构 / schema / IPC 详解 | `docs/quota-token-reference.md` §4.6 |
| V0.1 验收清单(已加 #20/#21 manual checks) | `docs/v0.1-acceptance.md` |
| OpenSpec 归档状态 | `openspec/changes/` 只剩 `archive/` 子目录(无 active) |

## 不要踩的坑

1. **不要写加密代码** — V0.1 决策明文存储,V0.2 评估升级方案。
2. **不要 fs::write 到 `%APPDATA%` 之外** — 唯一调用在 `lib.rs:25` (`app.path().app_data_dir()`)。
3. **不要把 `get_usage_periods_summary` 改回返回 `crate::types::PeriodsSummary`** — 必须返回 `PeriodsSummaryResponse` IPC DTO(已含 `total_cost_usd` + `token_breakdown`)。Bug #1 修复需要 IPC 层独立 DTO,不能让前端依赖内部类型。
4. **不要改 `notify-debouncer-full` 的 debounce window 低于 200ms** — 300ms 是单行 JSONL append 多次 Modify 事件的合并窗口,过低会触发 N 次 emit。
5. **不要 sync AGENTS.md → CLAUDE.md 时改内容** — Iron Rule §0 要求两份内容完全相同。
6. **不要把 openspec/ 的 active 改动自行归档** — 由 orchestrator 在确认 stage 全部 done 后做。本次 session 已确认 `openspec/changes/` 下无 active 改动,无需归档操作。
7. **`pnpm tauri dev` 别直接跑** — 用 `cargo tauri dev`(项目无根 `package.json`),`tauri.conf.json` 的 `beforeDevCommand` 已修。
8. **mmx 1.0.15 不可信** — API key 必须手动加 `Bearer ` 前缀。

## 待用户决策 (优先级由用户排)

1. Task 2(托盘 popover 动画)— 当前 blocked,等 Task 1 验证
2. tray.png 是否要单独生成极简版(推荐: 16/24px 同心环单元素)
3. V0.2 RFC 路线(加密方案三选一: SQLCipher / master password + argon2id / DPAPI)
4. V0.2 是否引入 Mac port
5. V0.2 是否引入自动 refresh + 低额度告警
6. i18n 优先级(V0.2 还是 V0.3)
7. DESIGN.md UTF-16 → UTF-8 是否要做
8. PM 文档(`../PM思考工厂/keypilot/`)跟 dev 的对齐状态 — 上次同步是 2026-06-24

## 不在 scope (per AGENTS.md §3.3)

- 故障转移 / 代理 / 账号池
- MCP 管理
- 跨平台(Mac/Linux 留给 V0.3+)
- 加密(V0.2 RFC)
- 导入 / 导出 / 同步(V0.2)
- 浏览器扩展(永久不做)
- 团队协作 / 多 Vault(永久不做)