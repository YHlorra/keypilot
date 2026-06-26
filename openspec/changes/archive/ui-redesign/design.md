# UI Redesign (Kaku Design System) : Design

## 1. 架构总览

### 1.1 旧 (v1,当前 webui)

```
┌─────────────────────────────────────────────────┐
│ Titlebar (44px) : KeyPilot + 🔑                  │
├──────────┬──────────────────────────────────────┤
│ Sidebar  │  Main Pane                            │
│ (300px)  │  (ProviderDetail)                     │
│          │                                       │
│ search   │  Provider header                      │
│ category │  Quota section                        │
│ provider │  KV list                              │
│ list     │  Action buttons                       │
│          │                                       │
└──────────┴──────────────────────────────────────┘
```

### 1.2 新 (v2,本 change 目标)

```
┌──────────────────────────────────────────────────────────┐
│ Titlebar (44px) : KeyPilot [serif] | Theme | Settings    │
├──────────────────────────────────────────────────────────┤
│ Top Bar (60px) : [search] [chips] [density 1|2]          │
├──────────────────────────────────────────────────────────┤
│ CREDENTIALS                                              │
│                                                          │
│  ┌──────────────────────────────────────────────────┐   │
│  │ [≡] [AI] OpenAI Production   [⏰ 2h] [↻] $42.18  │   │
│  │             https://api.openai.com/v1            │   │
│  └──────────────────────────────────────────────────┘   │
│  ┌────────────────┐  ┌────────────────┐                  │
│  │ DeepSeek       │  │ Anthropic      │   (2-col)        │
│  └────────────────┘  └────────────────┘                  │
│                                                          │
│  ┌──── Detail Modal (720px, on click) ────────┐           │
│  │ Header: name + url + status pill            │           │
│  │ Quota section + 2px progress bar             │           │
│  │ Credentials (KV list, mono masked)           │           │
│  │ Footer: [Cancel] [Test] [Fetch]              │           │
│  └─────────────────────────────────────────────┘           │
└──────────────────────────────────────────────────────────┘
```

## 2. Token 实现

### 2.1 globals.css 改造

**Before** (V0.1 ~ V0.2, shadcn Radix):

```css
:root {
  --color-background: #fcfcfc;
  --color-foreground: #202020;
  --color-primary: #5b5bd6;
  --color-border: #d9d9d9;
  --color-success: #46a758;
  --color-error: #e5484d;
  --radius: 0.5rem;
}
.dark { /* hardcoded iris on dark, simple inversion */ }
```

**After** (Kaku):

```css
:root, [data-theme="light"] {
  --color-primary: #1b365d;
  --color-secondary: #faf9f5;
  --color-tertiary: #e5e7eb;
  --color-neutral: #141413;
  --color-surface: #f5f4ed;
  --color-surface-sunken: #efeddf;
  --color-on-surface: #141413;
  --color-background: #f5f4ed;
  --color-muted: #6b6b65;
  --color-border: #e5e7eb;
  --color-link: #1d4ed8;
  --color-success: #46a758;
  --color-error: #b42318;
  --font-serif: "Charter", Georgia, Palatino, "Times New Roman", serif;
  --font-sans: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", Helvetica, Arial, sans-serif;
  --font-mono: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
  --spacing-xs: 2px;
  --spacing-sm: 10px;
  --spacing-md: 18px;
  --spacing-lg: 28px;
  --spacing-xl: 64px;
  --radius-sm: 8px;
  --radius-pill: 999px;
}

[data-theme="dark"] {
  --color-primary: #7da3d1;
  --color-secondary: #0e0d0c;
  --color-surface: #1a1916;
  --color-surface-sunken: #131210;
  --color-neutral: #ece9df;
  --color-on-surface: #ece9df;
  --color-background: #1a1916;
  --color-muted: #8a8780;
  --color-border: #2e2c28;
  --color-link: #7da3d1;
  --color-success: #7ad27d;
  --color-error: #e87a72;
}
```

### 2.2 tailwind.config.ts 改造

`extend` 新增:

