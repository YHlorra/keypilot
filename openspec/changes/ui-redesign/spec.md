# UI Redesign (Kaku Design System) : Spec

## REQ-UI-001: 设计 Token

### REQ-UI-001.1: 颜色 (Color)

#### Light 主题 (`:root`,`[data-theme="light"]`)

| Token | 值 | 用途 |
|---|---|---|
| `--color-primary` | `#1b365d` | 主操作 (按钮 / active state / drag handle hover) |
| `--color-secondary` | `#faf9f5` | primary 上的文字 |
| `--color-tertiary` | `#e5e7eb` | 弱化边框、分割线 |
| `--color-neutral` | `#141413` | 主文字 |
| `--color-surface` | `#f5f4ed` | 主画布、卡片背景 |
| `--color-surface-sunken` | `#efeddf` | 凹陷面 (e.g., 已选卡片) |
| `--color-on-surface` | `#141413` | 画布上文字 |
| `--color-background` | `#f5f4ed` | 全局背景 |
| `--color-muted` | `#6b6b65` | 弱化文字 |
| `--color-border` | `#e5e7eb` | 1px 边框 |
| `--color-link` | `#1d4ed8` | URL / 链接文字 (Kaku 调性蓝,独立于 primary navy) |
| `--color-success` | `#46a758` | quota 正常、success pill |
| `--color-error` | `#b42318` | 错误状态 |

#### Dark 主题 (`[data-theme="dark"]`)

| Token | 值 | 说明 |
|---|---|---|
| `--color-primary` | `#7da3d1` | lightened navy,达 AA 对比度 |
| `--color-secondary` | `#0e0d0c` | primary 上的文字 |
| `--color-surface` | `#1a1916` | warm near-black 画布 |
| `--color-surface-sunken` | `#131210` | 凹陷面 |
| `--color-neutral` | `#ece9df` | warm off-white 文字 |
| `--color-on-surface` | `#ece9df` | 画布上文字 |
| `--color-muted` | `#8a8780` | 弱化文字 |
| `--color-border` | `#2e2c28` | 1px 边框 |
| `--color-link` | `#7da3d1` | 链接 (与 primary 同色) |
| `--color-success` | `#7ad27d` | quota 正常 |
| `--color-error` | `#e87a72` | 错误 |

#### Auto 主题 (`[data-theme="auto"]`)

JS 监听 `matchMedia('(prefers-color-scheme: dark)')`,在 light / dark 之间切换。无独立 token。

### REQ-UI-001.2: 字体 (Typography)

```css
--font-serif: "Charter", Georgia, Palatino, "Times New Roman", serif;
--font-sans: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", Helvetica, Arial, sans-serif;
--font-mono: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
```

字阶 (按 `DESIGN.md` 锁死):

| Token | font-size / weight / line-height / letter-spacing | 用途 |
|---|---|---|
| Headline display | 85.05px / 500 / 86.751px / -1.4px | (本 app 不用,保留与 kaku 一致) |
| Headline lg | 37.8px / 500 / 44.604px / 0 | Modal 标题 (本期未启用,留作 V0.X) |
| Headline md | 20px / 500 / 24px / 0 | Section 标题 (e.g., "Quota", "Credentials") |
| Body lg | 16px / 500 / 24px / 0.4px | 强调正文 |
| Body md | 16px / 500 / 24px / 0.4px | 正文 |
| Body sm | 15px / 400 / 22px / 0.2px | 弱化正文、说明、helper text |
| Label lg | 15px / 500 / 1.2 / 0 | 表单 label、按钮文字 |
| Label md | 15px / 500 / 1.2 / 0 | 次级 label |
| Label sm | 15px / 400 / 1.2 / 0 | index label,12px tracked uppercase |

serif 字体加载:

```html
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Charter:ital,wght@0,400;0,500;0,700&display=swap" rel="stylesheet">
```

离线 / 加载失败回退: Charter → Georgia → Palatino → Times New Roman。Georgia 保留编辑感,无回退到 sans 的路径。

### REQ-UI-001.3: 间距与圆角 (Spacing & Radius)

```css
--spacing-xs: 2px;
--spacing-sm: 10px;
--spacing-md: 18px;
--spacing-lg: 28px;
--spacing-xl: 64px;
--radius-sm: 8px;
--radius-pill: 999px;
```

- Pill (`999px`): 按钮、chip、search input、density toggle、theme toggle
- Card (`8px`): 卡片、modal、form input

### REQ-UI-001.4: 硬约束 (Hard Rules)

