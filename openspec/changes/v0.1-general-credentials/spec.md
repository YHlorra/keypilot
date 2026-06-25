# V0.1 General Credentials — Spec Delta

> **格式**: ADDED / MODIFIED / REMOVED。编号 `REQ-<DOMAIN>-<NNN>`。
> **Domain**: PROV(Provider/Adapter) / QUOTA(额度查询) / COPY(复制 UI) / SCHEMA(SQLite) / CAT(Category 分组) / VIS(Field Visibility) / THEME(主题)
> **Supersedes**: v0.1-spec-alignment REQ-PROV-001/002/005/006, REQ-SCHEMA-001(部分,扩 schema_version)
> **沿用不变**: REQ-PROV-003(is_preset 列), REQ-PROV-004(validate_key 实现), REQ-QUOTA-001/002/003, REQ-COPY-001, REQ-COPY-002

---

## ADDED

### REQ-CAT-001: Category 表(分组)

```sql
CREATE TABLE IF NOT EXISTS categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,                    -- "凭证"(默认不可删) / "数据库" / "开发" / 用户自定义
    is_default INTEGER NOT NULL DEFAULT 0, -- 1 = 默认 category(不可删除/重命名)
    sort_index INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 首次启动 seed 1 条
INSERT OR IGNORE INTO categories (id, name, is_default, sort_index, created_at, updated_at)
VALUES (1, '凭证', 1, 0, strftime('%s','now'), strftime('%s','now'));
```

- 约束:`is_default=1` 的 category 不可删除/重命名(UI 层 enforce,DB 层不强制)
- 约束:`providers.category_id` 不允许 NULL,FOREIGN KEY RESTRICT(删除 category 需先迁出 provider)
- 关系:1 个 provider → 1 个 category(flat 1:1,非 N:M)
- 拍板:2026-06-24(/think grill Q3)

### REQ-PROV-007: 5 个官方 preset seed

首次启动(`lib.rs::run()`)在 `categories(id=1, name='凭证')` 下 seed 5 条 provider 行:

| name | preset | pinned | 字段模板 |
|---|---|---|---|
| `OpenAI` | `openai` | 1 | base_url / api_key(默认 masked) / model |
| `DeepSeek` | `deepseek` | 1 | base_url / api_key / model |
| `Anthropic` | `anthropic` | 1 | base_url / api_key / model |
| `GitHub` | `github` | 1 | base_url / token / username / scopes |
| `PostgreSQL` | `postgres` | 0 | host / port / user / password / database |

- `preset` 列:nullable string,值为 `openai` / `deepseek` / `anthropic` / `github` / `postgres` / NULL(用户自定义)
- 触发:`meta.preset_seeded != "1"` 时 seed(沿用 v0.1-spec-alignment REQ-PROV-002)
- 完成后:`meta.preset_seeded = "1"`
- 用户删除 preset 后:不重建
- 拍板:2026-06-24(/think grill Q4)

### REQ-PROV-008: Provider 行 schema(v2 → v3)

```sql
-- providers 行存"凭证主表",字段下移到 provider_fields
CREATE TABLE IF NOT EXISTS providers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    preset TEXT,                              -- 'openai' | 'deepseek' | 'anthropic' | 'github' | 'postgres' | NULL
    is_preset INTEGER NOT NULL DEFAULT 0,    -- 沿用 v0.1-spec-alignment REQ-PROV-003
    category_id INTEGER NOT NULL,
    pinned INTEGER NOT NULL DEFAULT 0,
    notes TEXT,
    icon TEXT,
    icon_color TEXT,
    sort_index INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE RESTRICT
);
```

- `preset` vs `is_preset`:`preset` 是能力锚(决定走哪个 adapter / validate_key / fetch_quota),`is_preset` 是来源标记(决定是否显示"官方"徽章)。两者可独立。
- 改名/删除:沿用 v0.1-spec-alignment(允许重名 / uuid 区分,name 自由不强制唯一)
- 拍板:2026-06-24(/think grill Q1+Q4+Q5+Q6)

### REQ-VIS-001: provider_fields 表(任意 KV + visibility)

```sql
CREATE TABLE IF NOT EXISTS provider_fields (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id INTEGER NOT NULL,
    key TEXT NOT NULL,                       -- 'base_url' / 'api_key' / 'token' / 任意用户键
    value TEXT NOT NULL,                     -- V0.1 全部明文
    visibility TEXT NOT NULL DEFAULT 'visible',  -- 'visible' | 'masked'(V0.1 二态,'private' 推迟 V0.2)
    sort_index INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_provider_fields_provider ON provider_fields(provider_id, sort_index);
```

