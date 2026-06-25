# V0.1 General Credentials — Proposal

> **Status**: ✅ Done (Approved 2026-06-24 via /think grill)
> **Date**: 2026-06-24
> **Session**: ses-2026-06-24-grilling (grill-mode walkthrough of `docs/index.html`)
> **Author**: MiniMax-M3 (orchestrator)
> **Spec Owner**: keypilot owner (user)
> **Supersedes**: `openspec/changes/v0.1-spec-alignment/` for V0.1 product scope (schema migration framework / REQ-PROV-002 seed mechanism / REQ-SCHEMA-001 v1→v2 still apply; preset count + visibility + Category + theme are amended)

---

## Why

`docs/index.html` 设计文档(c.1400 行,静态原型)是 V0.1 UI 的参考真源,但与 `openspec/changes/v0.1-spec-alignment/`(V0.1 第一轮 spec 锁定, AI-Provider-only)存在 **8 处冲突**。本 change 通过:

1. **1-by-1 grill 决策树** — 跟用户逐一走完冲突点(9 轮 question 工具)
2. **设计 → spec 回写** — 通用凭证库取代 AI-only 范围,扩 Category / visibility / 多 theme
3. **保守派原则** — 多数冲突选择"沿用 spec 限制,UI 留 V0.2 标注",避免 V0.1 范围爆炸
4. **schema 重写** — fields 从 column 变 row(任意 KV),加 categories 表 + visibility 枚举

把 V0.1 spec 从"AI Provider 凭证库"重新锁定为"通用凭证库 + 字段级 opt-in 加密",使 Stage 1 重写有清晰依据。

## What changed(高层)

| 主题 | 旧(v0.1-spec-alignment) | 新(v0.1-general-credentials) |
|---|---|---|
| 产品范围 | AI Provider only(OpenAI / DeepSeek / Anthropic) | **通用凭证库**(AI + DB + Dev + 任意 KV) |
| Preset 数量 | 3 个 LLM preset | **5 个 preset**(OpenAI / DeepSeek / Anthropic / GitHub / PostgreSQL) |
| 字段存储 | `providers.base_url` / `providers.api_key` 硬列 | **`provider_fields` 行存任意 KV** |
| 加密 | 全部明文(§3.2 锁死) | **V0.1 不加密**:`visibility: visible \| masked`(二态,UI 掩码不落盘加密);V0.2 评估加密 RFC |
| Category 分组 | 无 | **`categories` 表 + sidebar 可折叠分组**,default `凭证` 不可删 |
| 主题 | 单主题(纯黑) | **Dark / Light / Follow System 三套** |
| 模板 | blank / llm / database / oauth(4) | **blank / llm / database(3)**,oauth 砍,preset + template 都降级为可选助手 |
| 导入/导出/同步 | 推到 V0.2(沿用 §3.3) | **沿用 V0.2**,UI 显示 disabled + tooltip |
| test_connection | 3 LLM | **3 LLM 启用,GitHub/Postgres disabled**(沿用设计 canTest 检测) |
| fetch_quota | OpenAI / DeepSeek / Anthropic=Unsupported | **3 LLM 启用,GitHub/Postgres quota 列隐藏** |
| 状态栏文案 | "本地 Keychain 加密存储" | **"本地 SQLite · 字段级加密 opt-in"** |
| Footer 版本 | `v 0.2.0` | **`v 0.1.0`** |

## Grill log(9 轮决策树)

1. **产品范围**:通用凭证库 ✅(用户决策,扩 spec)
2. **加密策略**:字段级 opt-in,默认明文 ✅(用户决策)
3. **Category 模型**:Flat 1:1 ✅
4. **Preset 列表**:5 个(OpenAI/DeepSeek/Anthropic/GitHub/PostgreSQL) ✅
5. **test_connection**:只 3 LLM 启用 ✅
6. **fetch_quota**:只 3 LLM 启用 ✅
7. **导入/导出/同步**:V0.2 推迟,UI disabled ✅(用户先问"什么叫导入导出",后决策 §3.3 不变)
8. **主题切换**:Dark / Light / Follow System 三套 ✅(用户决策,"深色，浅色，跟随系统")
9. **OAuth template**:砍,preset + template 降级为可选助手,默认 blank ✅(用户决策,"给一个默认即可")

详细 delta 见 `spec.md`(14 REQs,ADDED/MODIFIED/REMOVED),实现细节见 `design.md`,任务清单见 `tasks.md`。

## Reference projects cited(沿用 v0.1-spec-alignment)

| Project | Path | 用于 |
|---|---|---|
| openai-balance | `references/tier2-tech/openai-balance/` | OpenAI quota 算法(Go 163 行,canonical 实现) |
| cc-switch | `references/tier1-direct/cc-switch/` | DeepSeek quota / Preset seed 模式 / `init_default_official_providers` |
| api-key-checker | `references/tier1-direct/api-key-checker/` | validate_key 模式 |
| Bitwarden / 1Password | 设计文档 `docs/index.html` 静态原型 | Category 分组 / visibility 模式 / tray preview 卡 |

## Deferred(本次不锁,沿用 v0.1-spec-alignment 列表)

- 字段加密具体算法(主密码派生 / OS DPAPI / libsodium,V0.1 不实现,V0.2 RFC)
- Light theme CSS 调色板(Stage 3 实施时拍板,本 change 不锁具体颜色)
- Tray 右键菜单结构(Stage 5 详细 spec)
- i18n / 字符串归属
- 删除 Provider 确认弹窗细节
- Frontend 栈清理(MVP-范围.md "Svelte 或 React" 矛盾)
- GitHub 仓库名(`keypilot/keypilot` 拍板?)
- Azure Trusted Signing 申请责任

## Status & 后续

✅ **本 change 完成,Stage 1-4 实施可直接引用本 change 的 `spec.md` / `design.md` / `tasks.md`**。

Stage 1 实施 in-progress 状态:5 个 Rust 源文件缺失 / tauri.conf.json 3 处配置不一致 — 本 change 锁定 schema 后由 `@fixer` 后台重写。

下次 session 优先级:
1. **Stage 1 重写** — 按 `tasks.md T1.1-T1.10` 推进,5 个 Rust 源文件按新 schema 落地
2. **tauri.conf.json 修复** — 窗口 label / frontendDist / icon 占位
3. **Stage 2-9 范围重审** — `feature_list.json` 与本 change 对齐(preset 5 / visibility / 3 themes / oauth 砍)
4. **docs/index.html 同步** — 静态原型按本决策更新(preset 列表 / statusbar 文案 / 砍 oauth / footer 版本)
5. **AGENTS.md §3 硬约束更新** — §3.2 从"明文 api_key" 改为"明文默认 + 字段级 opt-in 加密"
