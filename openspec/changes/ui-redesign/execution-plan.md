# UI Redesign (Kaku Design System) : Execution Plan

> Stage-12 handoff plan. Design already approved (`feature_list.json` → `stage-12: planned`, notes 写"用户已批准设计")。
> 本文件是 implementation 阶段的执行手册,代码改动按 `tasks.md` T-UI-001..008 跑,本计划定义顺序、并行、owner、gate。
>
> **关联**: [`proposal.md`](./proposal.md) · [`spec.md`](./spec.md) · [`design.md`](./design.md) · [`tasks.md`](./tasks.md)

---

## 1. 状态

| 项 | 值 |
|---|---|
| Spec 状态 | approved |
| Stage | stage-12 (`feature_list.json` `status: planned`) |
| 依赖 | stage-1, stage-11 (均 done) |
| 文件改动总数 | 34 (含 ErrorBoundary.tsx untracked 首次入 git) |
| 新增依赖 | 0 (`lucide-react@0.460` + `@radix-ui/react-dialog@1.1.17` + `@radix-ui/react-dropdown-menu@2.1.18` 已在 `webui/package.json`) |
| 预计 build 体积 | +5KB CSS / +2KB JS (估算,待 stage-12 完成后实测) |
| 预计工时 | ~2.5h @fixer(并行)+ 15min 验收 |

---

## 2. Scope

### 2.1 Building

- Kaku token 替换(`globals.css` + `tailwind.config.ts` + `index.html` Charter)
- shadcn 组件 restyle(button/card/input/select/toast 全 pill 化,零 shadow)
- App Shell 重做:Titlebar (44px serif) + Top Bar (60px search/chips/density) + Card Grid (1-col/2-col density toggle) + Detail Modal
- 主题系统保持 3 态 (Auto/Light/Dark),dark 变体做 coherent inversion (warm near-black + lightened navy)
- 卡片右键 context menu 含 Delete(沿用 Radix DropdownMenu)
- Density state 持久化(`localStorage.keypilot.density`)
- 凭证 mask 显示用 `font-mono`,URL 用 link blue,quota 数字用 success green

### 2.2 Not Building

- 零新 IPC / 零新 schema / 零 Rust 改动
- 零新 npm 依赖
- 零 dnd 库接入(drag handle 仅 affordance)
- 零营销 / hero / landing 元素
- 零 macOS Liquid Glass 迁移
- 零加密
- 零 i18n
- 零移动端 left-swipe(Tauri Windows 应用,desktop pattern 不接)

---

## 3. 关键决策(已与用户确认)

| # | 决策 | 理由 |
|---|---|---|
| D1 | `Modal.tsx` **不重命名,原地改造**;不创建 `ModalShell.tsx` | 5 个调用方零 import 改动。`design.md` §3.2 + `tasks.md` T-UI-005 + `feature_list.json` stage-12 中 `ModalShell.tsx` 条目已删除 (H-1) |
| D2 | 删除走**右键 context menu** | Tauri WebView2 desktop 平台,left-swipe 是 mobile pattern 不接;Radix DropdownMenu 触发,内含 "Delete credential" 项,弹 ConfirmDialog 后调 `deleteProvider` IPC。`ProviderCard.onContextMenu` 内 `e.preventDefault()` 抑制 WebView2 原生菜单;T-UI-004 验收: dev 模式右键卡片确认 menu 弹出 (M-9) |
| D3 | `setManualQuota` 后端已实装,只清过期注释 | `src-tauri/src/commands/quota.rs:143` + `lib.rs:69` 注册真实 handler;`api.ts:82` 走 `invoke("set_manual_quota")`;`ManualQuotaModal.tsx:114` 实际已调 IPC。**预填逻辑在 `ManualQuotaModal.tsx:88-98` (useState initializer,保留)**;第 113 行注释从 "V0.1: localStorage-only, V0.1.1+: real IPC" 改为 "真实 IPC 调用,localStorage 仅用于预填" (M-2) |
| D4 | 卡片 Delete 走双入口 | 卡片右键 context menu(主入口,符合 Kaku 极简)+ Modal Header 右上 trash icon ghost button(次入口,行为对齐原 `ProviderDetail` Header 删除按钮) |
| D5 | density 持久化 key = `keypilot.density`,默认 `'1'` | 读时容错,无值 fallback `'1'` |
| D6 | Charter 走 Google Fonts CDN,fallback 链 Charter → Georgia → Palatino → Times New Roman | `display=swap` 不阻塞首屏;离线 / CDN 失败时 Georgia 保留编辑感 |
| D7 | `tailwindcss-animate` 保留 devDep | Stage-12 无组件直接使用,但 devDep 不删(其他库如 Radix Primitives 可能间接引用;若确认无人用,V0.3 清理) |
| D8 | shadcn 命名类保留,值改 Kaku token | 旧组件 className 不变,零 class 改名成本;tailwind `extend.colors` 一次性覆盖 |

