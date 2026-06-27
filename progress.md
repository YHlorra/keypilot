# Progress Log

> Per AGENTS.md §8 — Session 连续性日志。 每个 session 至少更新一次。
> 真相源: git log (commit 详情) + feature_list.json (feature 状态) + progress.md (session 进度)。

<!-- 2026-06-28 -->

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

<!-- 2026-06-27 之前的 session 记录由 git log 持有, 不在本文件重复 -->