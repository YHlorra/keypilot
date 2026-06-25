# KeyPilot V0.1 — 开发实施计划 (dev/)

> 决策日: 2026-06-24
> 真相源: **本目录 (`keypilot-dev/`) 才是开发代码,`../PM思考工厂/keypilot/` 是方向文档,冲突时以 dev 为准**
> 项目代号: KeyPilot
> 仓库路径(本地): `E:\Desktop\workspace\keypilot-dev`
> 远端仓库(待创建): TBD

---

## 0. 一句话定位

**KeyPilot = cc-switch 的"凭证管理 + 额度查询"功能独立剥出版**。

技术栈: **Rust 1.92 + Tauri 2.11 + SQLite(明文 + Windows ACL) + React/TS**(V0.1 不做主密码 / 加密)。
核心边界: **不写任何 CLI 配置文件**(1Password 范式)。

---

## 0.5 文档体系 (本目录真相源)

| 文件 | 角色 |
|---|---|
| `AGENTS.md` ≡ `CLAUDE.md` | Agent 规则(Iron Rule) |
| `feature_list.json` | 可执行 feature 状态(真相源) |
| `PLAN.md` | Stage 深度规范(本文件) |
| `progress.md` | Session 连续性日志 |
| `session-handoff.md` | 正式 session 交接 |
| `init.sh` | 标准初始化 / 验证脚本 |
| `README.md` | 用户向 |

关系:`PLAN.md` = "我们要建什么" / `feature_list.json` = "正在建哪个" / `progress.md` = "上次建到哪" / `session-handoff.md` = "下次怎么接上"

---

## 1. 真相源规则(读这段先)