| 规则 | 实现 |
|---|---|
| 零阴影 | 全局禁 `box-shadow` (除 `none`)、`backdrop-filter`、`filter: blur` |
| 零渐变 | 禁 `linear-gradient`、`radial-gradient`、`conic-gradient` |
| 零玻璃 | 禁 `backdrop-filter` |
| 零 em-dash | 任何文本、注释、CSS 内容、HTML 文本都不能含 `—` |
| 零营销元素 | 不加 hero、stats row、CTA 组合 |
| 分隔靠 1px 边框 + 背景步进 | 至少 2% lightness 差,主区域用 1px `--color-border` |
| 零 emoji | Titlebar 不带 🔑,Settings 按钮不用 ⚙️,统一 lucide-react SVG |
| 零 padding magic | 间距统一用 `--spacing-xs/sm/md/lg/xl` token,不写裸 `8px` / `16px` 等 |

## REQ-UI-002: App Shell 布局

### REQ-UI-002.1: Titlebar (44px 高)

- 左侧: serif "KeyPilot" 字标,Charter 17px / 500 / letter-spacing -0.2px,`var(--color-neutral)`,**无 emoji**
- 右侧: 3-button ThemeToggle (Auto / Light / Dark,active 1px navy 下划线 + weight 500) + Settings 文字按钮 (lucide-react `Settings` SVG 16px + 文字)

### REQ-UI-002.2: Top Bar (60px 高,Titlebar 下方)

- 左侧: 搜索 input (360px 宽,placeholder "Search credentials…",pill 999px,1px 边框,无填充,system sans 14px)
- 中部: 4 chip group (All / AI / Databases / Dev),pill 999px,1px 边框,active chip 填充 navy + 文字 off-white
- 右侧: DensityToggle (1-col / 2-col,两个 icon 按钮,active navy 填充 + off-white icon)

### REQ-UI-002.3: Card Grid (Top bar 下方,占满剩余高度,可滚动)

- Section label "CREDENTIALS" 顶部,serif label-sm 12px / 400 / tracking +0.06em / uppercase,`var(--color-primary)`
- Grid container: `display: grid; gap: var(--spacing-md);`
  - 1-col (`[data-density="1"]`): `grid-template-columns: 1fr`
  - 2-col (`[data-density="2"]`): `grid-template-columns: 1fr 1fr`
- 容器 padding: top=`var(--spacing-xl)` (64px), left/right=`var(--spacing-lg)` (28px), bottom=`var(--spacing-xl)` (64px)
- 响应式:
  - `<= 640px`: 强制 1-col,top bar 换行 (search row 1, chips + density row 2)
  - `<= 768px`: 2-col 仍可,1-col 优先

### REQ-UI-002.4: Detail Modal

- 居中 modal overlay,720px 宽,8px 圆角,1px 边框,无阴影
- 内部: Close 按钮右上 (lucide-react `X` SVG 16px) + Header (name + URL + status pill) + Quota section + Credentials section (KV 列表) + Footer (Cancel + Test + Fetch pills)
- 背景 scrim: `rgba(0,0,0,0.4)`,半透明黑
- 关闭触发: Escape 键 / 点击 overlay / 关闭按钮
- 沿用 Radix Dialog 行为 (focus trap, scroll lock)

## REQ-UI-003: Provider Card

### REQ-UI-003.1: 卡片结构 (从左到右,水平排列,垂直居中)

| 位置 | 元素 | 样式 |
|---|---|---|
| 1 | Drag handle | 6-dot grip SVG,16px 宽,`var(--color-muted)`,垂直居中 |
| 2 | Provider icon | 32px 圆,family-tinted 背景 (OpenAI=绿、DeepSeek=蓝、Anthropic=橙、GitHub=灰、Postgres=蓝、Redis=红),内部白字首字母 (e.g. "AI", "GH", "DB")。正式版可换真实品牌 SVG |
| 3 | Provider info (flex 1) | name 16px sans weight 600 `var(--color-neutral)`; URL 14px sans weight 400 `var(--color-link)` |
| 4 | Right meta cluster (右对齐,18px 间距) | clock icon 16px muted + 时间文本 (e.g. "刚刚" / "2h ago"),13px sans weight 400 muted / refresh icon button 16px SVG,hover 时 primary / quota 文本: `<quota-num> <span>unit</span>`,quota-num 15px sans weight 600 `var(--color-success)`,unit 13px sans weight 400 `var(--color-muted)` |

### REQ-UI-003.2: 状态