### REQ-VIS-002: visibility 二态语义(visible / masked)— **V0.1 不加密**

| 状态 | UI 默认 | 复制行为 | 落盘加密 |
|---|---|---|---|
| `visible` | 显示原文 | 直接复制 | ❌ 明文 |
| `masked` | 显示掩码(`sk-••••8mKp`) | 默认复制掩码版本,点 ◉ 切换为明文后复制明文 | ❌ 明文 |

- **V0.1 决策**:**不加密**(用户决策 2026-06-24,"不加密")
- 存储:明文 SQLite,依赖 Windows ACL + 用户 Windows 密码保护
- 默认:`visibility='visible'`(用户自己复制自己粘贴就行)
- preset seed 时,5 个 preset 的敏感字段(API key / token / password)默认 `visibility='masked'`(UI 掩码,落盘仍明文)
- 用户可逐字段切换 visible ↔ masked
- `private` 状态**不存在**(V0.1 spec 不引入)
- **V0.2 评估加密**(SQLCipher 全文件 / 主密码 / DPAPI / 字段级加密)— V0.1 不实现,需写 RFC
- 拍板:2026-06-24(/think grill Q2 + Q 再问"为什么需要加密"决策"不加密")

### REQ-THEME-001: Dark / Light / Follow System 三主题 — 基于 Radix UI Colors

```rust
// Tauri command: get/set theme
#[tauri::command]
fn get_theme() -> String { /* 'dark' | 'light' | 'auto' */ }
#[tauri::command]
fn set_theme(theme: String) -> Result<(), AppError> { /* 写 meta */ }
```