```ts
theme: {
  extend: {
    fontFamily: {
      serif: ['var(--font-serif)'],
      sans: ['var(--font-sans)'],
      mono: ['var(--font-mono)'],
    },
    spacing: {
      'space-xs': 'var(--spacing-xs)',
      'space-sm': 'var(--spacing-sm)',
      'space-md': 'var(--spacing-md)',
      'space-lg': 'var(--spacing-lg)',
      'space-xl': 'var(--spacing-xl)',
    },
    borderRadius: {
      sm: 'var(--radius-sm)',
      pill: 'var(--radius-pill)',
    },
    colors: {
      // 旧 shadcn 命名保留 (组件不改名),但值改 Kaku token
      background: 'var(--color-background)',
      foreground: 'var(--color-foreground)',
      card: 'var(--color-surface)',
      cardForeground: 'var(--color-on-surface)',
      primary: 'var(--color-primary)',
      primaryForeground: 'var(--color-secondary)',
      border: 'var(--color-border)',
      ring: 'var(--color-primary)',
      // 新增
      link: 'var(--color-link)',
      success: 'var(--color-success)',
    },
  },
},
```

### 2.3 index.html 加载 Charter

```html
<head>
  ...
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Charter:ital,wght@0,400;0,500;0,700&display=swap" rel="stylesheet">
</head>
```

## 3. 组件改造

### 3.1 删除

| 文件 | 原因 |
|---|---|
| `webui/src/components/CategorySidebar.tsx` | 侧边栏移除 |
| `webui/src/components/ProviderList.tsx` | 卡片替代 sidebar 内列表 |

### 3.2 新增

| 文件 | 职责 |
|---|---|
| `webui/src/components/Titlebar.tsx` | serif "KeyPilot" + ThemeToggle + Settings 文字按钮 |
| `webui/src/components/TopBar.tsx` | search + ChipGroup + DensityToggle 容器 |
| `webui/src/components/ProviderCard.tsx` | 单张凭证卡 (REQ-UI-003) |
| `webui/src/components/ProviderGrid.tsx` | 卡片网格,根据 `<html data-density>` 切 1-col / 2-col |
| `webui/src/components/DensityToggle.tsx` | 1-col / 2-col 切换器 |
| `webui/src/components/ProviderDetailModal.tsx` | 原 detail pane 改成 modal (REQ-UI-005),内含 inline 2px progress bar (非 QuotaBadge,见 execution-plan H-4) |
| `webui/src/components/SectionLabel.tsx` | 复用的 serif 索引 label (e.g., "CREDENTIALS") |
| `webui/src/components/ChipGroup.tsx` | 顶栏 category chip (硬编码字符串 "All / AI / Databases / Dev",V0.X 接入 `useCategories` — 见 execution-plan M-13) |

### 3.3 修改

| 文件 | 修改内容 |
|---|---|
| `webui/src/App.tsx` | 移除 sidebar,改用 Titlebar + TopBar + ProviderGrid + ProviderDetailModal;state: `density: '1' \| '2'`, `activeProviderId: number \| null`, `categoryFilter: string`, `search: string` |
| `webui/src/components/ThemeToggle.tsx` | 行为不变,色值走新 token;lucide SVG 替代可能的 emoji |
| `webui/src/components/ui/button.tsx` | primary / secondary 圆角 → pill (999px),字阶 → label-lg (15px / 500 / 1.2);无 shadow |
| `webui/src/components/ui/card.tsx` | 圆角 → 8px,边框 → 1px tertiary,无 shadow |
| `webui/src/components/ui/input.tsx` | 圆角 → 8px,边框 → 1px tertiary,字阶 → body-md |
| `webui/src/components/ui/select.tsx` | 同 input |
| `webui/src/components/ui/toast.tsx` | 边框 + 8px + 无 shadow |
| `webui/src/components/KvRow.tsx` | key 用 `var(--color-primary)`,value 用 mono |
| `webui/src/components/QuotaBadge.tsx` | 改用 Card 容器 + 2px progress bar (无圆角) |
| `webui/src/components/CopyButton.tsx` | 改 lucide `Copy` / `Eye` / `EyeOff` SVG |
| `webui/src/components/AddCredentialModal.tsx` | 应用 Kaku modal shell |
| `webui/src/components/AddKvModal.tsx` | 同上 |
| `webui/src/components/ManualQuotaModal.tsx` | 同上 |
| `webui/src/components/SettingsModal.tsx` | 同上 |
| `webui/src/components/ConfirmDialog.tsx` | 同上 |
| `webui/src/components/Icon.tsx` | 移除 emoji,统一 lucide-react |
| `webui/src/components/ErrorBoundary.tsx` | 错误页套 Kaku 风格 |
| `webui/src/components/TrayHoverCard.tsx` | 托盘 hover 卡 (system tray) 用 Card 风格 |

### 3.4 状态管理 (App.tsx)