---

## 4. Phases

| Phase | Tasks | 文件数 | 合并后状态 | 验证 |
|---|---|---|---|---|
| **Phase 1** Token + shadcn | T-UI-001 + T-UI-002 | 8 | 老布局(sidebar + detail pane)+ Kaku 主题色/字阶/圆角 | `pnpm tsc --noEmit` + `pnpm build` + 视觉比 v1 |
| **Phase 2** 布局 + 状态 | T-UI-003 + T-UI-004 + T-UI-005 + T-UI-007 | 18 | 卡片网格 + 详情 modal + density state | Phase 1 gates + 多数 grep gate |
| **Phase 3** 细节 + 验收 | T-UI-006 + T-UI-008 | 7 | 全部硬规则满足 + 视觉对齐 v2 preview | T-UI-008 全部 8 项 grep gate + 3 viewport 视觉 |

每阶段独立可合并。Phase 1 失败可回滚(只换 token,不动布局)。Phase 2 失败可回滚(还原 App.tsx + Modal.tsx)。Phase 3 失败可回滚(细节组件独立)。

---

## 5. Phase 1 — Token + shadcn Restyle

### 5.1 顺序

T-UI-001 和 T-UI-002 可并行写(无文件交叉),完成后统一跑 `pnpm tsc --noEmit` + `pnpm build`。

### 5.2 文件清单(8)

**T-UI-001 Token 替换**

| 文件 | 改动 |
|---|---|
| `webui/src/styles/globals.css` | 重写为 Kaku light + dark 主题(REQ-UI-001.1 / 001.3) |
| `webui/tailwind.config.ts` | `extend.fontFamily` / `spacing` / `borderRadius` / `colors` 引用 token(REQ-UI-001.2 / 001.3) |
| `webui/index.html` | 加载 Google Fonts Charter(REQ-UI-001.2) |

**T-UI-002 shadcn 组件落 Kaku 风格**

| 文件 | 改动 |
|---|---|
| `webui/src/components/ui/button.tsx` | 圆角 pill (999px),字阶 label-lg,无 shadow |
| `webui/src/components/ui/card.tsx` | 圆角 8px,1px 边框,无 shadow |
| `webui/src/components/ui/input.tsx` | 圆角 8px,1px 边框,字阶 body-md |
| `webui/src/components/ui/select.tsx` | 同 input |
| `webui/src/components/ui/toast.tsx` | 边框 + 8px + 无 shadow |

`webui/src/lib/utils.ts` 不变(shadcn `cn` 工具)。

### 5.3 验证

```bash
cd webui
pnpm tsc --noEmit
pnpm build
```

视觉:启动 dev,旧布局应是 Kaku 主题色(navy + off-white + Charter 标题),按钮 pill 化,卡片 8px 圆角 1px 边框,无 shadow。

### 5.4 Owner

单 @fixer。8 文件,~30 min。

---

## 6. Phase 2 — Layout + State

### 6.1 依赖

T-UI-001 / 002 done。

### 6.2 并行结构

```
T-UI-001/002 done
   │
   ├──► T-UI-003 (App.tsx + Titlebar + TopBar)         ─┐
   ├──► T-UI-004 (ProviderCard + Grid + Density + Chips) ─┤── 3 @fixer 并行
   ├──► T-UI-005 (Modal 重写 + ProviderDetailModal)     ─┘
   │
   └──► T-UI-007 (App.tsx state)  串行 T-UI-003
```

