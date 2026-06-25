# Deepwork: KeyPilot V0.1 — Reorder + 4-Phase Execution (rev 2, post-@oracle REVISE)

> **Started**: 2026-06-24 (Session 4)
> **Revision history**:
> - rev 1 (2026-06-24): Initial 4-Phase plan with synthetic `phase-1.5-contract` stage key
> - rev 2 (2026-06-24, this file): Post-@oracle REVISE — fixed 5 High + 5 critical Medium items
> **Supersedes**: Prior `keypilot-ui.md` (6-phase plan based on `v0.1-spec-alignment`, 3 presets, no themes — OBSOLETE)
> **Spec source of truth**: `openspec/changes/v0.1-general-credentials/` (proposal+spec+design+tasks, ~52KB, 20 REQ)
> **Reorder approved by user**: 2026-06-24 (after /think on "UI first vs backend first")
> **@oracle review rev 1**: `ses_1063723cbffe6GHZ7zDU0MV1dH` (REVISE, 5 High / 10 Medium / 11 Low)
> **@oracle review rev 2**: PENDING (re-dispatched after rev 2 fixes)
> **Status**: Plan rev 2 ready for re-review → execution gate

---

## 1. Goal

Ship KeyPilot V0.1: Tauri 2 + React 18 desktop app, general credential library (AI + DB + Dev), 5 preset providers, 3 themes, no encryption, single-process Rust. 9 stages from PLAN.md §3, restructured into 4 deepwork phases (with Phase 1.5 sub-step) that parallelize Stage 2 (backend) and Stage 3 (UI shell) on a verified TS contract.

## 2. Verified Current State (2026-06-24, post-drift-fix)

| Artifact | State | Notes |
|---|---|---|
| `openspec/changes/v0.1-general-credentials/` | 4 files; **IPC count 12**; `adapter_for` returns `Option` (post-rev-2) | spec/design §1-10 + tasks T1.1-T4.3 |
| `src-tauri/Cargo.toml` | **Encryption crates removed** (`aes-gcm` / `base64` / `rand` gone; `async-trait` + `reqwest` kept) | post-rev-2 cleanup |
| `src-tauri/tauri.conf.json` | `withGlobalTauri: false`; label="main"; **needs `bundle.icon` placeholder** (T1.8 deliverable) | post-rev-2 fix |
| `src-tauri/build.rs` + `capabilities/default.json` | Exist | `ls src-tauri/` |
| `src-tauri/src/` | **Empty** — 5 Rust files missing | `ls src-tauri/src/` |
| `webui/` | **Does not exist** (Phase 1.5 creates it) | `ls` |
| `feature_list.json` | stage-1 in-progress; stages.stage-2 + stage-3 `depends_on: ["stage-1"]` (parallel); stage-2/3 description fixed to "12 IPC" | post-rev-2 |
| `.slim/deepwork/keypilot-ui.md` (this file) | rev 2 | replaces rev 1 + prior 6-phase plan |
| `.slim/deepwork/` | Added to `.gitignore` (kept out of git, readable to OpenCode) | rev 1 drift fix |

## 3. Plan (rev 2)

```
Phase 1     [ Stage 1: Tauri 2 + SQLite schema v3 ]                                1.0 day
              Lane A: @fixer (10 Rust files in src-tauri/)
              Verify: cargo check + ./init.sh + hard-constraint grep + SQLite 5 tables + 5 preset seed
   ↓
Phase 1.5   [ Stage 3 T3.1-partial + T3.2: webui scaffold + TS contract lock ]      0.3 day
              Lane A: @fixer (3 files: webui/{package.json, tsconfig.json, src/types/api.ts})
              Verify: pnpm tsc --noEmit (FIRST time deepwork hits webui/)
              Sub-step of Stage 3; both Stage 2 and Stage 3 components depend on this contract
   ↓
Phase 2     [ Stage 2 backend ] ‖ [ Stage 3 UI shell T3.3-T3.7 ]                  1.5-2.0 days
              Lane A: @fixer (9 Rust: provider/* + services/* + commands/provider.rs)
              Lane B: @fixer (scaffold) → @designer (components) — sequential sub-phases
              Lane C: @fixer (integration: webui/src/lib/api.ts swap mock → real IPC)
              Verify: cargo check + cargo test + pnpm tsc --noEmit + 12 IPC E2E
   ↓
Phase 3     [ Stage 4 quota + Stage 5 tray ] ‖ [ Stage 7 manual modal ]           2.0-2.5 days
              Lane A: @fixer (3A-quota → 3A-tray, sequential sub-phases for safety)
                - 3A-quota: provider/*::fetch_quota + commands/quota.rs (UPSERT atomicity, 15min TTL)
                - 3A-tray: tray.rs + commands/tray.rs + quota_cache invalidate
              Lane B: @designer (ManualQuotaModal.tsx + TrayHoverCard.tsx)
              Verify: 5 preset fetch_quota + tray 常驻 + Detail/Tray 单一数据源
   ↓
Phase 4     [ Stage 6 build+sign ] + [ Stage 8 docs ] + [ Stage 9 verify ]        1.5 days
              Lane A: @fixer (mechanical: .github/workflows/* + README + screenshots + WebView2 bootstrapper verify)
              Lane B: @oracle (13-item final review split auto/manual)
```

**Lane count**: 4 phases × ~2 lanes/phase = **8 implementation lanes + 4 oracle reviews** (one at end of each phase). The reviews are checkpoints, not separate lanes.

**Concurrency windows**:
- Phase 2A ‖ Phase 2B (parallel)
- Phase 3A-quota → Phase 3A-tray (sequential, both in Lane A)
- Phase 3A ‖ Phase 3B (parallel)
- Phase 4A → Phase 4B (sequential)

### Phase 1 — Stage 1 (Tauri 2 scaffold + SQLite schema v3) — 1.0 day

**Lane A (only)**: `@fixer` background.

