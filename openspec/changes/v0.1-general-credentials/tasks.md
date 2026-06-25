# V0.1 General Credentials — Tasks

> **目标**: 把 `spec.md` 14 REQ + `design.md` 落到 Stage 1-4 实施任务。
> **格式**: T<Stage>-<NN> 编号,每个 task 含验收命令 + 引用 REQ。

---

## Stage 1 — Schema 重写 + 字段加密 + 5 preset seed

> **优先级**: P0(阻塞 Stage 2-9)。Stage 1 之前的 in-progress 状态视为"作废",本 tasks.md 是新的真相源。

### T1.1 重写 `src-tauri/src/database.rs` — schema v3

- DROP + 重建:`meta` / `categories` / `providers` / `provider_fields` / `quota_cache`
- 索引:`idx_providers_category` / `idx_providers_preset` / `idx_pf_provider`
- 初始 seed:meta(3 行) + categories(1 行 `凭证`)
- 引用:`REQ-SCHEMA-001 MODIFIED` + `REQ-CAT-001` + `REQ-PROV-008` + `REQ-VIS-001`
- 验证:`sqlite3 "%APPDATA%\com.keypilot.app\keypilot.db" ".tables"` 返回 5 张表
- 验证:`SELECT * FROM meta WHERE key='schema_version';` 返回 `'3'`

### T1.2 新建 `src-tauri/src/types.rs`

- `Visibility` enum(二态 `Visible` / `Masked`,无 `Private`) + parse/as_str
- `Provider` / `ProviderField` / `Category` / `Theme` struct,全部 `Serialize + Deserialize`
- 引用:`REQ-VIS-001` + `REQ-CAT-001` + `REQ-PROV-008` + `REQ-THEME-001`

### T1.3 ~~新建 `src-tauri/src/crypto.rs`~~ — **V0.1 不实现(用户决策"不加密")**

- V0.1 全部字段明文,不引入 crypto 模块
- 推迟到 V0.2 RFC(SQLCipher 全文件 / 主密码 + argon2id / Windows DPAPI 三选一)

### T1.4 重写 `src-tauri/src/error.rs`

- `AppError` enum 11 分支(Database / Io / Serde / InvalidVisibility / InvalidTheme / ProviderNotFound / CategoryNotFound / CategoryIsDefault / ProviderCannotTest / ProviderQuotaUnsupported / Http)— **去掉 Encryption 分支**
- `Serialize for AppError` 输出 `{ code, message }`(Tauri IPC 用)
- 引用:`design.md §3`

### T1.5 重写 `src-tauri/src/store.rs`

- `AppState { db: Arc<Mutex<Database>> }` (rev 2 修正:Send+Sync 要求,原 `Arc<Database>` 不编译)
- 引用:`design.md §5`

### T1.6 重写 `src-tauri/src/lib.rs`