**写冲突**:
- T-UI-003 和 T-UI-007 都改 `App.tsx`,串行 003 → 007
- T-UI-004 删 `CategorySidebar.tsx` + `ProviderList.tsx`,但 T-UI-003 不再 import 它们 → 无冲突
- T-UI-005 改 `Modal.tsx` + 5 个 modal 文件 + 删 `ProviderDetail.tsx`;5 个 modal import 路径不变 → T-UI-005 内部串行
- T-UI-003 / 004 / 005 之间无文件交叉 → 3 个 @fixer 并行
- **合并约束 (H-2)**: T-UI-003 改写 `App.tsx` 时 import `ProviderGrid` (T-UI-004 新建) + `ProviderDetailModal` (T-UI-005 新建)。若 003 先单独 merge,`pnpm tsc --noEmit` 失败于缺模块。**修复**: 三者必须 squash 成单个 stage-12 commit 同时落地 (或在 003 完成前 stub 占位 import)

### 6.3 T-UI-003 — App Shell 骨架

| 文件 | 改动 |
|---|---|
| `webui/src/App.tsx` | 完全重写,移除 sidebar;新 state: `density`, `activeProviderId`, `categoryFilter`, `search` |
| `webui/src/components/Titlebar.tsx` (新) | serif "KeyPilot" + ThemeToggle + Settings 文字按钮 |
| `webui/src/components/TopBar.tsx` (新) | search + ChipGroup + DensityToggle 容器 |
| `webui/src/components/ThemeToggle.tsx` | 行为不变,色值走新 token;lucide SVG 替代可能的 emoji |

App.tsx 重写后,**未实现 ProviderGrid / ProviderDetailModal 时**(Phase 2 进度中途)内容区是空的。这是可接受的,因为 T-UI-004 / 005 与 T-UI-003 并行执行,合并到主分支前全部到位。

### 6.4 T-UI-004 — 卡片网格

| 文件 | 改动 |
|---|---|
| `webui/src/components/ProviderCard.tsx` (新) | drag handle + icon + name + URL + clock+time + refresh + quota(REQ-UI-003) |
| `webui/src/components/ProviderGrid.tsx` (新) | 容器,根据 `<html data-density>` 切 1-col / 2-col |
| `webui/src/components/SectionLabel.tsx` (新) | 复用,serif 12px tracked uppercase |
| `webui/src/components/ChipGroup.tsx` (新) | 顶栏 category chip |
| `webui/src/components/DensityToggle.tsx` (新) | 1-col / 2-col 切换器 |
| `webui/src/components/ContextMenu.tsx` (新) | ProviderCard 右键触发,内含 "Delete credential" 项(用 Radix DropdownMenu) |
| **删** `webui/src/components/CategorySidebar.tsx` | sidebar 移除 |
| **删** `webui/src/components/ProviderList.tsx` | 卡片替代 |

**ProviderCard onContextMenu 路径**:
```
右键卡片
  → onContextMenu 内 `e.preventDefault()` 抑制 WebView2 原生菜单 (M-9)
  → ContextMenu 显示 (Radix DropdownMenu)
  → "Delete credential" 项
  → 弹 ConfirmDialog (Modal.tsx 新壳包,见 §6.5)
  → 用户确认
  → 调 deleteProvider IPC
  → invalidate queries: ['providers']
```

**搜索状态提升 (H-5 + M-1)**:
- 原 `CategorySidebar.tsx` 顶部 search input + `useProviders.filterProviders`(stage-11 锁)迁移
- search state 上提 `App.tsx`,调用 **独立函数** `filterProviders(providers, search)` 进行客户端过滤
- 调用点在 `ProviderGrid` (T-UI-004):render 前 `filterProviders(useProviders().data ?? [], search)` → map 出 `ProviderCard`
- `useProviders` 钩子和 `filterProviders` 函数签名都不变(零改动)

