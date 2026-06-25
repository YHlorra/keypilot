# UI Redesign (Kaku Design System) : Proposal

## Problem

KeyPilot 的 webui 当前用 shadcn 默认的 Radix Colors (iris/gray) 调色板,视觉语言是通用 SaaS 仪表盘。三个具体问题:

1. **视觉语言不一致**: 标题用 sans,无 serif 编辑感; 按钮方角,无 pill 形态; 主色 iris `#5b5bd6` 与 Kaku 编辑调性不符
2. **信息架构不匹配**: 侧边栏 (300px) 强制占据主空间,凭证列表的密度被压扁;Detail Pane 信息密度低,无法一眼看一组凭证
3. **主题感弱**: Auto/Light/Dark 主题色值混乱,Dark 模式只是简单反色,没有 coherent inversion
4. **缺乏编辑感**: 凭证是高价值、低频信息,值得用印刷品质感呈现,而不是 dashboard

## Proposed Solution

把 Kaku 设计系统 (`DESIGN.md`) 应用到 keypilot webui。视觉已通过 `webui/dist/design-preview.html` v2 验证,用户已批准。

| 维度 | 旧 (v1) | 新 (v2) |
|---|---|---|
| 主色 | `#5b5bd6` (iris) | `#1b365d` (navy) |
| 画布 | `#fcfcfc` (cool gray) | `#f5f4ed` (warm off-white) |
| 标题字 | system sans | Charter serif (Google Fonts) |
| 圆角 | `0.5rem` 方角 | `8px` (card) / `999px` (pill) |
| 布局 | 侧边栏 + Detail Pane | Card Grid (1-col / 2-col density toggle) |
| Detail | 永久 Pane | Modal (按需打开) |
| 主题 | 3 主题但色值混乱 | 3 主题 + coherent dark inversion |
| 分隔 | shadow | 1px border + background step |
| Titlebar | 🔑 emoji + ⚙️ emoji | 纯 serif "KeyPilot" + SVG icon |

### 视觉验证

`webui/dist/design-preview.html` 是 v1 + v2 的可视化原型,作为本次 spec 的实现目标。代码改动落地后,`webui/dist/` 的 build 产物应与该 preview 在 1280 / 768 / 375 三档 viewport 下一致。

## Scope

**本 change 包含**:

- **Token 替换**: `webui/src/styles/globals.css` + `webui/tailwind.config.ts` 切到 Kaku 色板 / 字阶 / 间距 / 圆角
- **布局重做**: 删 `CategorySidebar.tsx` + `ProviderList.tsx`,改用 `ProviderCard` + `ProviderGrid` + `DensityToggle`
- **新增组件**: `ProviderCard`, `ProviderGrid`, `DensityToggle`, `ProviderDetailModal`, `TopBar`, `SectionLabel`, `ChipGroup`
- **Detail 改 Modal**: `ProviderDetail.tsx` 拆为 modal form,加 `ProviderDetailModal.tsx` 容器
- **主题**: 保留 3 主题 (Auto / Light / Dark),dark 变体做 coherent inversion
- **凭证卡**: drag handle + provider icon + name + URL (link blue) + clock + time + refresh + quota (success green)
- **字体**: Charter via Google Fonts;system sans 作为 body;ui-monospace 作为 masked value
- **emoji 清除**: 移除 🔑 和 ⚙️,改 lucide-react SVG

**本 change 不包含**:

- 营销 landing 页面 / hero / 营销 section
- 新增 IPC 命令
- 新增 SQLite schema
- 新增 feature (quota 历史、usage 统计等,那些是 `token-usage-history` change 的范围)
- macOS 平台样式迁移 / Liquid Glass
- 加密
- 拖拽重排 (drag handle 仅作 affordance,接 dnd 库留给后续 V0.X)
- 多语言 i18n (中文 / 英文混排是 V0.1 现状,本 change 不动)

## Why Now

- V0.1 凭证库 (stage-1 ~ stage-9) + V0.2 search (stage-11) 完成
- 用户已在 v2 preview 上确认设计方向
- Token 替换是低风险、纯 UI,无后端依赖,可单独迭代和回滚
- 下个 V0.3 (`token-usage-history`) feature 落地前应用新设计系统,避免双轨 UI

## 关联 (Cross-references)

- 设计系统真相源: `DESIGN.md` (kaku)
- 视觉验证: `webui/dist/design-preview.html` (v2)
- 后续 openspec: `openspec/changes/token-usage-history/` (V0.3 用量统计,沿用本 change 的设计系统)
- 当前 webui 状态: `webui/src/` (V0.1 + V0.2 已落,V0.2 search 集成)