| 角色 | 路径 | 性质 |
|---|---|---|
| **方向 / 决策** | `E:\Desktop\workspace\PM思考工厂\keypilot\`(README.md / 技术方案.md / MVP-范围.md / 指导方案.md / 命名.md / 竞品分析.md / 架构图.md / codemap.md / 开源策略.md) | 思路、决策记录、设计方向 |
| **真相源** | `E:\Desktop\workspace\keypilot-dev\`(本目录) | 实际能跑、能编译、能装的代码 |

**冲突解决**:
- 当 PM 文档和 dev 代码冲突 → **以 dev 为准**,回写 PM 文档对齐
- 当 PM 文档说了而 dev 还没实现 → 视作 "TODO,待实现"
- 当 dev 实现超出 PM 文档 → 视作 "PM 文档需补"

**为什么这样分**:`PM思考工厂/CLAUDE.md` 死命令 "只记录想法,不生成代码"。dev 目录在 PM 工厂外,允许写代码。

---

## 2. V0.1 范围(从 PM 文档继承 + 必要调整)

> **2026-06-24 修订**:经 `/think` grill 决策,本节范围以 `openspec/changes/v0.1-general-credentials/` 为准(覆盖原 v0.1-spec-alignment 的 AI-only 范围)。变更摘要见 §0.5。

### 2.1 必做 ✅

| 功能 | 详细 | 状态 |
|---|---|---|
| Tauri 2 + Rust 跑通 | 脚手架 + 空窗口 | Stage 1 |
| SQLite + 通用凭证 schema | `rusqlite`,表 `meta` / `categories` / `providers` / `provider_fields` / `quota_cache`,schema v3 | Stage 1 |
| 字段级 opt-in 加密 | AES-256-GCM,`visibility='private'` 字段加密存储,默认 `visible` 明文 | Stage 1 |
| 5 家 Provider 预置 | OpenAI / DeepSeek / Anthropic / GitHub / PostgreSQL | Stage 1(seed) + Stage 2(adapter) |
| Category 分组 | sidebar 可折叠分组,default `凭证` 不可删,用户可新建/删除/重命名 | Stage 1(schema) + Stage 3(UI) |
| Provider CRUD | add / list / get / update / delete | Stage 2 |
| ProviderField CRUD | 加字段 / 改字段 / 删字段 / visibility 切换 | Stage 2 |
| test_connection | 3 LLM preset(OpenAI/DeepSeek/Anthropic),GitHub/Postgres 按钮 disabled | Stage 2 |
| 一键复制(visibility-aware) | visible 直接 / masked 需点 ◉ / private 触发解锁 toast | Stage 3 |
| 1Password 风格 UI + sidebar Category | 两栏布局 + 可折叠分组 | Stage 3 |
| 三主题 | Dark / Light / Follow System | Stage 3 |
| 额度查询 OpenAI | subscription + usage 算法 | Stage 4 |
| 额度查询 DeepSeek | `/user/balance` | Stage 4 |
| Anthropic 手动输入 | fetch_quota 返回 Unsupported,UI 走手动输入 | Stage 4 + Stage 7 |
| 额度查询 GitHub | `/rate_limit` | Stage 4 |
| 额度查询 PostgreSQL | `pg_database_size` | Stage 4 |
| 托盘常驻 | `tauri-plugin-system-tray` | Stage 5 |
| 打包 | GitHub Actions + tauri build | Stage 6 |
| SmartScreen 签名 | Azure Trusted Signing($10/月) | Stage 6 (申请在 Stage 1 后立即启动) |

### 2.2 必不做 ❌(从 PM 文档继承,硬约束)

- 写任何 CLI 配置文件(`~/.claude/settings.json` / `~/.codex/auth.json` / `~/.config/opencode/*` 等)
- 主密码 / Argon2id(Stage 1 用固定密钥占位,**Stage 2 必须评估升级 RFC**)
- 故障转移 / 代理 / 账号池
- MCP 管理
- 跨平台(V0.1 Win 优先)
- 自动刷新 / 低额度告警(V0.2)
- 导入 / 导出 / 同步(V0.2,V0.1 设置 modal 中显示 disabled + tooltip "V0.2 推出")
- OAuth preset / OAuth template(V0.1 不实现,用户用 blank template 手动填)

**`grep` 验证项(每次提交前,§3 + §10 派生)**:
- `grep "visibility TEXT NOT NULL DEFAULT 'visible'" src-tauri/src/database.rs` → 必须有
- `grep "value TEXT" src-tauri/src/database.rs` → 必须有(provider_fields.value)
- `grep "^aes-gcm" src-tauri/Cargo.toml` → 必须有(REQ-VIS-002)
- `grep -E "^argon2|^chacha20|ChaCha20Poly1305" src-tauri/Cargo.toml` → 必须空(Stage 1 不引 argon2)
- `grep -rn "fs::write" src-tauri/src/` → 出现的所有路径不在 `~/.claude/` / `~/.codex/` / `~/.config/opencode/` 下

---

## 3. Stage 路线(从 PM `指导方案.md` + `openspec/changes/v0.1-general-credentials/` 继承,本目录实际推进)

| Stage | 内容 | 文件清单 | 估计工时 | 状态 |
|---|---|---|---|---|
| **1** | Tauri 2 脚手架 + SQLite 数据层(schemas v3: categories + providers + provider_fields + quota_cache) | `Cargo.toml` / `tauri.conf.json` / `build.rs` / `main.rs` / `lib.rs` / `capabilities/default.json` / `database.rs` / `store.rs` / `error.rs` / `types.rs` | 1 天 | **进行中** (2026-06-24 启动) |
| 2 | Provider 模型 + 5 adapter (OpenAI/DeepSeek/Anthropic/GitHub/PostgreSQL) + CRUD | `provider/mod.rs` / `provider/adapter.rs` / `provider/{openai,deepseek,anthropic,github,postgres}.rs` / `services/provider.rs` / `commands/provider.rs` | 1-2 天 | 待启动 |
| 3 | UI 主窗口 — **shadcn/ui + Radix Colors + Radix Primitives + Tailwind utility**,3 themes,Detail+Tray 双 quota 显示 | `webui/src/{App.tsx,components/*,hooks/*,lib/*,types/*,styles/*}` | 1-2 天 | 待启动 |
| 4 | 余额查询 — 5 preset quota(3 LLM 算法 + GitHub rate_limit + Postgres pg_database_size) | `services/quota.rs` / `provider/{openai,deepseek,github,postgres}.rs::fetch_quota` / `commands/quota.rs` | 2-3 天 | 待启动 |
| 5 | 托盘常驻(hover 卡 + 右键菜单,Detail+Tray 双 quota 显示) | `tray.rs` / `commands/tray.rs` | 1-2 天 | 待启动 |
| 6 | 打包 + 签名 | `.github/workflows/release.yml` + Azure Trusted Signing | 1 天 | 待启动(签名申请 Stage 1 后立即提交) |
| 7 | Anthropic 手动输入额度(因 quota Unsupported) | UI modal + quota command 扩展 | 0.5 天 | 待启动 |
| 8 | README + 用户文档 | `README.md` 完善 + 截图 | 0.5 天 | 待启动 |
| 9 | V0.1 验收测试 | 13 项验收清单 | 1 天 | 待启动 |

**总工时估算**: 8-12 天(单人)。

**V0.1 spec 真源**:`openspec/changes/v0.1-general-credentials/` (proposal/spec/design/tasks,4 文件,~52KB)。本 §3 / §4 是其精简版,具体 REQ 引用查 spec.md。

---

## 4. Stage 1 详细任务(本次)

### 4.1 文件清单(10 个,沿 `openspec/changes/v0.1-general-credentials/design.md §9`)

```
keypilot-dev/
├── AGENTS.md              # Agent 规则 (Iron Rule) — 349 行 §0-13
├── CLAUDE.md              # Agent 规则副本(Iron Rule 同步)
├── README.md              # 用户向
├── PLAN.md                # Stage 深度规范(本文件)
├── progress.md            # Session 连续性日志
├── session-handoff.md     # 正式 session 交接
├── feature_list.json      # Feature 状态真相源
├── init.sh                # 标准初始化 / 验证脚本
├── openspec/              # 决策真源
│   └── changes/
│       ├── v0.1-spec-alignment/      (历史,11 REQ AI-only,部分沿用)
│       └── v0.1-general-credentials/ (现行,~52KB,20 REQ:11 ADDED + 5 MODIFIED + 4 REMOVED)
├── docs/
│   ├── index.html                     (布局参考,不锁 color)
│   └── preset-badge-options.html      (4 套 preset 配色对比,选 A:teal/indigo/orange/gray/cyan)
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── capabilities/
│   │   └── default.json
│   ├── icons/                (Stage 1 占位空目录,Stage 6 补)
│   └── src/
│       ├── main.rs
│       ├── lib.rs            (run() 启动链:setup_db → seed_default_category → seed_preset_providers → setup_window with theme)
│       ├── database.rs       (SQLite schema v3,5 张表: meta + categories + providers + provider_fields + quota_cache)
│       ├── store.rs          (AppState { db: Arc<Database> })
│       ├── error.rs          (AppError enum 11 分支 + Serialize for IPC,无 Encryption 分支)
│       └── types.rs          (Provider / ProviderField / Category / Visibility 二态 / Theme)
└── webui/                    (Stage 3 再写,Stage 1 仅占位)
    └── src/
```

> ❌ V0.1 **不写** `crypto.rs`(用户决策"不加密",V0.2 RFC 评估)

### 4.2 关键技术决定

| 维度 | 选型 | 备注 |
|---|---|---|
| 数据库 | `rusqlite` (bundled) | Tauri 内 tokio 跑,`tauri::async_runtime::spawn_blocking` 包同步调用 |
| 错误处理 | `thiserror` | 标准 Rust 模式 |
| 时间 | `chrono` | unix timestamp(秒) + ISO 8601 字符串 |
| 序列化 | `serde` + `serde_json` | QuotaSnapshot / Provider 都要 derive |
| 异步 | `tokio`(Tauri 自带) | 不需要 `full` features,只 `rt-multi-thread` + `macros` |
| Async trait | `async-trait = "0.1"` | ProviderAdapter trait(Stage 2 提前引入) |
| **加密** | **不引入**(V0.1 不加密,详见 §3.2 / `REQ-VIS-002`) | 不引 `aes-gcm` / `base64` / `rand` / `argon2` |
| Tauri 插件 | **本 Stage 不引入**(tray / clipboard 留给 Stage 2/3) | 减少首次编译时间 |

### 4.3 SQLite Schema v3(Stage 1 实现)

完整 DDL 走 `openspec/changes/v0.1-general-credentials/design.md §1.1`,关键摘要:

```sql
-- meta: 通用 key-value
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
INSERT OR IGNORE INTO meta (key, value) VALUES ('schema_version', '3');
INSERT OR IGNORE INTO meta (key, value) VALUES ('preset_seeded', '0');
INSERT OR IGNORE INTO meta (key, value) VALUES ('theme', 'auto');

-- categories: 分组
CREATE TABLE IF NOT EXISTS categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0,    -- 1 = default 凭证,不可删
    sort_index INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
INSERT OR IGNORE INTO categories (id, name, is_default, sort_index, created_at, updated_at)
VALUES (1, '凭证', 1, 0, strftime('%s','now'), strftime('%s','now'));

-- providers: 凭证主表(已无 base_url/api_key 列,字段下移到 provider_fields)
CREATE TABLE IF NOT EXISTS providers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    preset TEXT,                              -- 'openai'|'deepseek'|'anthropic'|'github'|'postgres'|NULL
    is_preset INTEGER NOT NULL DEFAULT 0,
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
CREATE INDEX idx_providers_category ON providers(category_id, sort_index);
CREATE INDEX idx_providers_preset ON providers(preset);

-- provider_fields: 任意 KV + visibility 二态(无加密)
CREATE TABLE IF NOT EXISTS provider_fields (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id INTEGER NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,                                       -- V0.1 全部明文
    visibility TEXT NOT NULL DEFAULT 'visible',                -- 'visible' | 'masked'(二态,'private' 推迟 V0.2)
    sort_index INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
);
CREATE INDEX idx_pf_provider ON provider_fields(provider_id, sort_index);

-- quota_cache: 最近一次额度查询缓存
CREATE TABLE IF NOT EXISTS quota_cache (
    provider_id INTEGER PRIMARY KEY,
    snapshot_json TEXT NOT NULL,
    fetched_at INTEGER NOT NULL,
    FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
);
```

**精简原则**: V0.1 不引入 AuthState / 加密派生密钥 / 内存 quota cache — 全部走 SQLite 单一数据源。
**Migration 策略**: V0.1 未发布,DROP + 重建 v3(无真实用户数据兼容负担)。若 Stage 1 后已有数据,需 RFC 评估 v2→v3 兼容。

### 4.4 AppState 形态

```rust
// store.rs
use std::sync::Arc;
use crate::database::Database;

pub struct AppState {
    pub db: Arc<Database>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        Self { db: Arc::new(db) }
    }
}
```

### 4.5 数据库路径(Windows)

```
%APPDATA%\com.keypilot.app\keypilot.db
```

Tauri 提供 `app.path().app_data_dir()`(Windows 上 = `%APPDATA%\<identifier>\`)。Stage 1 用 `tauri::Manager::path()` API 取这个路径。

### 4.6 验收(Stage 1 = Done)

**完整验证走 [`./init.sh`](./init.sh) (Sprint Contract 入口,见 AGENTS.md §10)。**

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` 通过
- [ ] `cargo build --manifest-path src-tauri/Cargo.toml` 通过(release 也可,debug 默认即可)
- [ ] `grep "visibility TEXT NOT NULL DEFAULT 'visible'" src-tauri/src/database.rs` 有结果
- [ ] `grep "value TEXT" src-tauri/src/database.rs` 有结果(provider_fields.value)
- [ ] `grep "preset TEXT" src-tauri/src/database.rs` 有结果(preset 列)
- [ ] `grep "category_id INTEGER NOT NULL" src-tauri/src/database.rs` 有结果(category_id FK)
- [ ] `grep -E "argon2|chacha20|ChaCha20Poly1305|aes-gcm" src-tauri/Cargo.toml` 为空(V0.1 不加密)
- [ ] `grep -r "std::fs::write" src-tauri/src/` 出现的所有路径不在 `~/.claude/` / `~/.codex/` / `~/.config/opencode/` 下
- [ ] 启动一次(空窗口),退出,SQLite 文件落在 `%APPDATA%\com.keypilot.app\keypilot.db`
- [ ] `sqlite3 "%APPDATA%\com.keypilot.app\keypilot.db" ".tables"` 返回 5 张表(meta / categories / providers / provider_fields / quota_cache)
- [ ] `SELECT * FROM meta WHERE key='schema_version';` 返回 `'3'`
- [ ] `SELECT count(*) FROM categories;` = 1(default `凭证`)
- [ ] `SELECT count(*) FROM providers WHERE is_preset=1;` = 5(OpenAI / DeepSeek / Anthropic / GitHub / PostgreSQL)
- [ ] 重启应用,`preset_seeded=1` 仍生效,preset 不重建

### 4.7 风险 / 阻塞

| 风险 | 缓解 |
|---|---|
| agent-desktop-plus 脚手架不完整(无 `lib.rs` / `build.rs` / `icons` / `webui`) | dev/ 目录从 Tauri 2 标准模板搭,不依赖 PM 文档的 "复制 agent-desktop-plus" 建议 |
| Windows 工具链问题(Visual Studio Build Tools / WebView2) | 用户的 Windows 11 已装 tauri-cli 2.11,先假定工具链齐;Stage 6 打包时再验 |
| SmartScreen 签名未到位 | Stage 1 完成后**立即申请** Azure Trusted Signing(1-3 天审批,不阻塞 Stage 2-5 开发) |
| rusqlite bundled 编译时间长 | 接受,Tauri 项目普遍 1-2 分钟首次编译 |

---

## 5. 进度跟踪

**可执行状态见 [`feature_list.json`](./feature_list.json)**,本节是 stage 路线总览。

- [ ] Stage 1: Tauri 2 + SQLite 数据层 (本次,2026-06-24 启动)
- [ ] Stage 2: Provider 模型 + CRUD
- [ ] Stage 3: UI 主窗口
- [ ] Stage 4: 余额查询
- [ ] Stage 5: 托盘常驻
- [ ] Stage 6: 打包 + 签名
- [ ] Stage 7: 手动输入额度
- [ ] Stage 8: README + 用户文档
- [ ] Stage 9: V0.1 验收

---

## 6. 与 PM 工厂的对接

- **写代码** → 在 `keypilot-dev/`
- **写设计 / 决策 / 反思** → 在 `PM思考工厂/keypilot/`
- **状态不一致** → 同步两边;以 dev 为准

---

*最后更新: 2026-06-24 (Stage 1 启动)*