**ProviderCard 图标 tint fallback (M-16)**:
- family-tinted 映射 (OpenAI=绿, DeepSeek=蓝, Anthropic=橙, GitHub=灰, Postgres=蓝, Redis=红) 硬编码在 `ProviderCard.tsx` 内
- 未匹配 family → `var(--color-muted)` 背景 + provider name 首字母 (mono font)

**ProviderGrid 三态 (L-2)**:
- **Empty** (0 providers): 居中文案 "No credentials yet" + 副标 "Add your first credential to get started"
- **Loading** (`useProviders().isLoading`): 2 张骨架卡 (1px border + `--color-border` bg + CSS pulse 动画)
- **Error** (`useProviders().isError`): inline "Failed to load credentials" + 重试文本按钮 (不阻塞交互)

### 6.5 T-UI-005 — Modal 体系

| 文件 | 改动 |
|---|---|
| `webui/src/components/Modal.tsx` (重写) | 文件名不动,内部切 Radix Dialog;720px 居中 + 8px 圆角 + 1px 边框 + scrim `rgba(0,0,0,0.4)` + focus trap + Esc 关闭 + overlay click 关闭 |
| `webui/src/components/ProviderDetailModal.tsx` (新) | Header (含 trash icon ghost button 次入口 Delete + Edit/Cancel 切换 + 派生 status pill) + Quota section (内含 inline 2px progress bar,**不是 QuotaBadge** — 见 H-4) + Credentials section (KV 列表 + mask + Eye 切换) + Footer (Cancel / Fetch quota / Test connection,左→右顺序钉死 — 见 M-15) |
| `webui/src/components/AddCredentialModal.tsx` | 应用 Modal.tsx 新壳(import 路径不变) |
| `webui/src/components/AddKvModal.tsx` | 同上 |
| `webui/src/components/ManualQuotaModal.tsx` | 同上;**改第 113 行注释**:`V0.1: localStorage-only, V0.1.1+: real IPC` → `真实 IPC 调用,localStorage 仅预填上次输入值`;**嵌套在 ProviderDetailModal 内打开** (Radix Dialog 自动处理 focus stack — 见 M-11) |
| `webui/src/components/SettingsModal.tsx` | 同上 |
| `webui/src/components/ConfirmDialog.tsx` | 同上 |
| **删** `webui/src/components/ProviderDetail.tsx` | 内容并入 `ProviderDetailModal` |

**ProviderDetailModal 行为保留审计**:

| 原 ProviderDetail 行为 | ProviderDetailModal 位置 |
|---|---|
| `isEditing` 编辑 mode (name + notes) | Header 顶部,Edit/Cancel 切换。**状态机 (M-10)**: View → Edit (点 Edit) → View (Cancel 丢弃改动) 或 View (modal close 隐式保存)。无显式 Save 按钮 |
| `testMutation` | Footer "Test connection" pill (右 1,最右) |
| `deleteMutation` (Header 按钮) | Header 右上 trash icon ghost button + 卡片右键 context menu 双入口 |
| 字段增/改/删 (`AddKvModal` / `KvRow.onUpdate/onDelete`) | Credentials section,逐行操作 |
| `manualQuotaOpen` | Quota section "Edit manually" 链接 → ManualQuotaModal 嵌套打开 (Radix Dialog 自动 focus stack) |
| `QuotaBadge` 嵌入 | **不嵌入 QuotaBadge**;Quota section 内含 inline `<ProgressBar>` (2px, full-width, primary fill, no radius)。`QuotaBadge` 是单独组件,grid 卡片右侧 quota 文本显示用 |
| `presetLabel` | Header status pill 旁 |
| `onFetchQuota` (新增) | Footer "Fetch quota" pill (右 2,在 Cancel 与 Test 之间) |
| Esc / overlay click 关闭 | Radix Dialog 默认 |
| Status pill 文案 (M-14) | 派生自 `provider.last_tested`:`formatDistanceToNow` → "Tested 2h ago" 等;`null` 时显示 muted pill "Not tested" |
| Toast firing (L-1) | (a) 删除成功 → "Credential deleted";(b) Quota 拉取成功 → "Quota updated";(c) Test connection 成功 → "Connection OK" (绿);(d) Test connection 失败 → 错误 toast 含 message;(e) Save on close → 静默无 toast |
| 长 credential 值 (L-3) | masked: `前 3 + ••• + 后 3` (e.g., `sk-•••••abc`);unmasked: mono font + `overflow-x: auto` 横向滚动 (无截断) |
| Modal 滚动 (L-4) | 容器不滚动;KV list 区域 `max-height: calc(100vh - 220px); overflow-y: auto` (Header + Footer 固定) |

