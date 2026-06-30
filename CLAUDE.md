# KeyPilot V0.1 — Agent Rules

> 适用对象: 所有在 `keypilot-dev/` 工作的 Agent / 开发者 / 协作者
> 唯一真相源: 本仓库 `keypilot-dev/`

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

## 2. 唯一真相源

本仓库 `keypilot-dev/` 是代码与文档的唯一真相源。所有规格、决策、参考都在仓库内 (`docs/`、`openspec/changes/archive/`、`feature_list.json`、`progress.md`、`session-handoff.md`、`.slim/deepwork/`)。仓库外无真相源,不与外部文档同步。

---

## 3. 硬约束 (V0.1 不可破,违反 = 设计失败)

### 3.1 不写 CLI 配置文件

**禁止**对以下路径做任何写操作:`~/.claude/` `~/.codex/` `~/.config/opencode/` `~/.local/share/opencode/` 及任何 `*claude*`/`*codex*`/`*opencode*` 目录。`fs::write` 必须落在 `%APPDATA%\com.keypilot.app\`(默认 current user 可读)、`%LOCALAPPDATA%\com.keypilot.app\`、`webui/dist/`、或用户明示临时目录。完整 grep gate 见 `init.sh §4`。

### 3.2 明文存储

V0.1 不引入加密 (`provider_fields.value` 明文,`visibility` 二态 `visible`/`masked` UI 掩码不影响落盘;依赖 Windows ACL + 强密码)。`Cargo.toml` 不引 `argon2` / `chacha20poly1305` / `aes-gcm` / `sodiumoxide` / `age`。详细 spec: `openspec/changes/v0.1-general-credentials/spec.md REQ-VIS-002`。V0.2 评估: SQLCipher / master-password argon2id / DPAPI 三选一,先在 `openspec/changes/v0.2-encryption/` 写 RFC。

### 3.3 Stage 3 UI 栈

V0.1 `webui/` 必须: shadcn/ui CLI (Radix Primitives + Tailwind utility, 默认 HSL token 必须 override 为 `var(--gray-*)` / `var(--iris-9)` 等 Radix 直接值) + @radix-ui/colors 色阶 + React 18 + TypeScript + Vite 5 + TanStack Query v5。**禁止** Tailwind 默认 colors 与 Radix UI Themes (与 docs/index.html brutalist 风格冲突)。详细: `openspec/changes/v0.1-general-credentials/spec.md REQ-THEME-002` + `docs/preset-badge-options.html`。

---

## 4. 目录与命名规范

### 4.1 目录

```
keypilot-dev/
├── AGENTS.md / CLAUDE.md   # 本文件 + 副本 (Iron Rule §0)
├── README.md               # 用户向
├── PLAN.md                 # Stage 深度规范
├── progress.md / session-handoff.md   # Session 连续性 + 交接
├── feature_list.json       # Feature 状态真相源
├── init.sh                 # 验证脚本 (§10)
├── .slim/deepwork/         # Orchestrator 工作笔记 (gitignore)
├── docs/                   # 公开文档
├── src-tauri/              # Rust 后端
│   ├── Cargo.toml / tauri.conf.json / build.rs
│   ├── capabilities/ / icons/ / data/
│   └── src/
│       ├── main.rs / lib.rs / database.rs / store.rs / error.rs / types.rs
│       ├── provider/       # Stage 2: 5 adapter (openai/deepseek/anthropic/github/postgres)
│       ├── services/       # provider / category / quota / token_usage / pricing / auto_import
│       ├── commands/       # provider / quota / tray / token_usage / action
│       ├── actions/        # Action Registry (Stage 10)
│       └── tray.rs         # Stage 5
└── webui/                  # React 18 + TS + Vite 5
    ├── package.json / tsconfig.json / vite.config.ts / index.html
    ├── playwright.config.ts + tests/    # Stage f
    └── src/
        ├── main.tsx / App.tsx
        ├── components/ (UsageKpiCards / UsageTimeSeries / UsageStatsSidebar / ... 30+ files)
        ├── pages/ (UsagePage)
        ├── hooks/ (useUsage / useUsageTick / useProviders / useCategories / useTheme / useActions / useQuota)
        ├── lib/ (api / utils / action-registry / format)
        ├── types/ (api)   # 12 IPC + Token Usage + PeriodsSummary 契约
        └── styles/ (globals.css)  # Kaku design tokens
