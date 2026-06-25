# KeyPilot V0.1 — Agent Rules

> 适用对象: 所有在 `keypilot-dev/` 工作的 Agent / 开发者 / 协作者
> 真相源分离: PM思考工厂/keypilot/ = 方向 / keypilot-dev/ = 代码,冲突时 dev 赢

---

## 0. Iron Rule: AGENTS.md ≡ CLAUDE.md

**AGENTS.md 和 CLAUDE.md 必须是内容完全相同的两个文件。**

不同 Agent 读取不同文件名:
- 部分 Agent 读 `AGENTS.md`(OpenCode / Codex / Cursor 等)
- 部分 Agent 读 `CLAUDE.md`(Claude Code 等)
- 两份必须内容一致

修改时同步:

```bash
# 修改 AGENTS.md 后必须立即同步 CLAUDE.md
cp AGENTS.md CLAUDE.md
```

**本文件模板同时用于两份,绝不创建内容不同的副本。**

---

## 1. 启动流程 (Startup Workflow)

写代码前必须完成:

1. **确认工作目录**:`pwd`(应落在 `keypilot-dev/` 内)
2. **完整读取 AGENTS.md**(与 CLAUDE.md 相同)
3. **读取 `docs/`** 子目录详细引用(如存在)
4. **运行 `./init.sh`** 验证环境健康
5. **读取 `feature_list.json`** 查看当前 feature 状态
6. **读取 `progress.md`** 看上次 session 交接
7. **审查最近提交**:`git log --oneline -5`(若 git 已初始化)

若 `./init.sh` 验证失败,先修复再添加新 scope。

---

## 2. 真源规则 (Truth Source)