### 6.6 T-UI-007 — 状态管理

| 文件 | 改动 |
|---|---|
| `webui/src/App.tsx` | 加 `density` state(`localStorage.keypilot.density` 读写,默认 `'1'`);`<html data-density="1\|2">` 注入,驱动 grid 切列;`categoryFilter` state(default 'all');`search` state(复用 stage-11 `useProviders.filterProviders`) |

`useProviders` 钩子签名不变 (无参数),`filterProviders` 函数签名不变 (零改动)。`categoryFilter` 走新加的 `ChipGroup`,**ChipGroup 暂不接 `useCategories`** (硬编码字符串 "All / AI / Databases / Dev",V0.X 接入动态分类 — 见 M-13)。

**Effect 顺序 (M-12)**: App.tsx 中 `useTheme()` 的 `useEffect` (写 `data-theme`) 在 `useState(density)` 后声明的 `useEffect` (写 `data-density`) **之前**执行。两者独立 React effect,执行顺序按 declaration order;为避免主题闪烁 (尤其 Auto 模式下依赖 matchMedia),`data-theme` 必须先解析。

### 6.7 验证

```bash
cd webui
pnpm tsc --noEmit
pnpm build
```

视觉:启动 dev,应有卡片网格,点卡片开 modal,右键卡片弹 context menu,删除有 confirm。

### 6.8 Owner

3 个 @fixer 并行(003 / 004 / 005),007 由 003 后续派。

---

## 7. Phase 3 — Detail Polish + Verify

### 7.1 顺序

T-UI-006 完成后跑 T-UI-008。串行。

### 7.2 T-UI-006 — 细节组件

| 文件 | 改动 |
|---|---|
| `webui/src/components/KvRow.tsx` | key 用 `var(--color-primary)`,value 用 mono |
| `webui/src/components/QuotaBadge.tsx` | 改用 Card 容器,**仅显示 quota 文本 + 数字 (无 progress bar)**;progress bar 在 `ProviderDetailModal` 内 inline 实现 (H-4) |
| `webui/src/components/CopyButton.tsx` | 改 lucide `Copy` / `Eye` / `EyeOff` SVG |
| `webui/src/components/Icon.tsx` | 移除 emoji,统一 lucide-react |
| `webui/src/components/ErrorBoundary.tsx` | **untracked (1538B stub)** → 套 Kaku 风格 + 首次纳入 git 跟踪 (`git add`) 后 commit (M-7) |
| `webui/src/components/TrayHoverCard.tsx` | 套 Card 风格(system tray) |

### 7.3 T-UI-008 — 验收