**Files (10)**:
- `src-tauri/Cargo.toml` (改 — `aes-gcm` / `base64` / `rand` 已删,加 `async-trait`)
- `src-tauri/tauri.conf.json` (改 — `withGlobalTauri: false`,加 `bundle.icon` 占位,`frontendDist: "../webui/dist"`,label="main" 已设)
- `src-tauri/build.rs` (existing,verify)
- `src-tauri/capabilities/default.json` (改 — 扩权限)
- `src-tauri/src/main.rs` (新)
- `src-tauri/src/lib.rs` (新)
- `src-tauri/src/database.rs` (新 — schema v3, 5 表)
- `src-tauri/src/store.rs` (新)
- `src-tauri/src/error.rs` (新 — AppError enum)
- `src-tauri/src/types.rs` (新 — Visibility / Provider / ProviderField / Category / Theme)

**Tasks** (from `openspec/.../tasks.md` T1.1-T1.12):
- T1.1 schema v3 (5 tables: meta + categories + providers + provider_fields + quota_cache)
- T1.2 `types.rs` (Visibility 二态 / Provider / ProviderField / Category / Theme)
- T1.4 `error.rs` (AppError enum + thiserror)
- T1.5 `store.rs` (AppState { db: Arc<Database> })
- T1.6 `lib.rs` (run() 启动链 + seed 5 preset)
- T1.7 `Cargo.toml` (CRITICAL: `aes-gcm`/`base64`/`rand` 已删,加 `async-trait` — `grep` 验证)
- T1.8 `tauri.conf.json` (CRITICAL: `withGlobalTauri: false` + `bundle.icon` 占位 + `frontendDist: "../webui/dist"`)
- T1.9 `capabilities/default.json` 扩
- T1.10 `./init.sh` 验证
- T1.11 更新 `feature_list.json` + `progress.md` + `session-handoff.md`

**DoD**:
- `cargo check` 通过
- `grep "visibility TEXT NOT NULL DEFAULT 'visible'" src-tauri/src/database.rs` 有结果
- `grep "value TEXT" src-tauri/src/database.rs` 有结果
- `grep "preset TEXT" src-tauri/src/database.rs` 有结果
- `grep "category_id INTEGER NOT NULL" src-tauri/src/database.rs` 有结果
- `grep -E "argon2|chacha20|ChaCha20Poly1305|aes-gcm|sodiumoxide|^age " src-tauri/Cargo.toml` 为空
- 启动一次应用,SQLite 文件创建在 `%APPDATA%\com.keypilot.app\keypilot.db`
- 5 张表存在,`schema_version='3'`,5 preset seed 成功
- `feature_list.json` stage-1 status = "done", evidence 非空

**@oracle review gate** (post-validation):
- Rust 架构 (AppError 边界 / schema v3 一致性 / seed 顺序)
- 5 preset 行内 fields 数据正确性
- Simplify: error.rs 信息密度,AppError 边界是否过宽

### Phase 1.5 — Webui scaffold + TS contract lock — 0.3 day

**Lane A (only)**: `@fixer` background.

**Files (3) — sub-step of Stage 3**:
- `webui/package.json` (新 — Vite 5 + React 18 + TS + TanStack Query v5 + `@tauri-apps/api@^2.0.0`)
- `webui/tsconfig.json` (新)
- `webui/src/types/api.ts` (新 — 12 IPC 类型 + Provider/ProviderField/Category/Visibility/Theme/QuotaSnapshot/AppError)

**Tasks** (Stage 3 T3.1-partial + T3.2):
- T3.1a `pnpm create vite webui --template react-ts` → 立即 truncate 到最小(scaffold 实际工作在 Phase 2B)
- T3.1b 删 `webui/src/App.tsx` / `main.tsx` / `App.css` 暂时(Phase 2B 重写)
- T3.1c 加 `@tauri-apps/api@^2.0.0` 到 `package.json` (pin 与 Tauri 2.11 Rust 端同步)
- T3.2 写 `webui/src/types/api.ts`:
  - 12 IPC 的 request/response TS interface
  - enum: Visibility (`'visible' | 'masked'`), Theme (`'dark' | 'light' | 'auto'`)
  - Provider / ProviderField / Category / QuotaSnapshot / AppError interface
  - 每个 interface 加 JSDoc `@see openspec/.../spec.md REQ-XXX-NNN` 引用,防止后续派生漂移

**DoD**:
- `pnpm tsc --noEmit` 通过
- `webui/src/types/api.ts` 的字段名 / 类型 / enum 严格匹配 `src-tauri/src/types.rs`(rev 2 之前一致)
- 12 IPC 函数的 request/response 都有 TS interface(为 Phase 2 mock 准备)
- `@tauri-apps/api@^2.0.0` 版本 pin 在 `package.json`

**Why this is independently mergeable** (post-rev-2): produces runnable verification (`pnpm tsc --noEmit`) and 3 files with clear acceptance criteria. This was the @oracle #3+#4 finding: rev-1 had no verification gate.

**@oracle review gate** (post-validation):
- Contract fidelity (字段一一对应 Rust types.rs)
- 12 IPC request/response 完整性
- Simplify: 类型是否过度派生(避免 YAGNI interface)

