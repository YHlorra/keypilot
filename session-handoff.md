# Session Handoff

> Per AGENTS.md §8 / §12 — 正式 session 交接。
> 目的: 让下次 session / 下个 Agent 能 `./init.sh` 起来就直接干活, 不需要问"上次搞到哪了"。

**Last updated**: 2026-06-27
**Last session commit**: `d70013d` (HEAD of `main`)
**Branch state**: `main`, clean working tree (only `_candidates/`, `.omc/`, `Cargo.toml` line-ending noise are non-tracked)

---

## 一句话定位

KeyPilot V0.1 已交付 (stage-9 完成 sign-off), 当前主线是 **Stage 12 增量 polish + 后续 V0.1.1 补丁**. 这次 session 把 stage-12 polish 封口, 同时修了 dev workflow 卡点 (`pnpm -C webui dev` 在 pnpm-workspace.yaml 存在时跑不通).

## 下次 session 起手

1. **跑 `./init.sh`** 验证环境 (这是 AGENTS.md §1 强制的).
2. **读 `progress.md`** — 看本次 session 完成了什么 + 已知遗留.
3. **读 `feature_list.json`** — 看 `stage-12-polish` 是 done, 其他 stages 状态.
4. **决定下一步 stage**: 候选有
   - 收 Stage 13 (token usage UI polish) — `feature_list.json` 没单独 stage, 但 token-usage-history openspec 已 archived, IPC + backend 都齐了, 剩下是 UI polish.
   - 启动 V0.2 RFC 评估 (加密方案 / 自动 refresh / i18n / Mac port) — 优先级由用户拍板.
   - 修 `DESIGN.md` UTF-16 → UTF-8 编码.
   - 补 `docs/screenshots/*.png` 真实截图 (替换 `.png.txt` 占位).
   - 关闭 `cargo tauri dev` 后台进程 + 补 tray.png 极简版.

## 当前环境状态

| 项 | 状态 |
|---|---|
| Rust 工具链 | ✅ `cargo check` 7.33s pass |
| Node 工具链 | ✅ `pnpm tsc --noEmit` 0 errors |
| Vite dev server | ✅ 跑在 `http://localhost:1420` (从 `cargo tauri dev` 启动) |
| Tauri WebView | ✅ 进程在跑 (`target\debug\keypilot.exe`), 用户正在看新设计 |
| mmx CLI | ⚠️ 1.0.15 Authorization 头 bug, 需用 `Invoke-WebRequest` 绕开 |
| SQLite db | ✅ `%APPDATA%\com.keypilot.app\keypilot.db`, schema v3 |
| Tray icon | ⚠️ 还是 595B 占位, 后续 stage 修 |

## 关键文件 cheat-sheet

| 想看什么 | 看哪里 |
|---|---|
| 设计 token 定义 | `webui/src/styles/globals.css` (lines 7-142) |
| Tailwind 颜色映射 | `webui/tailwind.config.ts` (lines 20-50) |
| 后端 schema v3 | `src-tauri/src/database.rs` |
| IPC 命令清单 | `src-tauri/src/commands/*.rs` + `src-tauri/src/lib.rs` |
| 前端 IPC 包装 | `webui/src/lib/api.ts` |
| Action Registry (Agent 可发现) | `src-tauri/src/actions/mod.rs` + `webui/src/lib/action-registry.ts` |
| Token 用量 schema v4 | `src-tauri/src/database.rs` (4 tables) + `src-tauri/data/migrations/v3_to_v4.sql` |
| Token 用量定价 | `src-tauri/data/pricing.json` (50 models) |
| OpenSpec 已 archive | `openspec/changes/archive/{v0.1-spec-alignment,v0.1-general-credentials,ui-redesign,token-usage-history}/` |
| 已删 openspec | (none — `openspec/changes/` 只剩 `archive/` 子目录) |
| V0.1 验收清单 | `docs/v0.1-acceptance.md` (13 项, V0.1 已全绿) |

## 不要踩的坑

1. **不要写加密代码** — V0.1 决策明文存储, V0.2 评估升级方案. `grep -E "argon2|chacha20|aes-gcm"` 应空.
2. **不要 fs::write 到 `%APPDATA%` 之外** — `fs::create_dir_all` / `fs::write` 唯一调用在 `src-tauri/src/lib.rs:25`, 目标是 `app.path().app_data_dir()`.
3. **不要新增依赖让 schema 漂移** — schema v3 已锁定 (provider_fields.value 明文 + visibility 二态), v0.2 才考虑升 v4 (token usage 已用 v4 但独立 migration).
4. **不要改 AGENTS.md §3.4 锁定的 UI 栈** — shadcn/ui CLI + @radix-ui/colors + React 18 + TS + Vite + TanStack Query v5. 禁默认 Tailwind palette, 禁 `@radix-ui/themes`.
5. **不要同步 AGENTS.md → CLAUDE.md 时改内容** — Iron Rule §0 要求两份内容完全相同.
6. **不要把 openspec/ 的 active 改动自行归档** — 由 orchestrator 在确认 stage 全部 done 后做.
7. **`pnpm tauri dev` 别直接跑** — 用 `cargo tauri dev` (项目无根 `package.json`), `tauri.conf.json` 的 `beforeDevCommand` 已修.
8. **mmx 1.0.15 不可信** — API key 必须手动加 `Bearer ` 前缀 (via `Invoke-WebRequest` 或 `curl`).

## 待用户决策 (优先级由用户排)

1. tray.png 是否要单独生成极简版 (推荐: 16/24px 同心环单元素)
2. V0.2 RFC 路线 (加密方案三选一: SQLCipher / master password + argon2id / DPAPI)
3. V0.2 是否引入 Mac port
4. V0.2 是否引入自动 refresh + 低额度告警
5. i18n 优先级 (V0.2 还是 V0.3)
6. DESIGN.md UTF-16 → UTF-8 是否要做
7. mmx 1.0.15 bug 是否要单独记一份 issue (写到 `~/.mmx/ISSUE.md` 或类似位置)
8. PM 文档 (`../PM思考工厂/keypilot/`) 跟 dev 的对齐状态 — 上次同步是 2026-06-24, 现在有大量增量未回写

## 不在 scope (per AGENTS.md §3.3)

- 故障转移 / 代理 / 账号池
- MCP 管理
- 跨平台 (Mac/Linux 留给 V0.3+)
- 加密 (V0.2 RFC)
- 导入 / 导出 / 同步 (V0.2)
- 浏览器扩展 (永久不做)
- 团队协作 / 多 Vault (永久不做)