- **配色系统**:**Radix UI Colors**(`@radix-ui/colors`,https://www.radix-ui.com/colors),30+ 色阶 × 12 步 × light/dark 双套,WCAG 合规
- **不使用** Tailwind 默认色板(`tailwindcss/colors`)
- **不使用** docs/index.html 原自定义 `--bg: #000` / `--fg: #f0f0fa`(layout 阶段参考,color 不锁定)
- 存储:`meta.theme` key,值 `dark` / `light` / `auto`(默认 `auto` = 跟随系统 `prefers-color-scheme`)
- UI:webui CSS 变量按 `:root[data-theme='dark']` / `:root[data-theme='light']` 两套,JS 在启动时读 meta 决定初值,运行时监听 `matchMedia('(prefers-color-scheme: dark)')` 切 auto
- 切换按钮:titlebar 右侧加 theme 切换按钮(Dark ↔ Light),auto 模式隐藏按钮(系统决定)

#### 色阶总表(Stage 3 实施拍板,2026-06-24 锁定)

**Chrome(背景 / 文字 / 边框 / CTA)— gray + iris**

| 角色 | light mode | dark mode |
|---|---|---|
| App 背景 | `gray.1` (#fcfcfd) | `gray.1` (#111113) |
| Panel 背景(sidebar / modal) | `gray.2` (#f9f9fb) | `gray.2` (#191919) |
| Elevated(hover / active) | `gray.3` (#eff0f3) | `gray.3` (#222222) |
| Hover overlay | `gray.4` (#e7e8ec) | `gray.4` (#2a2a2a) |
| Primary text | `gray.12` (#1a1a1a) | `gray.12` (#eeeeee) |
| Secondary text | `gray.11` (#6f6f6f) | `gray.11` (#9b9b9b) |
| Muted text | `gray.10` (#828282) | `gray.10` (#7d7d7d) |
| Disabled text | `gray.9` (#8e8e8e) | `gray.9` (#6e6e6e) |
| Subtle border | `gray.6` (#d8dad9) | `gray.6` (#3a3a3a) |
| Regular border | `gray.7` (#cdcecd) | `gray.7` (#484848) |
| Strong border(hover / focus) | `gray.8` (#b9bbba) | `gray.8` (#606060) |
| Primary CTA / 焦点 | `iris.9` (#3e63dd) | `iris.9` (#3e63dd) |
| Focus ring | `iris.7` | `iris.7` |

**状态色 — grass / amber / red / ruby**

| 角色 | light mode | dark mode |
|---|---|---|
| Success / ok | `grass.9` (#46a758) | `grass.10` (#3d9650) |
| Warn | `amber.9` (#ffc53d) | `amber.10` (#ffba18) |
| Danger | `red.9` (#e5484d) | `red.10` (#dc3d43) |
| Critical(quota 燃尽等) | `ruby.9` (#ca2e31) | `ruby.10` (#c2272a) |

**Preset 徽章 — Option A(teal / indigo / orange / gray / cyan)**

| preset | Radix 色阶 | step | light hex | dark hex | 理由 |
|---|---|---|---|---|---|
| OpenAI | `teal` | 9 | #00a2a2 | #00d2d2 | 鲜绿调,AI 主题 |
| DeepSeek | `indigo` | 9 | #3e63dd | #3e63dd | 品牌蓝紫 |
| Anthropic | `orange` | 9 | #f76808 | #f76808 | 品牌橙 |
| GitHub | `gray` | 9 | #8e8e8e | #6e6e6e | 中性,dev 工具 |
| PostgreSQL | `cyan` | 9 | #00a2c7 | #4cb4d7 | 品牌蓝 |

- 决策依据:对比 mockup `docs/preset-badge-options.html` 4 套方案,A 在"品牌识别"与"视觉舒适度"间平衡最佳(2026-06-24 视觉验证)
- 徽章使用位置:Sidebar 状态点 / Detail eyebrow 文字色 / Pill 背景色 / Tray card 行点 + quota 数字

- 拍板:2026-06-24(/think grill Q8 + 用户反馈"用 Radix UI Colors 不用 Tailwind colors" + 视觉验证)

#### Stage 3 实现栈(2026-06-24 锁定)

| 维度 | 选型 | 备注 |
|---|---|---|
| Framework | React 18 + TypeScript | feature_list.json 已锁 |
| Build tool | Vite 5 | pnpm create vite webui --template react-ts |
| Server state | `@tanstack/react-query` v5 | feature_list.json 已锁 |
| Router | (暂不引入,单页足够) | Stage 4+ 评估 |
| **UI 组件** | **shadcn/ui CLI**(Radix Primitives + Tailwind utility) | 2026-06-24 锁定 |
| **颜色系统** | `@radix-ui/colors` | 已锁,见上表 |
| **Tailwind 角色** | 仅 utility classes(不引 `tailwindcss/colors`) | 与 shadcn 默认 token 解耦 |
| 必需 shadcn 组件 | Button / Dialog / DropdownMenu / Tooltip / Popover / Toast(Sonner) / ToggleGroup / Form | 按需 `npx shadcn@latest add` |
| theme override | `src/styles/globals.css` 中 `--background` / `--foreground` / `--primary` / `--card` / `--muted` / `--border` / `--destructive` 等覆盖为 `var(--gray-1)` / `var(--iris-9)` 等 Radix 值 | 详见 REQ-THEME-002 |

### REQ-THEME-002: shadcn theme override 接 Radix Colors

shadcn 默认 token 是 HSL `h s l` 三段格式,Radix Colors 是 hex 直接值。覆盖方法:

```css
/* src/styles/globals.css */
@import '@radix-ui/colors/black-alpha.css';
@import '@radix-ui/colors/gray.css';
@import '@radix-ui/colors/iris.css';
@import '@radix-ui/colors/grass.css';
@import '@radix-ui/colors/amber.css';
@import '@radix-ui/colors/red.css';
@import '@radix-ui/colors/ruby.css';
@import '@radix-ui/colors/teal.css';
@import '@radix-ui/colors/indigo.css';
@import '@radix-ui/colors/orange.css';
@import '@radix-ui/colors/cyan.css';

:root {
  /* shadcn token → Radix value */
  --background: var(--gray-1);
  --foreground: var(--gray-12);
  --card: var(--gray-2);
  --card-foreground: var(--gray-12);
  --popover: var(--gray-2);
  --popover-foreground: var(--gray-12);
  --primary: var(--iris-9);
  --primary-foreground: #ffffff;
  --secondary: var(--gray-3);
  --secondary-foreground: var(--gray-12);
  --muted: var(--gray-3);
  --muted-foreground: var(--gray-11);
  --accent: var(--gray-4);
  --accent-foreground: var(--gray-12);
  --destructive: var(--red-9);
  --destructive-foreground: #ffffff;
  --border: var(--gray-6);
  --input: var(--gray-6);
  --ring: var(--iris-9);
  /* 状态色 */
  --success: var(--grass-9);
  --warning: var(--amber-9);
  --critical: var(--ruby-9);
  /* preset 徽章 (按需 className 引用) */
}

/* dark theme (Stage 3 通过 :root[data-theme='dark'] 或 .dark class 切换) */
:root[data-theme='dark'] {
  --background: var(--gray-1);
  --foreground: var(--gray-12);
  /* ... 同结构,Radix mode-aware 切换 */
}
```

- 拍板:2026-06-24(/think Stage 3 栈决策 + 视觉验证)
- 实施:Stage 3 T3.5 落实,先 light mode,后 dark mode,最后 auto mode
- 验证:`pnpm tsc --noEmit` 通过 + 浏览器对比 docs/preset-badge-options.html 颜色一致

- 拍板:2026-06-24(/think grill Q8 + 用户反馈"用 Radix UI Colors 不用 Tailwind colors" + 视觉验证)

### REQ-PROV-009: test_connection 只在 3 LLM preset 启用

```rust
trait ProviderAdapter {
    fn can_test(&self) -> bool;  // OpenAI/DeepSeek/Anthropic=true, GitHub/Postgres=false
    async fn test_connection(&self, base_url: &str, api_key: &str) -> Result<(), ValidateError>;
}
```

- 5 个 preset adapter 实现 `test_connection`(沿用 v0.1-spec-alignment REQ-PROV-004 的 3 LLM 算法)
- GitHub / PostgreSQL adapter 只实现 `fetch_quota`(GitHub: rate limit / Postgres: pg_database_size;见 REQ-QUOTA-005/006)或返回 `QuotaError::Unsupported`,不实现 `test_connection`
- UI:`canTest = false` → "测试连通性" 按钮 disabled + tooltip "本类型暂不支持"
- 拍板:2026-06-24(/think grill Q5)

### REQ-QUOTA-005: GitHub fetch_quota(rate limit)

```
GET https://api.github.com/rate_limit
Authorization: token {api_key}

→ 200: { 
    resources: { core: { limit: 5000, used: 1800, remaining: 3200, reset: i64 } }
  }

QuotaSnapshot {
    total: Some(limit as f64),
    used: Some(used as f64),
    remaining: Some(remaining as f64),
    unit: "req",
    plan_name: Some("GitHub"),
    is_valid: true,
    source: QuotaSource::Api,
    fetched_at: now,
}
```

### REQ-QUOTA-006: PostgreSQL fetch_quota(database size)

```
SELECT pg_database_size(current_database());

→ 1 row, 1 col: size_in_bytes
→ bytes → GB: size_in_bytes / 1024^3

QuotaSnapshot {
    total: None,                              // PostgreSQL 无总额度
    used: Some(size_gb),
    remaining: None,
    unit: "GB",
    plan_name: Some(current_database()),
    is_valid: true,
    source: QuotaSource::Api,
    fetched_at: now,
}
```

- `total = None` 时 UI quota 列隐藏,改显示进度条形式
- 实施需 `tokio-postgres` 依赖(Stage 4 加入 Cargo.toml)

### REQ-QUOTA-DISPLAY-001: Detail + Tray 双视图 quota 显示(单一数据源)

**用户决策**(2026-06-24):"Detail 和托盘都能看" — 推翻原设计"Detail 不显示 quota"的隐含分工。

#### Detail 头部 quota 区(Detail 右上角)

```
┌─────────────────────────────────────────────────────────┐
│ 🔵 DeepSeek                          ⏱ 7 小时前  🔄    │
│    https://platform.deepseek.com    剩余: 6.14 CNY    │
└─────────────────────────────────────────────────────────┘
```

- **左半区**:provider logo + 名称 + 主 URL(可点击)
- **右半区 quota block**:
  - 上行:相对时间(`7 小时前` / `刚刚` / `从未刷新`) + 刷新按钮(圆形箭头图标,点击触发 `fetch_quota` IPC)
  - 下行:`剩余: X.XX UNIT` 或 `已用: X.XX / X.XX UNIT`(取决于 QuotaSnapshot 字段)
  - 单位由 `quota_cache.snapshot_json.unit` 决定(USD / CNY / req / GB)
- 颜色:success(grass.9) / warn(amber.9) / danger(red.9) / critical(ruby.9) — 沿用 REQ-THEME-001 状态色
- 进度可视化:若有 `total` 显示百分比环或进度条,无 `total` 只显示已用数字(PostgreSQL 场景)

#### Tray hover 卡 quota 区

- **保留**原设计:每行 pinned provider 名 + 右侧 quota 数字
- 数据源:**与 Detail 同一 `quota_cache` 表**(确保双视图一致)
- 刷新时机:Detail 刷新按钮触发后,前端 react-query invalidate → Tray 卡也自动重渲染

#### 边界场景

| 场景 | Detail quota block 行为 | Tray 卡行为 |
|---|---|---|
| Custom preset(`preset=NULL`) | **隐藏整块** | 不显示在 tray 卡(pinned 不显示) |
| Anthropic(preset=`anthropic`) | 显示"未支持 · OAuth 路径"占位文字 + 刷新按钮 disabled | 同 Detail |
| GitHub / PostgreSQL quota(V0.1 fetch_quota 启用,见 REQ-QUOTA-005/006) | 正常显示 | 正常显示 |
| 从未 fetch(`quota_cache` 无行) | 显示"从未刷新" + 刷新按钮 enabled | 同 Detail |
| 上次 fetch 失败(`is_valid=false`) | 显示红色"上次失败" + 重试按钮 | 显示 stale 数字 + 红点 |
| 自动刷新(V0.1) | **不实现**(`§3.3` 硬约束) | 不实现 |

#### 数据流

```
fetch_quota IPC → quota_cache 表写入
                → 前端 react-query staleTime(5min)
                → Detail quota block 重新渲染
                → Tray 卡 react-query 同步 invalidate
                → 两处一致显示
```

- 拍板:2026-06-24(用户截图参考 DeepSeek console UI + 用户反馈"主界面和托盘都能看")
- 实施:Stage 3 UI + Stage 4 quota 联动

### REQ-CAT-002: Category sidebar UI 行为

- sidebar 分组展示,每组可折叠(group-title 点击切换 `aria-expanded`)
- `is_default=1` 的 category(初始 seed 的"凭证")删除按钮 hidden,重命名按钮 disabled
- 用户新建 category:右下角 "+ 新建分类" 按钮 → modal 输入 name → INSERT categories
- 用户删除 category:group-title hover 时显示 `×` → 二次确认 → 删除(若该 category 仍有 provider,UI 弹迁移对话框,选目标 category 后才删)
- 拍板:2026-06-24(/think grill Q3)

---

## MODIFIED

### REQ-PROV-001 MODIFIED(原 `v0.1-spec-alignment` AI-only 范围)

**原**:
```
enum ProviderKind { OpenAI, Anthropic, DeepSeek, Custom }
3 个官方 preset seed(OpenAI / Anthropic / DeepSeek)
```

**改为**:
```
enum Preset { OpenAI, Anthropic, DeepSeek, GitHub, PostgreSQL, Custom }
5 个官方 preset seed(OpenAI / DeepSeek / Anthropic / GitHub / PostgreSQL)
Custom 不再是 enum 变体,而是 `preset = NULL` 的语义位
```

- 拍板:2026-06-24(/think grill Q1+Q4)
- 影响:`src-tauri/src/provider/mod.rs` match 6 分支(5 preset + Custom(NULL))

### REQ-SCHEMA-001 MODIFIED(schema_version 1 → 2 → 3)

```sql
-- v1 (旧) — base_url/api_key 硬列
CREATE TABLE providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  base_url TEXT NOT NULL,
  api_key TEXT NOT NULL,
  ...
);

-- v2 (v0.1-spec-alignment) — 加 is_preset 列
ALTER TABLE providers ADD COLUMN is_preset INTEGER NOT NULL DEFAULT 0;
UPDATE meta SET value = '2' WHERE key = 'schema_version';

-- v3 (本 change) — 重构为 categories + provider_fields
-- 注意:v3 不向前兼容(v1/v2 直接 DROP 重建,因为 V0.1 尚未发布,无真实用户数据)
DROP TABLE IF EXISTS quota_cache;
DROP TABLE IF EXISTS providers;
CREATE TABLE categories (...);
CREATE TABLE providers (...);  -- 不再有 base_url/api_key 列
CREATE TABLE provider_fields (...);
CREATE TABLE quota_cache (...);
UPDATE meta SET value = '3' WHERE key = 'schema_version';
INSERT OR IGNORE INTO categories (id, name, is_default, sort_index, created_at, updated_at)
VALUES (1, '凭证', 1, 0, strftime('%s','now'), strftime('%s','now'));
```

- 迁移策略:V0.1 尚未发布,**DROP + 重建**,不写 v1→v2→v3 兼容代码(避免范围爆炸)
- 若 Stage 1 后已有用户数据,需补 RFC 评估 v2→v3 兼容迁移
- 拍板:2026-06-24(/think grill 隐式决策,DROP 重建 = YAGNI)

### REQ-PROV-004 MODIFIED(validate_key 三态 → 五态)

**原**(3 LLM only):
| Provider | 端点 | 成功 |
|---|---|---|
| OpenAI | GET /v1/models | 200 |
| DeepSeek | GET /user/balance | 200 |
| Anthropic | POST /v1/messages max_tokens=1 | 200/201 |

**改为**(5 preset,GitHub/Postgres 无 validate_key):
| Preset | 端点 | 成功 | 失败 |
|---|---|---|---|
| OpenAI | GET /v1/models | 200 | 401/403 |
| DeepSeek | GET /user/balance | 200 | 401/403 |
| Anthropic | POST /v1/messages max_tokens=1 | 200/201 | 401/403,400=Ambiguous |
| GitHub | (no test_connection) | — | — |
| PostgreSQL | (no test_connection) | — | — |

- 拍板:2026-06-24(/think grill Q5)

### REQ-QUOTA-004 MODIFIED(quota 算法覆盖 5 preset)

**原**:3 LLM quota 算法(OpenAI / DeepSeek / Anthropic=Unsupported)

**改为**:5 preset quota
- OpenAI / DeepSeek / Anthropic(Unsupported):沿用 v0.1-spec-alignment REQ-QUOTA-001/002/003
- GitHub:REQ-QUOTA-005(rate_limit API)
- PostgreSQL:REQ-QUOTA-006(pg_database_size)

- 拍板:2026-06-24(/think grill Q4+Q6)

### REQ-COPY-003 ADDED(visibility-aware 复制)— **V0.1 二态版本**

**原**(v0.1-spec-alignment REQ-COPY-001):只复制明文 key 或 key+url 合并

**改为**(V0.1 不加密版,二态):
- `visibility='visible'`:直接复制 value,toast "已复制"
- `visibility='masked'`:默认复制掩码显示版本(用于外发 / 截屏场景),**用户点 ◉ 切换为明文后再复制明文**,toast "已复制明文"
- 无 `private` 状态(V0.1 不加密)

- 拍板:2026-06-24(/think grill Q2 衍生 + Q 再问决策"不加密")

---

## REMOVED

### REQ-PROV-006 REMOVED(原 AI-only 范围外 5 个 stub)

**移除内容**:
- ProviderKind 原计划的 5 种未来变体:`Gemini`, `Mistral`, `Moonshot`, `Zhipu`
- 这些的 `default_base_url` 表行

**理由**:YAGNI(沿用 v0.1-spec-alignment)。但本 change 进一步扩到 GitHub + PostgreSQL,这两个**有完整 adapter**(不止 stub)。

**影响**:
- `provider/mod.rs` match 6 分支(5 preset + Custom),其中 GitHub / PostgreSQL 各 30-50 行 adapter 实现
- `webui/src/lib/format.ts` 的 `PROVIDER_SCHEMAS` 改为 5 项 + Custom(Stage 3 实施时跟进)

### REQ-COPY-002 REMOVED(沿用 v0.1-spec-alignment,format dropdown 砍)

不变,沿用。

### REQ-IMPORT-001 REMOVED(导入/导出/同步 V0.2 推迟)

**移除**(从 V0.1 范围):
- 设置 → 导出 按钮
- 设置 → 导入 按钮
- 设置 → 同步策略 下拉

**保留**(UI 占位):
- 三项在 V0.1 设置 modal 中显示,disabled + tooltip "V0.2 推出"
- 不写 IPC / 不写 Rust 命令

**理由**:§3.3 不变(沿用 v0.1-spec-alignment)

**影响**:docs/index.html 设置 modal 的"数据"段 V0.1 改为只读状态

### REQ-OAUTH-001 REMOVED(OAuth template 砍)

**移除**(从 V0.1 范围):
- 新建凭证 modal 的"OAuth 凭据(client_id / client_secret / token)" 选项
- `TEMPLATES.oauth` 在 webui/src/lib/templates.ts 中的条目

**保留**:
- blank / llm / database 三种 template
- preset + template 都降级为"可选助手",modal 默认选 blank

**理由**:用户决策"给一个默认即可,正常来说都是默认"(Q9),"OAuth 用户用 blank 手动填"

**影响**:docs/index.html 的新增凭证 modal 模板下拉从 4 项变 3 项