- `run()` 启动链:
  1. `app.path().app_data_dir()` → `%APPDATA%\com.keypilot.app\`
  2. `Database::open(path)` → `setup_db()`
  3. `seed_preset_providers()` — 检查 `meta.preset_seeded`,未 seed 则 INSERT 5 行 + 各自 fields
  4. `setup_window()` — 创建主窗口,label = "main",从 `meta.theme` 读初值
  5. `manage_state(AppState::new(db))`
- 引用:`REQ-PROV-007` + `REQ-THEME-001` + `REQ-CAT-001`

### T1.7 更新 `src-tauri/Cargo.toml`

- 新增:`async-trait = "0.1"`(Stage 2 提前引入 ProviderAdapter trait)
- **不引入**:`aes-gcm` / `base64` / `rand`(V0.1 不加密)
- 验证:`grep -E "^aes-gcm|^argon2|^chacha20|ChaCha20Poly1305" src-tauri/Cargo.toml` 必须**全部空**(V0.1 无加密 crate)

### T1.8 修复 `src-tauri/tauri.conf.json` 3 处配置

- `icon`:留空数组 `[]`(Stage 6 补真图标)
- `frontendDist`:改 `"../webui/src"`(Stage 3 再改 `dist`)
- `app.windows[0].label`:补 `"main"`

### T1.9 更新 `src-tauri/capabilities/default.json`

- 确保 `"windows": ["main"]` 与 tauri.conf.json label 一致
- permissions 暂只 `core:default`(Stage 3 加 clipboard,Stage 5 加 tray)

### T1.10 Stage 1 验收(走 `./init.sh`)

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` 通过
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` 通过(无 crypto roundtrip,V0.1 不加密)
- [ ] `cargo build --manifest-path src-tauri/Cargo.toml` 通过(首次编译 ~1-2 分钟)
- [ ] `grep "visibility TEXT NOT NULL DEFAULT 'visible'" src-tauri/src/database.rs` 有结果
- [ ] `grep -E "^aes-gcm|^argon2|^chacha20|ChaCha20Poly1305" src-tauri/Cargo.toml` 必须**全部空**(V0.1 不加密)
- [ ] `grep -rn "fs::write" src-tauri/src/` 出现的所有路径不在 CLI 配置白名单外(§3.1 硬约束)
- [ ] 启动应用,退出,SQLite 文件落在 `%APPDATA%\com.keypilot.app\keypilot.db`
- [ ] `sqlite3 "%APPDATA%\com.keypilot.app\keypilot.db" ".tables"` 返回 5 张表(meta/categories/providers/provider_fields/quota_cache)
- [ ] `SELECT * FROM categories;` 返回 1 行(id=1, name='凭证', is_default=1)
- [ ] `SELECT * FROM providers;` 返回 5 行(OpenAI/DeepSeek/Anthropic/GitHub/PostgreSQL)
- [ ] `SELECT key, value FROM meta WHERE key IN ('schema_version','preset_seeded','theme');` 返回 schema_version=3, preset_seeded=1, theme=auto
- [ ] `meta.preset_seeded=1` 后再启动一次,`SELECT count(*) FROM providers WHERE is_preset=1;` 仍是 5(不重建)
- [ ] `SELECT key, visibility FROM provider_fields WHERE provider_id IN (SELECT id FROM providers WHERE preset='openai');` 5 行 preset seed,API key / token / password 字段 visibility='masked',其他 visibility='visible'

### T1.11 更新 `feature_list.json` + `progress.md` + `session-handoff.md`

- `feature_list.json` stage-1 status = "done" + evidence 字段填验证命令输出
- `progress.md` 加本 session 段
- `session-handoff.md` 加 v0.1-general-credentials change 引用

### T1.12 同步 `docs/index.html` 到新设计 — **推迟**(docs/index.html 是布局参考,color 不锁定,Stage 3 实施时直接接 Radix Colors 重新写)

- preset 列表:DeepSeek / GitHub / PostgreSQL → OpenAI / DeepSeek / Anthropic / GitHub / PostgreSQL(布局参考足够,不锁)
- 模板下拉:4 项 → 3 项(布局参考)
- 状态栏文案:"本地 SQLite · 字段级加密 opt-in" → "本地 SQLite" (V0.1 不加密)
- footer 版本:`v 0.2.0` → `v 0.1.0 · build 2026.06.24`
- 不实现 visibility 三态视觉(V0.1 二态,改 UI 时直跳)
- **说明**:docs/index.html 仅作布局参考,Stage 3 实施时由 shadcn + Radix Colors 直接重写,不再同步中间状态

---

## Stage 2 — Provider model + 5 adapter + CRUD IPC

### T2.1 新建 `src-tauri/src/provider/mod.rs`

- `pub mod adapter;` + 6 个 adapter module
- `pub fn adapter_for(preset: &str) -> Option<Box<dyn ProviderAdapter>>` — preset=NULL 返回 None(走 Custom 路径)

### T2.2 新建 `src-tauri/src/provider/adapter.rs`

- `ProviderAdapter` trait(沿用 `design.md §6`)
- `adapter_for` factory
- 引用:`REQ-PROV-009` + `REQ-QUOTA-004`

### T2.3 新建 5 个 adapter module

- `openai.rs` — `test_connection: GET /v1/models`(REQ-PROV-004),`fetch_quota: subscription + usage`(REQ-QUOTA-001)
- `deepseek.rs` — `test_connection: GET /user/balance`,`fetch_quota: /user/balance`(REQ-QUOTA-002)
- `anthropic.rs` — `test_connection: POST /v1/messages max_tokens=1`,`fetch_quota: Unsupported`(REQ-QUOTA-003)
- `github.rs` — `test_connection: N/A(can_test=false)`,`fetch_quota: GET /rate_limit`(REQ-QUOTA-005)
- `postgres.rs` — `test_connection: N/A(can_test=false)`,`fetch_quota: pg_database_size`(REQ-QUOTA-006)
- 引用:`REQ-PROV-004` + `REQ-PROV-009` + `REQ-QUOTA-001/002/003/004/005/006`

### T2.4 新建 `src-tauri/src/services/provider.rs`

- `list_providers(db)` / `get_provider(db, id)` / `add_provider(db, req)` / `update_provider(db, id, req)` / `delete_provider(db, id)`
- 所有 SQL 操作 `Arc<Database>::conn()` 包装
- visibility 只有 visible / masked 二态,value 始终明文(V0.1 不加密)

### T2.5 新建 `src-tauri/src/services/category.rs`

- `list_categories(db)` / `add_category(db, name)` / `delete_category(db, id, migrate_to)`
- `delete_category`:若 `is_default=1` → `AppError::CategoryIsDefault`;若有 provider → 强制 `migrate_to` 必填(UPDATE providers.category_id 后才删)

### T2.6 新建 `src-tauri/src/commands/provider.rs`

- **12 个 IPC 命令**(沿用 `design.md §7`,rev 2 修正)
- `test_connection` + `fetch_quota` 走 `adapter_for(preset) -> Option<Box<dyn ProviderAdapter>>`(rev 2: 返回 `Option` 取代 panic,unknown/Custom 返回 None),Custom(None) → `AppError::ProviderCannotTest` / `AppError::ProviderQuotaUnsupported`
- `get_theme` + `set_theme` 读写 `meta.theme`
- Async runtime (rev 2): HTTP 调用走 `tauri::async_runtime::spawn`;SQLite 调用走 `tauri::async_runtime::spawn_blocking`

### T2.7 Stage 2 验收

- [ ] `cargo check` / `cargo test` 通过
- [ ] 单元测试:test_connection 4 类响应(200/401/403/timeout)
- [ ] 集成测试:`add_provider` → `list_providers` → `delete_provider` 端到端
- [ ] `grep -rn "fs::write" src-tauri/src/` 路径白名单合规
- [ ] visibility=masked 的字段 add 后 SQL 直接查 `value` 是明文(UI 渲染时按 visibility 决定显示原文/掩码)

---

## Stage 3 — UI 主窗口 + 3 theme + Category sidebar

### T3.1 新建 `webui/` Vite + React + TS

- `pnpm create vite webui --template react-ts`
- 安装:`@tanstack/react-query` + `@tauri-apps/api` + `react-router-dom`
- 引用:`feature_list.json` stage-3 文件清单

### T3.2 新建 `webui/src/types/api.ts`

- 沿用 `design.md §8`(Visibility / Theme / Provider / ProviderField / Category / AppError)

### T3.3 新建 `webui/src/lib/api.ts`

- `invoke<T>(cmd, args)` 包装 `@tauri-apps/api/core::invoke`
- **12 个 IPC 函数**(listProviders / getProvider / addProvider / updateProvider / deleteProvider / listCategories / addCategory / deleteCategory / testConnection / fetchQuota / getTheme / setTheme) (rev 2 修正)
- 错误统一 toast 展示(AppError.message)

### T3.4 新建 `webui/src/components/ProviderList.tsx` + `ProviderForm.tsx` + `CopyButton.tsx`

- ProviderList:sidebar 展示 categories + 折叠/展开 + 5 preset 徽章(📌 官方)
- ProviderForm:detail 区域 + KV 列表(visibility 三态视觉) + actions(测试连通性/重命名/删除)
- CopyButton:`visibility=visible` 直接复制 / `masked` 点 ◉ 切换明文后再复制明文
- 引用:`REQ-CAT-002` + `REQ-COPY-003`

### T3.5 新建 `webui/src/components/ThemeToggle.tsx` + 3 theme CSS 变量

- `:root[data-theme='dark']` / `:root[data-theme='light']` 两套 CSS 变量
- 启动时读 meta.theme,auto 模式监听 `prefers-color-scheme`
- titlebar 加切换按钮(Dark ↔ Light),auto 模式隐藏按钮
- 引用:`REQ-THEME-001`

### T3.6 模板下拉砍 OAuth

- `webui/src/lib/templates.ts` — TEMPLATES 只 `blank` / `llm` / `database` 3 项
- 引用:`REQ-OAUTH-001 REMOVED`

### T3.7 Stage 3 验收

- [ ] `cd webui && pnpm tsc --noEmit` 通过
- [ ] `cd webui && pnpm build` 通过
- [ ] 浏览器(开发模式)打开 `docs/index.html` 与 `webui/` 实际渲染对比,layout/spacing/typography 误差 < 5px
- [ ] 切换 dark / light / auto,主窗口背景色实时变化
- [ ] visibility 二态切换 + 复制按钮(visible 直接复制 / masked 需点 ◉ 切换明文)
- [ ] sidebar 折叠/展开,新增/删除 category(default category 删按钮 hidden)

---

## Stage 4 — fetch_quota 全实现(沿用 Stage 2)+ 集成

### T4.1 实装 `provider/{openai,deepseek,anthropic,github,postgres}.rs::fetch_quota`

- 沿用 `REQ-QUOTA-001/002/003/005/006` 算法
- 引用:`openspec/changes/v0.1-spec-alignment/spec.md REQ-QUOTA-001/002/003`

### T4.2 新建 `src-tauri/src/commands/quota.rs`

- `fetch_quota(state, id: i64) -> Result<QuotaSnapshot, AppError>`
- 写 `quota_cache` 表(15min TTL,前端 staleTime)

### T4.3 Stage 4 验收

- [ ] 单元测试:5 preset 各 1 个 happy path + 1 个 auth fail(401)
- [ ] Anthropic fetch_quota 返回 QuotaError::Unsupported(UI 走手动输入路径)
- [ ] GitHub / Postgres quota 列在 UI 显示(REQ-QUOTA-005/006 实际值)

---

## Stage 5-9

沿用 `feature_list.json` + `PLAN.md §3`,Stage 5 托盘 / Stage 6 打包签名 / Stage 7 Anthropic 手动输入 / Stage 8 README / Stage 9 验收。本 change 不修改 Stage 5-9 文件清单(若 Stage 5 tray hover 卡 UI 需要显示 quota,沿用 Stage 4 fetch_quota 数据)。

---

## Verification matrix(完整)

| 维度 | 命令 | 期望 |
|---|---|---|
| Rust 编译 | `cargo check --manifest-path src-tauri/Cargo.toml` | exit 0 |
| Rust 测试 | `cargo test --manifest-path src-tauri/Cargo.toml` | exit 0, 单元测试全过(无 crypto roundtrip,V0.1 不加密) |
| TypeScript | `cd webui && pnpm tsc --noEmit` | exit 0(Stage 3+ 启用) |
| Schema 验证 | `sqlite3 "%APPDATA%\com.keypilot.app\keypilot.db" ".tables"` | 5 张表 |
| Seed 验证 | `sqlite3 ... "SELECT count(*) FROM providers WHERE is_preset=1;"` | 5 |
| Hard:CLI 路径 | `grep -rn "fs::write" src-tauri/src/` | 路径不在 `~/.claude/` / `~/.codex/` / `~/.config/opencode/` |
| Hard:加密 crate | `grep -E "^aes-gcm\|^argon2\|^chacha20\|ChaCha20Poly1305" src-tauri/Cargo.toml` | **空(V0.1 不加密)** |
| Hard:visibility 列 | `grep "visibility TEXT" src-tauri/src/database.rs` | 有(REQ-VIS-001) |
| Hard:visibility 二态 | `grep "visibility TEXT NOT NULL DEFAULT 'visible'" src-tauri/src/database.rs` | 有 |
| Hard:provider_fields | `grep "value TEXT" src-tauri/src/database.rs` | 有(REQ-VIS-001) |

---

## Risks

| 风险 | 缓解 |
|---|---|
| Stage 1 DROP + 重建 vs 已有数据兼容 | V0.1 未发布,DROP 安全。若 Stage 1 后有真实用户,需 RFC 评估 v2→v3 兼容迁移 |
| V0.1 不加密 = 字段裸奔 | 用户已知情(用户决策"不加密"),靠 Windows ACL + 用户密码保护;README 明示限制 |
| V0.2 加密 RFC 推迟 | 单独 session 评估 SQLCipher / 主密码 / DPAPI;V0.1 完成后立即启动 |
| PostgreSQL fetch_quota 需 tokio-postgres | Stage 4 加入 Cargo.toml,Stage 1-3 不影响 |
| Light theme 调色板未在本 change 锁定 | Stage 3 实施时拍板,本 change 不锁具体颜色(避免设计 doc 污染 spec) |