| 角色 | 路径 | 性质 |
|---|---|---|
| **方向 / 决策** | `E:\Desktop\workspace\PM思考工厂\keypilot\`(README.md / 技术方案.md / MVP-范围.md / 指导方案.md / 命名.md / 竞品分析.md / 架构图.md / codemap.md / 开源策略.md) | 思路、决策记录、设计方向 |
| **真相源** | `E:\Desktop\workspace\keypilot-dev\`(本目录) | 实际能跑、能编译、能装的代码 |

**冲突解决**:
- PM 文档和 dev 代码冲突 → **以 dev 为准**,回写 PM 文档对齐
- PM 文档说了但 dev 还没实现 → 视作 "TODO,待实现"
- dev 实现超出 PM 文档 → 视作 "PM 文档需补"

`PM思考工厂/CLAUDE.md` 死命令 "只记录想法,不生成代码"。dev 目录在 PM 工厂外,允许写代码。

---

## 3. 硬约束 (V0.1 不可破,违反 = 设计失败)

### 3.1 不写 CLI 配置文件

**禁止**对以下路径做任何写操作:

- `~/.claude/settings.json`
- `~/.claude/projects/**/*.jsonl`(只读,V0.2 token 统计用)
- `~/.codex/auth.json`
- `~/.codex/sessions/**/*.jsonl`(只读)
- `~/.config/opencode/**`
- `~/.local/share/opencode/**`
- 任何 `*claude*` / `*codex*` / `*opencode*` 配置目录

**验证**(每次提交前):

```bash
# 应在 src-tauri/src/ 下执行,所有 std::fs::write 路径必须在白名单内
grep -rn "fs::write\|fs::create_dir_all" src-tauri/src/
```

`fs::write` 出现的所有路径必须落在:
- `%APPDATA%\com.keypilot.app\`
- `%LOCALAPPDATA%\com.keypilot.app\`
- `webui/dist/`(build 产物)
- 临时目录(用户明示)

### 3.2 明文存储 (V0.1 决策,2026-06-24 更新)

**V0.1 不引入加密**(用户决策 /think grill Q 再问"为什么需要加密"):

- `Cargo.toml` 不引 `argon2` / `chacha20poly1305` / `aes-gcm` / `sodiumoxide` / `age` 等加密 crate
- SQLite `provider_fields.value` 列 = `TEXT NOT NULL`(明文,V0.1 全部)
- `visibility` 二态:`visible` / `masked`(UI 掩码,落盘仍明文)
- 依赖 Windows ACL(`%APPDATA%` 默认只对当前用户可读)+ 用户用强 Windows 密码
- 详细 spec:`openspec/changes/v0.1-general-credentials/spec.md REQ-VIS-002`

**验证**:

```bash
# 应为空
grep -E "argon2|chacha20|ChaCha20Poly1305|aes-gcm|sodiumoxide|^age " src-tauri/Cargo.toml

# 必须有(V0.1 schema v3)
grep "visibility TEXT NOT NULL DEFAULT 'visible'" src-tauri/src/database.rs
grep "value TEXT" src-tauri/src/database.rs  # provider_fields.value 列
```

**V0.2 评估升级**(2026-06-24 推到 V0.2 RFC):
- 三选一:SQLCipher 全文件加密 / 主密码 + argon2id / Windows DPAPI
- 评估前先在 `PM思考工厂/keypilot/技术方案.md` 写 RFC
- 不要在 V0.1 偷偷加加密(保持决策纯度)

### 3.3 边界 (从 PM 文档继承,2026-06-24 更新)

V0.1 明确不做:

- 故障转移 / 代理 / 账号池
- MCP 管理
- 跨平台(Mac/Linux 留给 V0.3+)
- 自动刷新 / 低额度告警(留给 V0.2)
- 加密(明文存储,V0.2 RFC 评估 SQLCipher / 主密码 / DPAPI)
- 导入 / 导出 / 同步(留给 V0.2)
- 加密同步(留给 V0.3+)
- OAuth template / OAuth preset(留给 V0.2 评估)
- i18n(留给 V0.2)
- 团队协作 / 多 Vault(永久不做)
- 浏览器扩展(永久不做)

**V0.1 已做(2026-06-24 不再"不做")**:
- ~~主题切换~~ → **V0.1 已做**:Dark / Light / Follow System 三主题,Radix UI Colors,见 §3.4
- ~~3 个 LLM preset only~~ → **V0.1 已扩**:5 preset(OpenAI / DeepSeek / Anthropic / GitHub / PostgreSQL),见 `openspec/changes/v0.1-general-credentials/spec.md REQ-PROV-007`
- ~~无 Category~~ → **V0.1 已做**:categories 表 + sidebar 可折叠分组,见 `openspec/changes/v0.1-general-credentials/spec.md REQ-CAT-001`

### 3.4 Stage 3 UI 栈 (2026-06-24 锁)

V0.1 `webui/` **必须**使用:

- **shadcn/ui CLI**(`npx shadcn@latest add ...`)— Radix Primitives + Tailwind utility classes
- **@radix-ui/colors** 作为色阶系统(`var(--gray-1)` / `var(--iris-9)` 等)
- React 18 + TypeScript + Vite 5 + TanStack Query v5(沿用 `feature_list.json`)

**禁止**:

- 用 Tailwind 默认 colors(`tailwindcss/colors`)— shadcn 默认 HSL token 必须 override 为 `var(--gray-*)` / `var(--iris-9)` 等 Radix 直接值
- 不用 Radix UI Themes(@radix-ui/themes 是 opinionated,与 docs/index.html brutalist 风格冲突)

详细:`openspec/changes/v0.1-general-credentials/spec.md REQ-THEME-002` + `docs/preset-badge-options.html`(preset 徽章对比)

---

## 4. 目录与命名规范

### 4.1 目录

```
keypilot-dev/
├── AGENTS.md              # 本文件
├── CLAUDE.md              # 本文件副本(Iron Rule,见 §0)
├── README.md              # 用户向
├── PLAN.md                # 实施计划(Stage 深度规范)
├── progress.md            # Session 连续性日志
├── session-handoff.md     # 正式 session 交接
├── feature_list.json      # Feature 状态真相源
├── init.sh                # 标准初始化 / 验证脚本
├── .gitignore
├── docs/                  # 额外文档
├── src-tauri/             # Rust 后端 + Tauri 配置
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── capabilities/
│   │   └── default.json
│   ├── icons/             # Stage 6 补
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── database.rs
│       ├── store.rs
│       ├── error.rs
│       ├── provider/      # Stage 2+
│       ├── services/      # Stage 2+
│       ├── commands/      # Stage 2+
│       └── tray.rs        # Stage 5
└── webui/                 # V0.1 Stage 3 起步
    ├── package.json
    ├── tsconfig.json
    ├── vite.config.ts
    ├── index.html
    └── src/
        ├── main.tsx
        ├── App.tsx
        ├── components/
        ├── hooks/
        └── lib/
```

### 4.2 命名

- **Rust**: snake_case 文件名、snake_case 函数、PascalCase struct/enum、SCREAMING_SNAKE_CASE const
- **TypeScript/React**: PascalCase 组件、camelCase 函数/变量、`*.tsx` 用于 JSX
- **Tauri 命令**: `verb_noun` 形式,跨语言一致(`list_providers` / `add_provider` / `fetch_quota`)

### 4.3 错误处理

- Rust 用 `thiserror` 定义 `AppError` enum,所有 fallible 函数返回 `Result<T, AppError>`
- Tauri command 把 `AppError` 序列化成 `{ code, message }` JSON
- 前端用 TanStack Query 的 `error` 字段拿到,统一 toast 展示

---

## 5. 工具链

- **Rust**: 1.92 nightly(已装)
- **Node**: 22.22(已装)
- **pnpm**: 11.0.9(已装)
- **tauri-cli**: 2.11.0(已装)
- **WebView2**: 假设 Win11 自带;若 Stage 6 打包失败再装
- **MSVC Build Tools**: 假设已装;若 cargo check 报 link 错误再装

---

## 6. 开发循环 (Workflow)

```bash
# 跑 dev(热重载)
cd keypilot-dev
pnpm tauri dev

# Rust 单测
cargo test --manifest-path src-tauri/Cargo.toml

# Rust 编译检查
cargo check --manifest-path src-tauri/Cargo.toml

# TS 类型检查
cd webui && pnpm tsc --noEmit

# 验证硬约束(提交前)
grep -E "argon2|chacha20|ChaCha20Poly1305|aes-gcm" src-tauri/Cargo.toml  # 应空(V0.1 不加密)
grep -rn "fs::write" src-tauri/src/  # 路径应在白名单
grep "visibility TEXT NOT NULL DEFAULT 'visible'" src-tauri/src/database.rs  # schema v3 必有
grep "value TEXT" src-tauri/src/database.rs  # provider_fields.value 必有
```

完整 init/verify 走 `./init.sh`,见 §10。

---

## 7. 提交规范

每个 Stage 完成的提交,信息格式:

```
<Stage> <一句话总结>

- 文件清单: <列出>
- 验证: <cargo check / cargo test / cargo build 结果>
- 硬约束: <grep 验证结果>
- 关联 PM 文档: <../PM思考工厂/keypilot/对应 .md 同步状态>
```

例:

```
Stage 1: Tauri 2 脚手架 + SQLite 数据层

- 文件清单: Cargo.toml / tauri.conf.json / build.rs / main.rs / lib.rs / capabilities/default.json / database.rs / store.rs / error.rs
- 验证: cargo check 通过
- 硬约束: api_key TEXT 明文确认 / 无加密 crate / 无 CLI 写路径
- 关联 PM 文档: PM思考工厂/keypilot/技术方案.md §6 同步
```

---

## 8. 状态工件 (State Artifacts)

| 文件 | 角色 | 更新时机 |
|---|---|---|
| `feature_list.json` | Feature 状态真相源 | 每完成 / 开始一个 feature 立即更新 |
| `progress.md` | Session 连续性日志 | 每个 session 至少更新一次 |
| `session-handoff.md` | 正式 session 交接 | session 结束时生成 / 更新 |
| `PLAN.md` | Stage 深度规范(文件清单 / schema / 验收) | Stage 详情变更时更新 |

**运行模式**:PLAN.md 是"我们要建什么",`feature_list.json` 是"我们正在建哪个",`progress.md` 是"上次建到哪",`session-handoff.md` 是"下次怎么接上"。

**Schema 偏差**:`feature_list.json` 使用 `stages` / `stage-N` / `depends_on` 命名(对齐 PLAN.md 9-Stage 路线),**有意**偏离 hybrid-harness schema(`features` / `feat-N` / `dependencies`)。`status` 枚举和 `evidence` 字段语义与 hybrid-harness 一致。

---

## 9. 范围 (Scope) — 一次一个 Stage

- **一次一个 Stage**:从 `feature_list.json` 选一个 `status != "done"` 的 feature
- **不修改范围外文件**:与当前 feature 无关的文件不碰
- **Stage DoD**(Definition of Done):
  - [ ] PLAN.md 中该 Stage 列出的文件清单全部创建
  - [ ] `./init.sh` 全绿
  - [ ] 硬约束 grep 全部通过(见 §10.2)
  - [ ] `feature_list.json` 中该 feature `status = "done"` + `evidence` 字段记录验证命令输出
  - [ ] `progress.md` 记录本 Stage 完成项
  - [ ] 若 PM 文档有相关章节,同步状态

---

## 10. 验证 (Verification) — Sprint Contract

每个 Stage 完成前**必须**全部通过(防 Agent 自评幻觉):

### 10.1 编译 / 类型

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` 通过
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` 通过(有 test 时)
- [ ] `cd webui && pnpm tsc --noEmit` 通过(`webui/` 存在时)

### 10.2 硬约束 grep

- [ ] `grep "visibility TEXT NOT NULL DEFAULT 'visible'" src-tauri/src/database.rs` 有结果(schema v3)
- [ ] `grep "value TEXT" src-tauri/src/database.rs` 有结果(provider_fields.value)
- [ ] `grep "preset TEXT" src-tauri/src/database.rs` 有结果(preset 列)
- [ ] `grep "category_id INTEGER NOT NULL" src-tauri/src/database.rs` 有结果(category_id FK)
- [ ] `grep -E "argon2|chacha20|ChaCha20Poly1305|aes-gcm|sodiumoxide|^age " src-tauri/Cargo.toml` 为空(无加密 crate)
- [ ] `grep -rn "fs::write" src-tauri/src/` 所有路径不在 CLI 配置白名单外(§3.1)

### 10.3 行为

- [ ] 启动一次应用(Stage 1 = 空窗口 + SQLite 文件创建)
- [ ] 关键功能手动验证(按 Stage 验收清单)

### 10.4 文档

- [ ] `feature_list.json` status 更新 + evidence 字段非空
- [ ] `progress.md` 记录本 session 完成项
- [ ] PM 工厂对应章节状态同步(若适用)
- [ ] 提交信息按 §7 格式

### 10.5 反自欺清单

- [ ] 没有 `.skip` / `.todo` / `unimplemented!()` 残留
- [ ] 没有"应该可以工作"但没跑过的代码路径
- [ ] 没有"我跑过测试"但实际是 `cargo check` 的谎称

---

## 11. 求助顺序 (Escalation)

出问题时的顺序:

1. 查 `PLAN.md` 当前 Stage 的文件清单
2. 查 `session-handoff.md` / `progress.md` 看上次卡在哪
3. 查 `../PM思考工厂/keypilot/codemap.md` 看参考项目对应章节
4. `ctx_search(source: "cc-switch-src-tauri", queries: [...])` 查 knowledge base
5. 派 `oracle` 评审架构 / 派 `librarian` 查库文档
6. 问用户(避免在 debug 黑洞里转太久)

---

## 12. Session 结束流程 (End-of-Session)

结束 session 前**必须**:

1. 更新 `progress.md`(本 session 完成项 / 进行中 / 风险 / 下一步)
2. 更新 `feature_list.json`(本 session 涉及的 feature 状态)
3. 更新 `session-handoff.md`(本 session 正式交接)
4. 同步 AGENTS.md → CLAUDE.md(若有变更)
5. 提交并写描述性 message(若 git 已初始化)
6. 留下足够干净的状态,下次 session 可直接 `./init.sh` 跑起来

---

## 13. 反重复门(Anti-Duplication Gate)

参考 hybrid-harness `references/gotchas.md` §16,KeyPilot 增量规则:

### 13.1 当前 (V0.1 单进程 Rust + Tauri)

- [ ] 无 `.skip` / `.todo` / `unimplemented!()` 残留
- [ ] 没有并行声明的同义类型(Rust struct ↔ TS interface 必须先在 `src-tauri/src/types.rs` 定义,前端 import,不重复)
- [ ] Provider Command(`list_providers` / `add_provider` / 等)合并前已在 PLAN.md Stage N 文件清单列出(无 scope creep)

### 13.2 未来 (V0.2+ IPC 通道增多)

- [ ] 新 IPC 命令先在 PLAN.md / INTEGRATIONS.md 登记,后写代码
- [ ] 跨进程类型(`src-tauri/src/` ↔ `webui/src/types/`)单一来源,无重复

---

*最后更新: 2026-06-24(hybrid-harness overlay 落地,Stage 1 进行中)*