# Progress Log

> Per AGENTS.md §8 — Session 连续性日志。 每个 session 至少更新一次。
> 真相源: git log (commit 详情) + feature_list.json (feature 状态) + progress.md (session 进度)。

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
- `docs/screenshots/*.png.txt` 是占位文件, 实际截图待补.
- `Cargo.toml` 显示 LF/CRLF 行尾告警, 无实际内容改动, 未进 commit.
- `docs/index.html` (1993 行) 是 UI 设计预览 HTML, 不是 markdown doc, 不在 neat-freak scope.

### 进行中 / 下一步

- **tray.png** 还是 595B 占位 (`cargo tauri icon` 不动它). 如果要让托盘图标和 app 图标协调, 需单独生成 16/24px 极简版本 (推荐: 同心环单元素, 去掉中心 dot).
- 关闭 `cargo tauri dev` 后台进程 (sprint 结束).

---

<!-- 2026-06-27 之前的 session 记录由 git log 持有, 不在本文件重复 -->