```bash
# 编译 / 类型
cargo check --manifest-path src-tauri/Cargo.toml
cd webui && pnpm tsc --noEmit
pnpm build

# 硬约束 grep(AGENTS.md §9 + spec §4)
grep -rn "box-shadow" webui/src/ | grep -v "box-shadow: none"             # 期望 0
grep -rn "linear-gradient\|radial-gradient\|conic-gradient" webui/src/   # 期望 0
grep -rn "backdrop-filter" webui/src/                                    # 期望 0
grep -rn "filter: blur" webui/src/                                       # 期望 0
grep -rn " — " webui/src/                                                # 期望 0
grep -rn "#5b5bd6\|#fcfcfc" webui/src/                                   # 期望 0
grep -rnP "[\x{1F300}-\x{1F9FF}\x{2600}-\x{27BF}]" webui/src/ webui/index.html  # 期望 0 (零 emoji — M-3)
grep -rnE "1px solid #[0-9a-fA-F]{3,6}" webui/src/                       # 期望 0 (零裸色边框 — M-3,改用 1px solid var(--color-border))
grep -rnE "\b(8|10|16|18|20|28|32|40|64)px\b" webui/src/ | grep -vE "var\(--spacing|var\(--radius"  # 报警 (raw px 间距/圆角 — M-3)

# 视觉(开发机)
pnpm tauri dev   # 三 viewport 验收清单 (L-5):
                 # 375px  → 顶栏 2 行 (search / chips+density), grid 1-col, card 全宽, modal 全宽 + 横向 padding
                 # 768px  → 顶栏 1 行, grid 可切 1-col/2-col (user toggle), modal 720px + 侧 padding
                 # 1280px → 2-col 满,所有元素无横滚, pill 按钮 border-radius: 999px (DevTools computed)
                 # 主题切换 Auto/Light/Dark 三态
                 # 密度切换 1-col/2-col
                 # Detail Modal 打开/关闭/Escape/overlay click
                 # 右键卡片 → Delete → ConfirmDialog (含 WebView2 contextmenu 事件验证)
                 # 对照 webui/dist/design-preview.html v2
```

### 7.4 Owner

单 @fixer(006 改动小 + 008 是验证命令串)。

---

## 8. 组件数据流

```
                       ┌─────────────────┐
                       │   App.tsx       │
                       │  state:         │
                       │   density       │
                       │   activeProviderId
                       │   categoryFilter│
                       │   search        │
                       └──┬──────┬───────┘
                          │      │
            ┌─────────────┘      └──────────────┐
            ▼                                    ▼
   ┌─────────────────┐                ┌──────────────────────┐
   │  Titlebar       │                │  TopBar              │
   │  ThemeToggle    │                │  search → App        │
   │  Settings btn   │                │  ChipGroup → App     │
   └─────────────────┘                │  DensityToggle → App │
                                      └──────────┬───────────┘
                                                 │
                                                 ▼
                                      ┌──────────────────────┐
                                      │  ProviderGrid        │
                                      │  data-density        │
                                      │  read: providers     │
                                      └──────┬───────────────┘
                                             │ onClick / onContextMenu
                          ┌──────────────────┴──────────────────┐
                          ▼                                     ▼
              ┌──────────────────────┐            ┌──────────────────────┐
              │  ProviderDetailModal │            │  ContextMenu         │
              │  Header(trash icon)  │            │  "Delete credential" │
              │  Quota section      │            │  → ConfirmDialog     │
              │  Credentials (KV)    │            │  → deleteProvider    │
              │  Footer actions      │            └──────────────────────┘
              └──────────────────────┘
```

无环。`activeProviderId` 与 `density` 是 single source of truth,`App` 唯一写者。

---

## 9. 攻击角度

| 角度 | 问题 | 缓解 | 抗住? |
|---|---|---|---|
| 依赖失败 | Google Fonts CDN 不可达 | Charter → Georgia → Palatino → Times New Roman 字体回退链(系统字体,离线可用) | ✓ |
| 依赖失败 | lucide-react / Radix 包故障 | 本地 node_modules,无网络调用 | ✓ |
| 回滚成本 | 改完 30 文件后想回滚 V0.1 风格 | git revert 单 commit,改动全在 webui/ | ✓ |
| 数据迁移 | localStorage 加 `keypilot.density` 新 key | 读时容错,无值 fallback `'1'` | ✓ |
| 平台差异 | Tauri WebView2 不支持某 CSS feature | spec 内全部 token / utility 已是 WebView2 baseline 支持 | ✓ |
| 暗色对比度 | navy 暗色变体 AA 不达标 | primary `#7da3d1` 与 surface `#1a1916` 对比度约 7.4:1 (WCAG AA 通过 4.5:1 阈值);muted `#8a8780` (实测命令:`npx wcag-contrast #7da3d1 #1a1916`) | ✓ |

---

## 10. Verification Gates(最终验收)