| 状态 | 视觉 |
|---|---|
| 默认 | surface 背景,1px border-tertiary |
| Hover | border darkens 到 `--color-primary` |
| Selected | 4px left border `--color-primary` + 表面 stepped (background `var(--color-surface-sunken)`) |
| 拖拽 affordance | drag handle hover cursor `grab`,active `grabbing` (本期不接 dnd 库,仅 cursor) |

### REQ-UI-003.3: 引用图片样式 (与 v2 preview 一致)

```
[≡] [AI] DeepSeek                            [⏰ 刚刚] [↻] 6.14 CNY
          https://platform.deepseek.com
```

### REQ-UI-003.4: 点击行为

- 点击卡片任意位置 → 打开 `ProviderDetailModal`,传入 `providerId`
- 点击 refresh 按钮 → 单独触发 `fetch_quota(providerId)`,不打开 modal (`e.stopPropagation()`)
- 点击 URL → 新窗口打开 (不打开 modal)

## REQ-UI-004: 主题系统 (Theme System)

### REQ-UI-004.1: 状态

3 状态: `auto` / `light` / `dark`,存在 `<html data-theme="...">`。

### REQ-UI-004.2: 切换器

Titlebar 上的 3 个文字按钮 (Auto / Light / Dark),active 状态 1px navy 下划线 + weight 500 + 1px navy underline。

### REQ-UI-004.3: Auto 行为

JS 监听 `matchMedia('(prefers-color-scheme: dark)')`,在 `data-theme="auto"` 时:

- matchMedia matches dark → 应用 `[data-theme="dark"]` 规则
- matchMedia matches light → 应用 `[data-theme="light"]` 规则
- 切换 OS 主题时,实时响应

无独立 `data-theme="auto"` 规则,auto 是 resolver,不是 state。

### REQ-UI-004.4: 持久化

- localStorage key: `keypilot.theme`
- 应用启动时读 → 写 `data-theme`
- ThemeToggle 改变时写回

## REQ-UI-005: Detail Modal 内容

### REQ-UI-005.1: Header

- Provider name: serif headline-md 20px / 500,`var(--color-neutral)`
- URL: `var(--color-link)` 14px,点击新窗口打开
- Status pill: "Tested 2h ago" 文字 + 1px 边框,无填充,小号 sans 13px

### REQ-UI-005.2: Quota Section

- Heading: serif headline-md "Quota"
- Quota main: 16px sans weight 500 "<used> of <total> used" + 右侧 "Resets <date>" 13px muted
- Progress bar: 2px 高,全宽,canvas 背景,primary 填充百分比,无圆角,无 glow
- Sub: 13px sans weight 400 muted "Subscription · Tier N"

### REQ-UI-005.3: Credentials Section

- Heading: serif headline-md "Credentials"
- KV 列表 (复用 `KvRow` 组件):
  - Key: 14px sans weight 500 `var(--color-primary)`
  - Value: mono 13px weight 400,默认 masked (`sk-•••••abc`)
  - 右侧按钮: visibility toggle (lucide `Eye` / `EyeOff`) + copy (lucide `Copy`)
- 最后一行无 border-bottom

### REQ-UI-005.4: Footer

- 左侧: secondary pill "Cancel"
- 右侧: secondary pill "Fetch quota" + primary pill "Test connection"

## REQ-UI-006: 边界 (Boundaries)

- **零新 IPC**: 本 change 不新增 / 不改 IPC 命令
- **零新 schema**: SQLite schema v3 不动
- **零新依赖**: 不引 drag-and-drop 库 (本期不做拖拽重排,仅 affordance)
- **零新后端代码**: 纯前端
- **零平台迁移**: 不动 Windows 平台特有 UI (Titlebar 不换成 Tauri decoration,继续 44px 固定高度)
- **零加密**: 沿用 V0.1 明文存储决策
- **零 emoji**: Titlebar 不带 🔑,Settings 按钮不用 ⚙️,改 lucide SVG
- **零 1px 裸值**: 边框统一 `1px solid var(--color-border)`,不写 `1px solid #e5e7eb`
- **零 16px 裸值**: 行高 / 间距 / icon 尺寸都走 token 或 Tailwind utility

## 关联需求 (Cross-references)

- 数据契约: 来自 `webui/src/types/api.ts` (V0.1 + V0.2 锁,本 change 不改)
- 既有 hook: `useProviders`, `useQuota`, `useTheme`, `useCategories` (V0.1 + V0.2,本 change 不改逻辑,只更新样式)
- 设计系统: `DESIGN.md` (kaku)
- 视觉验证: `webui/dist/design-preview.html` (v2)
