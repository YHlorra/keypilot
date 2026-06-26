# UI Redesign (Kaku Design System) : Tasks

## T-UI-001: Token 替换

- [ ] `webui/src/styles/globals.css` : 替换为 Kaku light + dark 主题 (REQ-UI-001.1 / 001.3)
- [ ] `webui/tailwind.config.ts` : extend `fontFamily` / `spacing` / `borderRadius` / `colors` 引用 token (REQ-UI-001.2 / 001.3);**`darkMode: ["selector", "[data-theme='dark']"]`** (H-3)
- [ ] `webui/index.html` : 加载 Google Fonts Charter (REQ-UI-001.2)
- [ ] `webui/src/lib/utils.ts` : 不变 (shadcn `cn` 工具)
- [ ] `webui/src/components/ThemeToggle.tsx:17-28` : `documentElement.classList` 改 `setAttribute("data-theme", active)`,Auto 模式由 matchMedia 解析为 light/dark (H-3)

## T-UI-002: shadcn 组件落 Kaku 风格

- [ ] `webui/src/components/ui/button.tsx` : 圆角 pill (999px),字阶 label-lg,无 shadow
- [ ] `webui/src/components/ui/card.tsx` : 圆角 8px,1px 边框,无 shadow
- [ ] `webui/src/components/ui/input.tsx` : 圆角 8px,1px 边框,字阶 body-md
- [ ] `webui/src/components/ui/select.tsx` : 同 input
- [ ] `webui/src/components/ui/toast.tsx` : 边框 + 8px + 无 shadow
- [ ] 验证: `pnpm tsc --noEmit` PASS,`pnpm build` PASS

## T-UI-003: 标题栏与顶栏

- [ ] `webui/src/App.tsx` : 重写: 移除 sidebar,改用 Titlebar + TopBar + ProviderGrid + ProviderDetailModal
- [ ] `webui/src/components/Titlebar.tsx` (新) : serif "KeyPilot" + ThemeToggle + Settings 文字按钮
- [ ] `webui/src/components/TopBar.tsx` (新) : search + ChipGroup + DensityToggle
- [ ] `webui/src/components/ThemeToggle.tsx` : 行为不变,色值走 token
- [ ] 删 🔑 emoji 和 ⚙️ emoji,改 lucide-react SVG (KeyRound / Settings)

## T-UI-004: 卡片网格

- [ ] `webui/src/components/ProviderCard.tsx` (新) : drag handle + icon + name + URL + clock+time + refresh + quota
- [ ] `webui/src/components/ProviderGrid.tsx` (新) : 容器,根据 `<html data-density>` 切 1-col / 2-col
- [ ] `webui/src/components/SectionLabel.tsx` (新) : 复用,serif 12px tracked uppercase
- [ ] `webui/src/components/ChipGroup.tsx` (新) : 顶栏 category chip
- [ ] `webui/src/components/DensityToggle.tsx` (新) : 1-col / 2-col 切换器
- [ ] 删 `webui/src/components/CategorySidebar.tsx`
- [ ] 删 `webui/src/components/ProviderList.tsx`

## T-UI-005: Detail Modal (Modal.tsx 原地改造,不再创建 ModalShell)

- [ ] `webui/src/components/Modal.tsx` (重写) : 文件名不动,内部切 Radix Dialog;720px 居中 + 8px 圆角 + 1px 边框 + scrim `rgba(0,0,0,0.4)` + focus trap (H-1, decision D1)
- [ ] `webui/src/components/ProviderDetailModal.tsx` (新) : Header (含 trash icon + Edit/Cancel 切换 + 派生 status pill) + Quota section (内含 inline 2px progress bar,**不是 QuotaBadge**) + Credentials section (KV 列表 + mask + Eye 切换) + Footer (Cancel / Fetch quota / Test connection,左→右顺序钉死)
- [ ] `webui/src/components/AddCredentialModal.tsx` : 应用 Modal.tsx 新壳 (import 路径不变)
- [ ] `webui/src/components/AddKvModal.tsx` : 同上
- [ ] `webui/src/components/ManualQuotaModal.tsx` : 同上;**改第 113 行注释**;嵌套在 ProviderDetailModal 内打开 (Radix Dialog 自动 focus stack)
- [ ] `webui/src/components/SettingsModal.tsx` : 同上
- [ ] `webui/src/components/ConfirmDialog.tsx` : 同上
- [ ] Esc 关闭,点 overlay 关闭 (沿用 Radix Dialog)
- [ ] 删 `webui/src/components/ProviderDetail.tsx` (内容并入 `ProviderDetailModal`)