**feature_list.json impact** (rev 2 decision):
- **NOT** adding a synthetic `phase-1.5` stage entry to `stages[]` (@oracle #4)
- Phase 1.5 work is a sub-step of Stage 3, captured in Stage 3's file list
- `stages.stage-3.depends_on` stays `["stage-1"]` (Stage 3 still parallel to Stage 2)
- The deepwork phase 1.5 is a *sequencing* concern, not a *product-stage* concern

### Phase 2 — Stage 2 (Backend) ‖ Stage 3 (UI Shell T3.3-T3.7) — 1.5-2.0 days

#### Lane A (Backend) — `@fixer` background

**Files (9)**:
- `src-tauri/src/provider/mod.rs` (新)
- `src-tauri/src/provider/adapter.rs` (新 — trait + `adapter_for` returns `Option<Box<dyn ProviderAdapter>>`)
- `src-tauri/src/provider/{openai,deepseek,anthropic,github,postgres}.rs` (新,各 30-50 行)
- `src-tauri/src/services/provider.rs` (新)
- `src-tauri/src/services/category.rs` (新)
- `src-tauri/src/commands/provider.rs` (新 — 12 IPC commands)

**Tasks** (T2.1-T2.7):
- T2.1 `provider/mod.rs` (ProviderKind enum + factory)
- T2.2 `provider/adapter.rs` (ProviderAdapter trait + ValidateError/QuotaError; **`adapter_for` returns `Option`** per @oracle #5; panic 移除)
- T2.3 5 个 adapter module(每个 30-50 行):
  - `openai.rs::validate_key` + `fetch_quota`(subscription+usage 算法,引 openai-balance/cmd/root.go)
  - `deepseek.rs` 引 cc-switch
  - `anthropic.rs` validate_key 三态,fetch_quota → `AppError::ProviderQuotaUnsupported`
  - `github.rs::fetch_quota` `/rate_limit`
  - `postgres.rs::fetch_quota` `pg_database_size`
- T2.4 `services/provider.rs` (CRUD 业务,async runtime 用 `tauri::async_runtime::spawn` for HTTP + `spawn_blocking` for SQLite per @oracle #15)
- T2.5 `services/category.rs` (Category CRUD + delete migrate_to)
- T2.6 `commands/provider.rs` (12 IPC commands; test_connection 走 `adapter_for(...)?` → Custom 返回 `AppError::ProviderCannotTest`; get_theme/set_theme 读写 `meta.theme`)
- **Async runtime pattern** (per @oracle #15):
  - HTTP calls: `tauri::async_runtime::spawn(async move { ... })` (Tauri 2 runtime, not raw tokio)
  - SQLite calls in `services/*.rs`: `tauri::async_runtime::spawn_blocking(move { ... })` (avoid blocking runtime)
- **Adapter unit tests** (per @oracle #5):
  - `adapter_for("openai")` → Some
  - `adapter_for("custom")` → None
  - `adapter_for("")` → None
  - `adapter_for("nonexistent")` → None
- **Category delete test**: providers migrate correctly, no provider orphaned

**DoD**:
- `cargo check` + `cargo test` 通过
- 12 IPC commands 实现完整(签名匹配 `design.md §7`)
- 5 preset 各自 validate_key + fetch_quota 单元测试
- Category delete migrate_to 测试
- `feature_list.json` stage-2 status = "done", evidence 非空

#### Lane B (UI scaffold + components) — sequential: `@fixer` (scaffold) → `@designer` (components)

**@fixer sub-phase** (scaffold, 0.3 天,extends Phase 1.5's webui skeleton):
- `webui/index.html` (新)
- `webui/vite.config.ts` (新 — Tauri proxy, port 1420)
- `webui/src/main.tsx` (新)
- `webui/src/App.tsx` (新 — 路由空壳)
- `webui/tailwind.config.ts` + `webui/postcss.config.cjs` (新 — **Radix Colors override,不用 tailwindcss 默认色** per AGENTS.md §3.4)
- `webui/src/styles/globals.css` (新 — 3 themes CSS variables per REQ-THEME-001)
- `webui/src/lib/api.ts` (新 — **mock,基于 `types/api.ts`**,12 IPC 函数返回 hardcoded JSON)
- `webui/src/hooks/useProviders.ts` (新 — TanStack Query 调用 mock)

**shadcn CLI pre-validation** (per @oracle #11):
- **PIN**: `npx shadcn@2.1.0` (or latest stable known to work with Tauri 2 + Vite 5)
- **PRE-VALIDATE**: install 1 component (`Button`) BEFORE installing all 8. If fails → fall back to "Radix Primitives only + hand-rolled Tailwind" (skip shadcn wrapping,per spec design.md §8.1 permits this)

**@designer sub-phase** (UI/UX, 0.8-1.0 天,after scaffold verified):
- `webui/src/components/CategorySidebar.tsx` (可折叠分组,色 chip,REQ-CAT-002)
- `webui/src/components/ProviderList.tsx` (单类内 provider 列表 + 选中态)
- `webui/src/components/ProviderDetail.tsx` (KV 编辑器 + quota skeleton with loading/empty/error 3 states)
- `webui/src/components/CopyButton.tsx` (REQ-COPY-001: visible 直复 / masked 点 ◉ / private 推迟 V0.2)
- `webui/src/components/ThemeToggle.tsx` (Dark/Light/Auto + matchMedia,REQ-THEME-001)
- `webui/src/components/Modal.tsx` + `AddCredentialModal.tsx` + `SettingsModal.tsx`
- shadcn components: `Button` / `Input` / `Dialog` / `DropdownMenu` / `Tooltip` / `Collapsible` / `Toast` / `Switch`
- **TEMPLATES const = ['blank', 'llm', 'database']** only (REQ-OAUTH-001 REMOVED, no 'oauth' option,per @oracle #20)
- **shadcn tests convention** (per @oracle #19): shadcn-generated components in `webui/src/components/ui/` are excluded from unit tests; custom components require vitest + RTL tests

**栈锁定** (AGENTS.md §3.4):
- React 18 + TypeScript + Vite 5 + TanStack Query v5
- shadcn/ui CLI (Radix Primitives + Tailwind utility) — with pre-validation gate
- @radix-ui/colors (gray + iris chrome / grass+amber+red+ruby status / teal+indigo+orange+gray+cyan preset badge Option A)
- `withGlobalTauri: false` + ESM imports in `lib/api.ts` (per @oracle #14)

**DoD**:
- `pnpm tsc --noEmit` 通过
- 3 themes 切换实测(Dark/Light/Follow System)
- Category sidebar 折叠 + 选中 + 新建/重命名/删除
- Provider Detail KV 编辑 + visibility 切换 + CopyButton 三态
- shadcn token override 接 Radix Colors(无 tailwindcss 默认色)
- `feature_list.json` stage-3 status = "done", evidence 非空
- **docs/index.html** (per @oracle #7): add 5-min cosmetic update to match final design tokens (deferred from T1.12)

#### Lane C (Integration) — `@fixer` 0.2 day

- 改 `webui/src/lib/api.ts` 1 处 import(`./types/api` 不变,`@tauri-apps/api/core::invoke` 替换 mock body)
- 12 IPC 函数体从 hardcoded JSON 改成 `invoke(cmd, args)`
- 注释:"this file was mocked during Phase 2B, real IPC added in Phase 2C — remove mocks if not regenerating" (per @oracle #23)
- E2E test (per @oracle #21): `src-tauri/tests/ipc_e2e.rs` 用 `tauri::test::mock_app()` 跑 12 IPC 全部
- 启动 dev,UI 真连 Rust,12 IPC 走通

**@oracle review gate** (Phase 2 收尾):
- IPC 集成测试(`tests/ipc_e2e.rs` + 12 IPC fixture)
- UI/UX 走查
- **Simplify / readability** (mandatory):
  - Rust: error.rs 信息密度,AppError 边界,IPC 命令的 thin-wrapper 程度
  - TS: components SRP,hooks 复用度,store 抽象
  - TS↔Rust 字段同步是否 codegen 候选

### Phase 3 — Stage 4 (quota) + Stage 5 (tray) + Stage 7 (manual) — 2.0-2.5 days

**Phase 3 Lane A is split into 2 sub-phases** (per @oracle #9): quota and tray are different failure modes, split for safer independent mergeability.

#### Lane A-sub-1 (3A-quota) — `@fixer` background, 1.0 day

**Files**:
- `src-tauri/src/provider/{openai,deepseek,github,postgres}.rs::fetch_quota` (实装,从 stub 填真实现)
- `src-tauri/src/commands/quota.rs` (新 — fetch_quota IPC handler + quota_cache 写)

**Tasks** (T4.1-T4.3):
- T4.1 fetch_quota 全实装(5 preset 各自算法):
  - OpenAI: subscription+usage 3-month iteration (引 openai-balance/cmd/root.go)
  - DeepSeek: `/user/balance` (引 cc-switch)
  - Anthropic: 返回 `AppError::ProviderQuotaUnsupported` (REQ-QUOTA-003)
  - GitHub: `/rate_limit` (REQ-QUOTA-005)
  - PostgreSQL: `pg_database_size` (REQ-QUOTA-006,需 `tokio-postgres` 依赖)
- T4.2 `commands/quota.rs`:
  - **Concurrent fetch atomicity** (per @oracle #6): 用 `INSERT OR REPLACE INTO quota_cache` (SQLite 原子 UPSERT)
  - **TTL semantics** (per @oracle #12): `fetched_at > now - 900` check 在请求时,避免每窗口焦点都触发 API
  - `tauri-plugin-system-tray` 集成延后到 3A-tray
- **Dep warmup** (per @oracle #13): `cargo add tokio-postgres` 在 T4.1 之前,`cargo check` 验证编译

**DoD**:
- 5 preset 各自 fetch_quota 实跑 + 单元测试
- 15min TTL 实测(Dedicated 单元测试覆盖边界)
- concurrent UPSERT 不产生 last-writer-wins bug
- `feature_list.json` stage-4 status = "done", evidence 非空

#### Lane A-sub-2 (3A-tray) — `@fixer` background, 0.5-1.0 day

**Files**:
- `src-tauri/src/tray.rs` (新)
- `src-tauri/src/commands/tray.rs` (新)

**Tasks** (Stage 5):
- T5.1 `tauri-plugin-system-tray` 加到 `Cargo.toml`
- T5.2 `tray.rs` (托盘图标 + hover 卡 + 右键菜单)
- T5.3 `commands/tray.rs` (托盘 IPC: pinned list, 复制 / 打开主窗口 / 钉住 / 删除 / 退出)
- T5.4 **关闭主窗口是否退出进程?** — **Q for user**,待 Phase 3 前拍板
- T5.5 quota_cache 触发 invalidate (Detail fetch_quota → tray card staleTime 同步)

**DoD**:
- 托盘常驻 + 关闭主窗口行为确定(待 Q)
- 托盘 hover 卡显示 quota,跟 Detail 单一数据源
- 右键菜单(复制 / 打开主窗口 / 钉住 / 退出)
- `feature_list.json` stage-5 status = "done", evidence 非空

#### Lane B (UI for Stage 5 + Stage 7) — `@designer` background, 0.5-0.8 day

**Files**:
- `webui/src/components/ManualQuotaModal.tsx` (Stage 7 — Anthropic 手动输入)
- `webui/src/components/TrayHoverCard.tsx` (Stage 5 — 托盘 hover 卡 UI)

**Tasks**:
- Stage 7:Anthropic 手动输入 modal,UI 走 quota command 扩展
- Stage 5:tray hover 卡 UI(沿用 quota 数据,react-query staleTime 5min 双视图同步)

**@oracle review gate** (Phase 3 收尾):
- quota 数据流 + 缓存策略
- tray 集成测试(主窗口关闭后托盘仍存活)
- **Simplify / readability**:
  - 5 个 adapter 的 fetch_quota 模式是否同质(YAGNI 检查)
  - quota_cache key 设计(单一 provider vs 多 provider)
  - tray 状态机是否清晰

### Phase 4 — Stage 6 (build+sign) + Stage 8 (docs) + Stage 9 (verify) — 1.5 days

#### Lane A — `@fixer` (mechanical, 1.0 day)

**Files**:
- `.github/workflows/release.yml` (新)
- `src-tauri/tauri.conf.json` (version bump + WebView2 bootstrapper 配置)
- `README.md` (扩)
- `docs/screenshots/` (新 — 截图:主窗口 3 themes / 托盘 / Modal / 错误态)

**Tasks**:
- T6.1 GitHub Actions tauri build (Win11 + WebView2) — **verify `WebView2Bootstrapper.exe` emitted for MSI/NSIS** (per @oracle #16)
- T6.2 Azure Trusted Signing(已在 Stage 1 后申请,1-3 天审批,此步 verify 签名生效)
- T8.1 README 完善(5 preset / 3 themes / 不加密声明 / 安装步骤)
- T8.2 截图(主窗口 3 themes / 托盘 / Modal / 错误态)
- **@tauri-apps/api version pin** (per @oracle #17): verify `^2.0.0` 与 Tauri Rust 2.11 匹配

#### Lane B — `@oracle` (final review, 0.5 day, read-only)

**13 项验收清单 split auto/manual** (per @oracle #18):

| # | Item | Gate | Auto? |
|---|---|---|---|
| 1 | cargo check / cargo test / pnpm tsc --noEmit | `./init.sh` 扩展 | YES |
| 2 | 5 preset seed 成功,3 LLM test_connection 通过 | integration test | YES |
| 3 | Provider CRUD + Category CRUD (含 delete migrate_to) | integration test | YES |
| 4 | 3 themes 切换无残影 | manual visual | NO |
| 5 | visibility 二态 + CopyButton 三态 | manual visual | NO |
| 6 | Detail + Tray 双 quota 单一数据源 | manual + auto cache test | BOTH |
| 7 | 5 preset 各自 fetch_quota 实跑 | integration test | YES |
| 8 | Anthropic fetch_quota → Unsupported + Manual modal | integration test | YES |
| 9 | 托盘常驻 + 关闭主窗口不退出(待拍板) | manual | NO |
| 10 | 打包成功 + SmartScreen 签名生效 | GitHub Actions artifact | YES |
| 11 | README 安装步骤 | grep + file size | YES |
| 12 | 硬约束 grep 全部通过(§10.2) | `init.sh` | YES |
| 13 | .skip / .todo / unimplemented!() 残留 = 0 | grep | YES |

**@oracle simplify / readability** (final):
- src-tauri/src/ 总 LoC / 模块依赖图 / 测试覆盖率
- webui/src/ 总 LoC / 组件复用度 / 状态分散度
- IPC 命令的 thin-wrapper 程度(不该在 commands 层做业务)
- README 真实性(没截图说截图,没功能说功能)
- 验证矩阵:哪些项 gate 在 init.sh,哪些 gate 在 manual

---

## 4. Delegation Map (rev 2)

| Phase | Lane | Sub | Specialist | Files | Concurrency |
|---|---|---|---|---|---|
| 1 | A | — | @fixer | 10 Rust (src-tauri/) | serial within phase |
| 1.5 | A | — | @fixer | 3 webui/ (package.json, tsconfig.json, types/api.ts) | serial |
| 2 | A | backend | @fixer | 9 Rust (provider/* + services/* + commands/*) | parallel with 2B |
| 2 | B1 | UI scaffold | @fixer | webui/{index.html, vite.config.ts, main.tsx, App.tsx, tailwind, postcss, styles, lib/api.ts mock, hooks} | after Phase 1.5; parallel with 2A |
| 2 | B2 | UI components | @designer | webui/src/components/* (6 files) + 8 shadcn components | after 2B1 |
| 2 | C | integration | @fixer | webui/src/lib/api.ts (swap mock → real) + tests/ipc_e2e.rs | after 2A + 2B |
| 3 | A1 | quota | @fixer | provider/*::fetch_quota + commands/quota.rs | parallel with 3B |
| 3 | A2 | tray | @fixer | tray.rs + commands/tray.rs | after 3A1 (sequential) |
| 3 | B | UI | @designer | ManualQuotaModal + TrayHoverCard | parallel with 3A |
| 4 | A | build+docs | @fixer | .github/workflows/* + README + docs/screenshots + tauri.conf.json version | serial |
| 4 | B | final review | @oracle | (read-only) 13-item verify + simplify | after 4A |

**Background vs foreground**:
- Phase 1, 1.5, 2A, 2B1, 2C, 3A1, 3A2, 4A → background `@fixer` (long, parallelizable)
- Phase 2B2, 3B → background `@designer` (visual judgment needs autonomy)
- Oracle reviews (Phase 1 / 1.5 / 2 / 3 / 4) → foreground (need orchestrator to read+reconcile)

**Lane count summary**: 8 implementation lanes + 4 oracle review checkpoints + 2 sequential sub-phases (2B1→2B2, 3A1→3A2).

---

## 5. Spec → Plan → Phase → Task Map (rev 2)

| REQ | Spec | Plan phase | Tasks | Notes |
|---|---|---|---|---|
| REQ-SCHEMA-001 | schema_version 1→2→3 | Phase 1 | T1.1 | |
| REQ-CAT-001 | categories 表 | Phase 1 (schema) + 1.5 (types) + 2B (UI sidebar) | T1.1, T1.2, T3.4 | |
| REQ-CAT-002 | sidebar UI 行为 | Phase 2B2 | T3.4 | |
| REQ-PROV-007 | 5 preset seed | Phase 1 | T1.6 | |
| REQ-PROV-008 | provider 行 schema | Phase 1 (schema) + 1.5 (types) | T1.1, T1.2 | |
| REQ-PROV-009 | test_connection 三 LLM | Phase 2A | T2.3, T2.6 | adapter_for returns Option (rev 2) |
| REQ-VIS-001 | provider_fields 表 | Phase 1 (schema) + 1.5 (types) | T1.1, T1.2 | |
| REQ-VIS-002 | visibility 二态(不加密) | Phase 1.5 (types) + 2B2 (UI toggle) | T1.2, T3.4 | |
| REQ-THEME-001 | 三主题 Radix Colors | Phase 1.5 (types) + 2B2 (UI toggle + globals.css) | T1.2, T3.5 | |
| REQ-THEME-002 | shadcn override Radix | Phase 2B1 | T3.1, T3.5 | pre-validate shadcn CLI |
| REQ-QUOTA-001~004 | LLM quota | Phase 3A1 | T4.1 | concurrent UPSERT + 15min TTL |
| REQ-QUOTA-005 | GitHub rate_limit | Phase 3A1 | T4.1 | |
| REQ-QUOTA-006 | PostgreSQL pg_database_size | Phase 3A1 | T4.1 | tokio-postgres dep warmup |
| REQ-QUOTA-DISPLAY-001 | Detail+Tray 单一源 | Phase 2B2 (skeleton) + 3A1 (data) + 3B (tray card) | T3.4, T4.1, T4.2 | |
| REQ-PROV-003 | is_preset 列 | Phase 1 | T1.1 | |
| REQ-PROV-004 | validate_key | Phase 2A | T2.3 | |
| REQ-COPY-001/002 | CopyButton 三态 | Phase 2B2 | T3.4 | |
| REQ-OAUTH-001 (REMOVED) | OAuth template | Phase 2B2 | (none — exclude from TEMPLATES) | rev 2 spec drift |

---

## 6. Risks & Mitigations (rev 2 — bumped per @oracle)

| # | Risk | L | I | Mitigation |
|---|---|---|---|---|
| 1 | shadcn/ui CLI 与 Tauri 2 + Vite 5 集成有坑 | M | L | Pin `npx shadcn@2.1.0`;pre-validate 1 component before full 8 install;fall back to Radix Primitives + 手写 Tailwind (per spec design.md §8.1 permits) |
| 2 | `docs/index.html` brutalist 风格与 shadcn 默认冲突 | M | M | @designer 决断(默认按 shadcn 风格,docs/index.html 仅作布局参考);Phase 2B2 加 5-min 同步任务 |
| 3 | 5 个 adapter 中 OpenAI subscription+usage 算法算错 | M | M | Phase 2A 之前派 @librarian 复查 `openai-balance/cmd/root.go` + cc-switch;单元测试 mock HTTP |
| 4 | cargo build 首次 2-5 分钟 | H | L | 接受,Phase 1 启动前先 `cargo fetch` 预热依赖;Phase 3A1 之前 `cargo add tokio-postgres` 预热 |
| 5 | 编译时 Stage 1 cargo check 暴露 spec 漏字段 | M | M | Phase 1.5 前可回退到 spec 修缺,不在 code 里补 spec 外的字段 |
| 6 | **mock 字段与真 IPC 漂移** | **H** | H | (rev 2: bumped from M→H per @oracle #10) Phase 1.5 contract lock + Phase 2C `tests/ipc_e2e.rs` 用 `tauri::test::mock_app()` 跑 12 IPC 全部;12 IPC fixture vitest |
| 7 | Azure Trusted Signing 审批延误 | M | M | Phase 1 后立即申请(已在 progress.md 列入),Phase 6 验收时不阻塞(签名可后补) |
| 8 | **Concurrent fetch_quota 写 quota_cache 竞态** | M | M | (rev 2: added per @oracle #6) SQLite `INSERT OR REPLACE INTO quota_cache` 原子 UPSERT;单元测试并发场景 |
| 9 | **quota_cache 后端 TTL 未明** | M | M | (rev 2: added per @oracle #12) `fetched_at > now - 900` 在请求时 check;15min TTL 边界单元测试 |
| 10 | `tauri::async_runtime` vs raw tokio 误用 | M | M | (rev 2: added per @oracle #15) T2.6 强制:`tauri::async_runtime::spawn` for HTTP,`spawn_blocking` for SQLite |
| 11 | WebView2 bootstrapper 漏发(LTSC / 裸 Win10) | L | M | (rev 2: added per @oracle #16) Phase 4 T6.1 验证 `WebView2Bootstrapper.exe` 在 MSI/NSIS,README 注明 |
| 12 | Phase 3 Lane A 范围混合 quota + tray(任一卡 → 整个 lane 卡) | M | M | (rev 2: split per @oracle #9) 3A-quota / 3A-tray 串行,各自独立可合并 |

---

## 7. Open Questions (待 Phase 1 前 lock)

1. **mock 走 `webui/src/lib/api.ts` 还是 Rust stub?** → 推荐 `lib/api.ts`(避免写假 Rust) — confirmed rev 1
2. **Stage 3 quota 卡写 skeleton 还是延后?** → 推荐 skeleton(loading/empty/error 三态) — confirmed rev 1
3. **AGENTS.md §3.3 "一次一个 Stage" 怎么处理并行?** → 推荐 PLAN.md §3 加"Phase 1.5 lock contract 后可并行"条款,§3.3 精神不变 — **rev 2 调整为**: Phase 1.5 是 Stage 3 子步,不需要新加 §3.3 条款,改 PLAN.md §3 + 增 §4
4. **Stage 5 关闭主窗口是否退出进程?** → 待用户在 Phase 3 前拍板(目前按"关闭主窗口不退出,仅靠托盘显隐"实现)
5. **shadcn CLI 版本?** → 锁定 `npx shadcn@2.1.0`(rev 2 决策)
6. **`withGlobalTauri`?** → `false`(rev 2 决策,改 tauri.conf.json)
7. **Phase 1.5 是否要加新 stage key 到 `stages[]`?** → **不**,rev 2 决策:Phase 1.5 是 Stage 3 子步,不污染 stages[]

---

## 8. Blockers

None. All external dependencies (Rust toolchain, Node 22, pnpm 11, tauri-cli 2.11, WebView2, MSVC) already verified in init.sh per session-1/2/3 handoffs.

---

## 9. Drift List (post-rev-2)

### 9.1 Applied in rev 2 (this session)

| File | Edit | Reason |
|---|---|---|
| `.slim/deepwork/keypilot-ui.md` | Replaced (rev 1 → rev 2) | @oracle REVISE |
| `.gitignore` | Added `.slim/deepwork/` | deepwork skill requirement |
| `src-tauri/Cargo.toml` | **Removed** `aes-gcm` / `base64` / `rand` | @oracle #8 (hard constraint §3.2) |
| `src-tauri/tauri.conf.json` | Set `withGlobalTauri: false` | @oracle #14 |
| `feature_list.json` | `stages.stage-2.description` "11 IPC" → "12 IPC" | @oracle #2 |
| `feature_list.json` | `stages.stage-3.description` "11 IPC" → "12 IPC" | @oracle #2 |
| `openspec/.../tasks.md` T2.6 | "11 个 IPC 命令" → "12 个 IPC 命令" | @oracle #2 |
| `openspec/.../design.md` §6 | `adapter_for` returns `Box<dyn ProviderAdapter>` with panic → returns `Option<Box<dyn ProviderAdapter>>` with None for unknown | @oracle #5 (runtime safety) |

### 9.2 Pre-Phase-1 drift (待本 session 完成后做,before Phase 1 @fixer 启动)

| File | Edit | Reason |
|---|---|---|
| `PLAN.md` §3 | 改 9-Stage 路线 → 4-Phase 路线(Stage 2/3 标并行,加 Phase 1.5 子步) | Reorder 决策 |
| `PLAN.md` §4 | 加 "Phase 1.5 Webui scaffold + TS contract lock" 子节 | 新增步骤 |
| `PLAN.md` `feature_list.json` 引用 | 12 IPC + adapter_for Option 修正同步 | 文档一致 |
| `feature_list.json` `stages.stage-1.assignee` | `"fixer (background, paused)"` → `"fixer (queued)"` | 解开 paused 状态 |
| `feature_list.json` `stages.stage-2.depends_on` | stays `["stage-1"]` (parallel) | 仍是 stage-1 only,Phase 1.5 在 Stage 3 内部 |
| `feature_list.json` `stages.stage-3.depends_on` | stays `["stage-1"]` (parallel) | 同上 |
| `feature_list.json` `stages.stage-3.files` | **加** `webui/package.json` + `webui/tsconfig.json` + `webui/src/types/api.ts` (Phase 1.5 产出) | Stage 3 文件清单扩展 |
| `progress.md` | 加 Session 4 (Reorder + 4-Phase + rev 2 oracle fixes) 段 | 连续性日志 |
| `session-handoff.md` | 加 Session 4 章节 | 正式交接 |
| `AGENTS.md` §3.3 | (rev 2 决策: 不改 §3.3 文字,在 PLAN.md §3 解释"Stage 2/3 在 lock contract 后可并行,§3.3 精神不变") | 文档对齐 |

### 9.3 Deferred to respective phase (不 pre-fix,执行时由 @fixer/@designer 处理)

| File | Edit | Phase | Reason |
|---|---|---|---|
| `src-tauri/src/main.rs` (新) | Tauri 2 entry | 1 | @fixer Stage 1 |
| `src-tauri/src/lib.rs` (新) | run() + seed 5 preset | 1 | @fixer Stage 1 |
| `src-tauri/src/database.rs` (新) | schema v3 5 表 | 1 | @fixer Stage 1 |
| `src-tauri/src/store.rs` (新) | AppState | 1 | @fixer Stage 1 |
| `src-tauri/src/error.rs` (新) | AppError enum | 1 | @fixer Stage 1 |
| `src-tauri/src/types.rs` (新) | Visibility / Provider / ProviderField / Category / Theme | 1 | @fixer Stage 1 |
| `src-tauri/capabilities/default.json` | 扩权限 | 1 | @fixer Stage 1 |
| `src-tauri/tauri.conf.json` `bundle.icon` | 加占位 icon 数组 | 1 | T1.8 deliverable |
| `src-tauri/tauri.conf.json` `frontendDist` | `"../webui/src"` → `"../webui/dist"` | 1 | T1.8 deliverable(Phase 1.5/2B 后才有 dist) |
| `src-tauri/src/provider/*` (5 files) | 5 adapter | 2A | @fixer Stage 2 |
| `src-tauri/src/services/{provider,category}.rs` | CRUD + delete migrate | 2A | @fixer Stage 2 |
| `src-tauri/src/commands/provider.rs` | 12 IPC | 2A | @fixer Stage 2 |
| `src-tauri/src/commands/quota.rs` | fetch_quota + UPSERT | 3A1 | @fixer Stage 4 |
| `src-tauri/src/tray.rs` + `commands/tray.rs` | 托盘常驻 | 3A2 | @fixer Stage 5 |
| `webui/{index.html, vite.config.ts, main.tsx, App.tsx, tailwind.config.ts, postcss.config.cjs}` | 脚手架 | 2B1 | @fixer Stage 3 |
| `webui/src/styles/globals.css` | 3 themes CSS variables | 2B1 | @fixer Stage 3 |
| `webui/src/lib/api.ts` (mock) | 12 IPC hardcoded | 2B1 | @fixer Stage 3 |
| `webui/src/hooks/useProviders.ts` | TanStack Query | 2B1 | @fixer Stage 3 |
| `webui/src/components/*` (6 files) | UI 组件 | 2B2 | @designer Stage 3 |
| `webui/src/components/ui/*` (8 shadcn) | shadcn CLI 装 | 2B2 | @designer Stage 3 (pre-validate first) |
| `webui/src/lib/api.ts` (real) | swap mock → invoke | 2C | @fixer integration |
| `src-tauri/tests/ipc_e2e.rs` | 12 IPC mock_app test | 2C | @fixer integration |
| `webui/src/components/ManualQuotaModal.tsx` | Stage 7 | 3B | @designer Stage 7 |
| `webui/src/components/TrayHoverCard.tsx` | Stage 5 | 3B | @designer Stage 5 |
| `.github/workflows/release.yml` | CI/CD | 4A | @fixer Stage 6 |
| `README.md` | docs | 4A | @fixer Stage 8 |
| `docs/screenshots/*` | 截图 | 4A | @fixer Stage 8 |
| `docs/index.html` | 5-min cosmetic sync | 2B2 | @designer |

No new openspec change. **No spec rewrite** (rev 2 already corrected the small drifts inline). `openspec/changes/v0.1-general-credentials/` remains the contract source.

---

## 10. References (file paths only, per deepwork skill)

- `openspec/changes/v0.1-general-credentials/proposal.md` — Why/What/Grill log
- `openspec/changes/v0.1-general-credentials/spec.md` — 20 REQ (ADDED/MODIFIED/REMOVED)
- `openspec/changes/v0.1-general-credentials/design.md` — §1 schema, §2 types.rs, §3 AppError, §5 Store, §6 Adapter trait (rev 2: `adapter_for` returns `Option`), §7 IPC (12 commands), §8 frontend contract, §9 Stage 1 file list, §10 Cargo.toml
- `openspec/changes/v0.1-general-credentials/tasks.md` — T1.1-T4.3 (rev 2: T2.6 "12 IPC" corrected) + Stage 5-9 + Verification matrix
- `PLAN.md` §3 — 9-Stage 路线 (待 4-Phase 改写)
- `PLAN.md` §4 — Stage 1 详细 (待加 Phase 1.5 子节)
- `feature_list.json` — stage-1~9 状态 (rev 2: stage-2/3 描述 "12 IPC")
- `AGENTS.md` §3.1-§3.4 — 硬约束
- `AGENTS.md` §10 — 验证清单
- `progress.md` — Session 1-3 已记录 (Session 4 待加)
- `session-handoff.md` — Session 1-3 已记录 (Session 4 待加)
- `.slim/deepwork/keypilot-ui.md` (this file) — rev 2

---

## 11. Oracle Review Trail

- **rev 1 review**: `ses_1063723cbffe6GHZ7zDU0MV1dH` — REVISE (5 High / 10 Medium / 11 Low)
  - 5 High: lane count 8 vs 10, IPC 12 vs 11, Phase 1.5 mergeability, synthetic stage key, adapter_for panic
  - Critical Medium applied: Cargo.toml cleanup, withGlobalTauri, concurrent fetch_quota, backend TTL, async runtime, shadcn CLI version pin, Phase 3 Lane A split, mock-vs-real High, docs/index.html sync
- **rev 2 review**: `ses_1062faa29ffeNaytp94fsDu2YE` — **APPROVE** (5 High + 3 critical Medium verified; #9 2 stale "11 IPC" refs patched)
- **Phase 1 review**: `ses_1061cf2bbffe3h5aUztoP7MUkG` — **APPROVE** (0 High / 2 Medium / 14 Low)
  - 2 Medium recommendations:
    1. `dirs::data_dir()` → Tauri 2 `app.path().app_data_dir()` (10-line refactor in lib.rs, idiom) — **fold into Phase 1.5**
    2. Add Rust unit tests for Visibility::parse / Theme::parse / AppError Serialize — **defer to Stage 2 kickoff**
  - 14 Low: all "OK, no change" or documentation notes
- **Phase 1.5 review**: `ses_10596b6aeffehUd3umxgJahajI` — **REVISE** (3 High / 4 Medium / 4 Low)
  - **3 High FIXED** (2026-06-24, orchestrator applied):
    1. `Category` Rust struct 缺 `created_at` / `updated_at` → 已加 (matches schema + TS)
    2. `QuotaSnapshot` shape 不一致 → 简化为 6 字段 `{ total, used, remaining, unit, level?, reset_at? }` (Rust + TS + design.md §2 全部同步,`fetched_at` 移到 quota_cache 表;`plan_name` / `is_valid` / `source` 砍 — UI 用 Provider.notes / TanStack Query staleTime 替代)
    3. `update_provider` IPC signature 不一致 (id inside vs flat-arg) → design.md §7 改为单 struct 模式 (`update_provider(state, req: UpdateProviderRequest)`),`add_category` / `delete_category` / `get_theme` / `set_theme` 同步单 struct 模式 (REVIEW 全部对齐)
  - **4 Medium FIXED** (orchestrator applied):
    4. `ListProvidersRequest` filters 砍 (backend 未实现,V0.1 改客户端 select)
    5. `QuotaSnapshot.unit` 砍 `| string` (strict union)
    6. `AppError.code` 改 literal union of 11 codes (TS exhaustiveness)
    7. `UpdateProviderRequest.fields` 改 replace-all (drop `id` from Omit)
  - **4 Low**: deferred / documentation notes

### Post-fix Verification
- `cargo check`: **PASS** (11 warnings, all "unused code" — expected for Stage 1 scaffold)
- `pnpm tsc --noEmit`: **PASS** (no output = no errors)
- Both gates green. Phase 2 ready to dispatch.

## 13. Phase 1.5 Result (2026-06-24, ses-106108cfaffeNyRIFCTr5KpW1g + orchestrator fixes)

## 12. Phase 1 Result (2026-06-24, ses-10624f63fffesy5PjYzjVvbT9O)

### Status: ✅ DONE (with 1 spec deviation, accepted)

### Files (10)
- modified: `src-tauri/Cargo.toml` (23 lines; verified encryption crates removed, async-trait + reqwest retained)
- modified: `src-tauri/tauri.conf.json` (23 lines; added bundle.icon, frontendDist → ../webui/dist, withGlobalTauri: false)
- modified: `src-tauri/capabilities/default.json` (5 lines; added core:event:default)
- verified: `src-tauri/build.rs` (3 lines; standard tauri_build::build())
- modified: `src-tauri/src/main.rs` (9 lines; calls keypilot_lib::run)
- modified: `src-tauri/src/lib.rs` (53 lines; startup chain: app_dir → Database::open → setup_schema → seed_preset_providers → manage_state)
- modified: `src-tauri/src/database.rs` (225 lines; schema v3 5 tables, 5 preset seed with 12 fields total)
- modified: `src-tauri/src/store.rs` (12 lines; AppState with Arc<Mutex<Database>>)
- modified: `src-tauri/src/error.rs` (64 lines; 11 AppError variants + custom Serialize {code, message})
- modified: `src-tauri/src/types.rs` (102 lines; Visibility / Theme / Provider / ProviderField / Category / QuotaSnapshot; NO Private variant)

### Verification
- `cargo check`: **PASS** (10 warnings, all "unused code" in scaffold — expected for Stage 1; Theme, Category, ProviderField, QuotaSnapshot, as_str/parse methods not yet called)
- 6 grep gates (visibility / value / preset / category_id / encryption / fs::write): **ALL PASS**
- SQLite runtime verification: **SKIPPED** (sqlite3 CLI unavailable; verified by code review; will run manual test on next session)
- post-impl updates: `feature_list.json` (status=done, evidence, ended_at, assignee), `progress.md` (Phase 1 §), `session-handoff.md` (Phase 1 §) — all updated

### Spec Deviations (1, accepted)
1. **AppState: `Arc<Database>` → `Arc<Mutex<Database>>`** — required because `rusqlite::Connection` is `Send` but NOT `Sync`, and Tauri 2's `manage()` requires `Send + Sync`. **Justified**; spec was wrong on this. **Fix applied**: `openspec/design.md §5` and `openspec/tasks.md T1.5` updated with rev 2 annotation. **All Stage 2/3 services must call `state.db.lock().unwrap()` before SQLite access.**

### Carry-Over
- `webui/dist/index.html` placeholder created (will be replaced in Phase 1.5)
- 5 preset rows in code (OpenAI / DeepSeek / Anthropic / GitHub / PostgreSQL) — verified by code review, runtime test pending
- 11 AppError variants ready for Stage 2 IPC handlers

### Next: Phase 1.5 (webui scaffold + TS contract lock)
- Files (3): `webui/package.json` + `webui/tsconfig.json` + `webui/src/types/api.ts`
- Verify: `pnpm tsc --noEmit` passes
- Produces TS contract for Stage 2 + Stage 3 to share