```

### 4.2 命名

- **Rust**: snake_case 文件名、snake_case 函数、PascalCase struct/enum、SCREAMING_SNAKE_CASE const
- **TypeScript/React**: PascalCase 组件、camelCase 函数/变量、`*.tsx` 用于 JSX
- **Tauri 命令**: `verb_noun` 形式,跨语言一致(`list_providers` / `add_provider` / `fetch_quota`)

### 4.3 错误处理

- Rust 用 `thiserror` 定义 `AppError` enum,所有 fallible 函数返回 `Result<T, AppError>`
- Tauri command 把 `AppError` 序列化成 `{ code, message }` JSON
- 前端用 TanStack Query 的 `error` 字段拿到,统一 toast 展示

### 4.4 bd Worktree 规则

每个 bd worktree **必须**开在新分支上,禁止在现有进行中分支上派生:

- **新分支命名**:`bd/<task-id>/<slug>`(例:`bd/proj-123/token-cost-aggregation`)
- **禁止起点**:`docs` / `main` / `ui-update` / 任何 stage 进行中的分支
- **禁止操作**:`git worktree add <path> <existing-branch>`(重复占用)
- **允许起点**:稳定 commit SHA / 已发布 tag / 主线干净 base
- **why**:
  - stage A/B 进行中成果必须先合到 base,worktree 才看得到,否则下游 worktree 在空 base 上做无用功
  - worktree 与主分支共用分支 = 互相污染,staging / 工作树冲突
  - 多 bd 并行时独立分支可独立合并、独立回滚

**新建命令模板**:

```bash
# 1. 先确保 base 已包含上游 stage 成果(cherry-pick 或 rebase)
git fetch origin
git rebase origin/main  # 或目标 base

# 2. 开新分支 + worktree
git worktree add -b bd/<task-id>/<slug> .claude/worktrees/<slug> <base-sha>
```

**验证**(每次开 bd worktree 前):

```bash
git worktree list --porcelain | grep "^branch"  # 应全是 refs/heads/bd/*
```

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
# 跑 dev(热重载)— 项目无根 package.json,必须用 cargo,不能用 pnpm tauri dev
cd keypilot-dev
cargo tauri dev

# 编译/类型/测试 — 完整矩阵走 ./init.sh
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
cd webui && pnpm tsc --noEmit
```

提交前必跑 `./init.sh` (硬约束 grep + JSON 校验),见 §10。

---

## 7. 提交规范

每个 Stage 完成的提交,信息格式:

```
<Stage> <一句话总结>

- 文件清单: <列出>
- 验证: <cargo check / cargo test / cargo build 结果>
- 硬约束: <grep 验证结果>
```

例:

```
Stage 1: Tauri 2 脚手架 + SQLite 数据层

- 文件清单: Cargo.toml / tauri.conf.json / build.rs / main.rs / lib.rs / capabilities/default.json / database.rs / store.rs / error.rs
- 验证: cargo check 通过
- 硬约束: api_key TEXT 明文确认 / 无加密 crate / 无 CLI 写路径
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

---

## 10. 验证 (Verification) — Sprint Contract

每个 Stage 完成前**必须**跑 `./init.sh` 全绿。机械 grep gates (8 项: visibility / value / preset / category_id / agent_file_cursor / 无加密 crate / notify-debouncer-full / fs::write 白名单) 全部内化在 init.sh §4,不再在此重复。**反自欺 3 条**: 没有 `.skip`/`.todo`/`unimplemented!()` 残留;没有"应该可以工作"但没跑过的代码路径;没有谎称 `cargo test` 实际只跑了 `cargo check`。

---

## 11. 求助顺序 (Escalation)

出问题时的顺序:

1. 查 `PLAN.md` 当前 Stage 的文件清单
2. 查 `session-handoff.md` / `progress.md` 看上次卡在哪
3. 派 `oracle` 评审架构 / 派 `librarian` 查库文档
4. 问用户(避免在 debug 黑洞里转太久)

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

*真理源: git log (变更历史) + feature_list.json (feature 状态) + progress.md (session 进度) + session-handoff.md (下次 session 起点)*