### 10.1 编译 / 类型

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` PASS
- [ ] `pnpm tsc --noEmit` PASS
- [ ] `pnpm build` PASS,产物更新到 `webui/dist/`

### 10.2 硬约束 grep(AGENTS.md §9 + spec §4)

- [ ] `box-shadow` 除 `none` → 0
- [ ] `linear-gradient` / `radial-gradient` / `conic-gradient` → 0
- [ ] `backdrop-filter` / `filter: blur` → 0
- [ ] em-dash ` — ` → 0
- [ ] 旧色 `#5b5bd6` / `#fcfcfc` → 0

### 10.3 行为

- [ ] 启动一次应用,1280 / 768 / 375 三 viewport 视觉对齐 `webui/dist/design-preview.html` v2
- [ ] 主题切换 Auto / Light / Dark 三态 OK
- [ ] 密度切换 1-col / 2-col OK
- [ ] Detail Modal 打开 / Escape 关闭 / overlay click 关闭
- [ ] 右键卡片 → context menu → Delete → ConfirmDialog → 删除
- [ ] Header trash icon → 删除(次入口)
- [ ] Manual quota 修改持久(走 IPC,刷新不丢)

### 10.4 反自欺

- [ ] 无 `.skip` / `.todo` / `unimplemented!()` 残留
- [ ] 无"应该可以工作"但没跑过的代码路径
- [ ] 跨多 SQL 语句的命令无(本 change 不动 Rust,但若涉及需 `tx.commit()?` 事务化)

---

## 11. Rollback

**3 个 phase commit (M-8)**: T-UI-001..008 落地为 3 个 phase commit (Phase 1 / Phase 2 squash / Phase 3)。

**完整回滚**:
```bash
git revert <phase-3-commit>          # Phase 3
git revert <phase-2-commit>          # Phase 2 (含 003/004/005 squash)
git revert <phase-1-commit>          # Phase 1
```

或单点回滚到 N-1 阶段:
```bash
git reset --hard <phase-{N-1}-commit>  # 仅保留 Phase 1..N-1
```

- 不动 `src-tauri/`、不动 SQLite schema、不动 IPC
- 用户偏好(theme / density)留 `localStorage`,下次启动无影响
- 视觉回到 V0.1 风格,功能零丢失

---

## 12. 关联需求 (Cross-references)

- 数据契约: `webui/src/types/api.ts` (V0.1 + V0.2 锁,本 change 不改接口)
- 既有 hook: `useProviders`, `useQuota`, `useTheme`, `useCategories` (V0.1 + V0.2,本 change 不改逻辑,只更新样式)
- 设计系统真相源: `DESIGN.md` (kaku)
- 视觉验证基准: `webui/dist/design-preview.html` (v2)
- 后续 openspec: `openspec/changes/token-usage-history/` (V0.3 沿用本 change 的设计系统)
- AGENTS.md §9 Sprint Contract 全部适用

---

## 13. 后续动作

**Plan 落地前 (L-8 pre-flight)**:

0. Stage-12 fixer 在动手前先跑 `git diff --stat webui/src/ src-tauri/`,确认 doc 未提交 WIP 与本次 Kaku token 迁移无冲突;若冲突,先 git stash 再开工。

**Plan 落地后**:

1. 派 @fixer 跑 Phase 1(token + shadcn),单 @fixer,~30 min。**T-UI-001 必须包含 (a) tailwind.config.ts:4 darkMode 迁移 + (b) ThemeToggle.tsx:17-28 data-theme 迁移 (H-3)**
2. Phase 1 全绿后,派 3 个 @fixer 并行跑 Phase 2 (003/004/005),**三者 squash 成单个 stage-12 commit 同时落地** (H-2);007 串行
3. Phase 2 全绿后,派 @fixer 跑 Phase 3(006 + 008 验证)
4. 跑 `/check` 收尾
5. 更新 `feature_list.json` `stage-12.status = "done"` + `evidence` 字段记录 grep gate 输出
6. 提交 commit,信息格式按 AGENTS.md §6

---

*最后更新: 2026-06-25(基于 `openspec/changes/ui-redesign/{proposal,spec,design,tasks}.md` 衍生)*