```tsx
const [density, setDensity] = useState<'1' | '2'>(
  () => (localStorage.getItem('keypilot.density') as '1' | '2') ?? '1'
);
const [activeProviderId, setActiveProviderId] = useState<number | null>(null);
const [categoryFilter, setCategoryFilter] = useState<string>('all');
const [search, setSearch] = useState('');

useEffect(() => {
  document.documentElement.setAttribute('data-density', density);
  localStorage.setItem('keypilot.density', density);
}, [density]);
```

`useProviders` filter (stage-11) 复用,`search` 走它,`categoryFilter` 走新加的 `ChipGroup`。无新 IPC。

## 4. 组件契约

### 4.1 ProviderCard

```ts
interface ProviderCardProps {
  provider: ProviderWithFields;
  selected: boolean;
  onClick: () => void;
  onRefresh: (e: React.MouseEvent) => void;
}
```

### 4.2 DensityToggle

```ts
interface DensityToggleProps {
  value: '1' | '2';
  onChange: (v: '1' | '2') => void;
}
```

### 4.3 ProviderDetailModal

```ts
interface ProviderDetailModalProps {
  providerId: number | null;
  onClose: () => void;
  onTest: (id: number) => void;
  onFetchQuota: (id: number) => void;
}
```

### 4.4 ChipGroup

```ts
interface ChipGroupProps {
  options: { value: string; label: string }[];
  value: string;
  onChange: (v: string) => void;
}
```

## 5. Token 映射表 (旧 → 新)

| 用途 | 旧 (shadcn Radix) | 新 (Kaku) |
|---|---|---|
| 主色 | `#5b5bd6` (iris) | `#1b365d` (navy) |
| 主文字 | `#202020` (gray-12) | `#141413` (neutral) |
| 画布 | `#fcfcfc` (gray-1) | `#f5f4ed` (warm off-white) |
| 卡片背景 | `#f9f9f9` (gray-2) | `#f5f4ed` (与画布同,靠边框分) |
| 边框 | `#d9d9d9` (gray-6) | `#e5e7eb` (tertiary) |
| 链接 | (无独立 token) | `#1d4ed8` (link) |
| 成功 | `#46a758` (grass-9) | `#46a758` (沿用) |
| 危险 | `#e5484d` (red-9) | `#b42318` (error,更暗) |
| 标题字 | system sans | Charter serif |
| Body 字 | system sans | system sans (不变) |
| 圆角 | `0.5rem` | `8px` (card) / `999px` (pill) |
| 阴影 | `0 1px 3px rgba(0,0,0,0.1)` | none |
| Dark 主色 | `#5b5bd6` (同 light) | `#7da3d1` (lightened) |

## 6. 非功能需求

| 需求 | 目标 |
|---|---|
| 字体加载失败回退 | Charter 不可用时退到 Georgia,保留编辑感 |
| 主题切换响应 | Auto 主题下 `matchMedia` change 事件触发后,React 重渲染同步完成 (无 wall-clock SLA;不写固定 timeout — 见 execution-plan M-6) |
| 2-col 在 768px 不拥挤 | 768px 2-col 仍可,375px 强制 1-col |
| Modal 打开性能 | 60ms 内出现 (Radix Dialog 已保证) |
| Build 体积 | 与 V0.1 + V0.2 相比 +5KB CSS,+2KB JS (估算) |

## 7. 风险与缓解

| 风险 | 缓解 |
|---|---|
| Charter 字体加载失败 (Google Fonts 受限 / 离线) | system fallback 链: Charter → Georgia → Palatino → Times New Roman;离线时退化到 Georgia,编辑感保留 |
| 大量组件同时改易出回归 | 阶段化: 先 token (T-UI-001),再 ui 组件 (T-UI-002),再 app shell (T-UI-003),最后 modal 改 modal (T-UI-005) |
| 2-col 在 768px 拥挤 | `@media (max-width: 768px) { grid-template-columns: 1fr; }` 强制 1-col |
| Dark mode 对比度不足 | primary navy lightened 到 `#7da3d1`,已验证 AA;muted 用 `#8a8780` 替代 `#6b6b65` |
| 拖拽 affordance 没真功能 | 卡片保留 drag handle 视觉,但本期不接 dnd 库,鼠标 hover 不变;后续 V0.X 接 @dnd-kit 时再启用 |
| localStorage 写穿到 Tauri 之外 | key 命名 `keypilot.*` 隔离,无 PII |
| 现有用户的偏好丢失 | Theme 已有持久化,density 新增 key 不冲突;无破坏性 |
| Modal 焦点陷阱与 Esc 关闭 | 沿用 Radix Dialog 默认行为,不重写 |
| App 启动到首屏可见 | 不引新字体除 Charter,Charter 用 `display=swap` 不阻塞首屏 |