## T-UI-006: 细节组件

- [ ] `webui/src/components/KvRow.tsx` : key 用 `var(--color-primary)`,value 用 mono
- [ ] `webui/src/components/QuotaBadge.tsx` : 改用 Card 容器,**仅显示 quota 文本 + 数字 (无 progress bar)**;progress bar 在 `ProviderDetailModal` 内 inline 实现 (H-4)
- [ ] `webui/src/components/CopyButton.tsx` : 改 lucide `Copy` / `Eye` / `EyeOff` SVG
- [ ] `webui/src/components/Icon.tsx` : 移除 emoji,统一 lucide-react
- [ ] `webui/src/components/ErrorBoundary.tsx` : **untracked (1538B stub)** → 套 Kaku 风格 + 首次纳入 git 跟踪 (`git add`) 后 commit (M-7)
- [ ] `webui/src/components/TrayHoverCard.tsx` : 套 Card 风格 (system tray)

## T-UI-007: 主题持久化与状态

- [ ] `webui/src/hooks/useTheme.ts` : 验证 dark token 切换 (V0.1 已实现,本 change 验通)
- [ ] `webui/src/App.tsx` : 加 `density` state (localStorage `keypilot.density`,默认 `'1'`)
- [ ] `webui/src/App.tsx` : 加 `categoryFilter` state (default 'all'),加 `search` state (复用 stage-11 `useProviders` filter)
- [ ] `<html data-density="1|2">` 注入,驱动 grid 切列

## T-UI-008: 验证

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` PASS (本 change 不动后端,仅 sanity)
- [ ] `pnpm tsc --noEmit` PASS
- [ ] `pnpm build` PASS (build 产物更新)
- [ ] 浏览器视觉验证 (1280 / 768 / 375 三档)
- [ ] 主题切换 (Auto / Light / Dark) 三态都验
- [ ] 密度切换 (1-col / 2-col) 视觉验
- [ ] Detail Modal 打开 / 关闭 / Escape / overlay click 验
- [ ] Grep gates:
  - `box-shadow:` 除 `none` → 0
  - `linear-gradient` / `radial-gradient` → 0
  - `backdrop-filter` / `filter: blur` → 0
  - em-dash `—` → 0
  - 旧色 `#5b5bd6` / `#fcfcfc` → 0
- [ ] 视觉对比 `webui/dist/design-preview.html` v2,无明显 drift

## 验收标准 (Definition of Done)

- [ ] 所有 T-UI-001 .. T-UI-008 子项 [x]
- [ ] `pnpm build` 输出在 `webui/dist/` 更新
- [ ] `feature_list.json` 中 stage-12 status = done + evidence 记录验证命令输出
- [ ] 视觉与 `webui/dist/design-preview.html` v2 一致
- [ ] AGENTS.md §9 全部 grep gate 通过

## 关联

- `webui/src/types/api.ts` : V0.1 + V0.2 锁,本 change 不改
- `webui/src/hooks/useProviders.ts` : V0.2 search 复用 (REQ-UI-002.2)
- `webui/src/hooks/useTheme.ts` : V0.1 theme 持久化复用
- `DESIGN.md` : Kaku 设计系统真相源
- `webui/dist/design-preview.html` v2 : 视觉验证基准
