# V0.1 Spec Alignment — Proposal

> **Status**: ✅ Done
> **Date**: 2026-06-24
> **Session**: ses-2026-06-24-grilling(继 ses-2026-06-24-overlay 后)
> **Author**: MiniMax-M3 (orchestrator)
> **Spec Owner**: keypilot owner(user)

---

## Why

V0.1 spec 在 PM 工厂(`技术方案.md` / `MVP-范围.md` / `codemap.md`)和 dev(`PLAN.md` / `feature_list.json` / `AGENTS.md`)之间存在 **多处不明确 / 冲突**。本 change 通过:

1. **Grill 设计树** — 跟用户 1-by-1 走完关键 spec 决策
2. **抄参考项目** — 从 `references/` 下的已验证实现(cc-switch / openai-balance / api-key-checker / litellm)提取算法和模式
3. **YAGNI / 实用主义** — 砍掉过度设计,锁死最小可工作 spec

把 V0.1 核心 spec 决策从"模糊"变成"可实施",使 Stage 1-4 不再因 spec 不明而停摆。

## What changed(高层)

| 主题 | 决策 |
|---|---|
| Copy-out UX | 砍 B(智能 format),留 A(纯明文复制) |
| ProviderKind 范围 | 3 enum + 3 实现 + Custom,**无 stub**(YAGNI) |
| OpenAI quota 算法 | 抄 `openai-balance/cmd/root.go`(subscription + usage 间接计算) |
| DeepSeek quota 算法 | 抄 `cc-switch/services/balance.rs::query_deepseek` |
| Anthropic quota | 显式 `QuotaError::Unsupported`(OAuth 路径违反 §3.1) |
| Anthropic validate_key | `POST /v1/messages` + `max_tokens=1`(Anthropic 无 `/v1/models`) |
| OpenAI/DeepSeek validate_key | `GET /v1/models` / `GET /user/balance`,200/401 二态 |
| Provider 重复添加 | 允许(学习 cc-switch),uuid 区分,name 自由,不自动 #1 #2 |
| Preset 模式 | 3 个官方 seed + `is_preset` 标记 + `meta.preset_seeded` flag(不重建) |
| Schema 改动 | `providers.is_preset INTEGER NOT NULL DEFAULT 0`(schema v1 → v2) |

详细 delta 见 `spec.md`,实现细节见 `design.md`,任务清单见 `tasks.md`。

## Reference projects cited

| Project | Path | 用于 |
|---|---|---|
| openai-balance | `references/tier2-tech/openai-balance/` | OpenAI quota 算法(Go 163 行,canonical 实现) |
| cc-switch | `references/tier1-direct/cc-switch/` | DeepSeek quota / Provider 重复 / Preset seed 模式 / `init_default_official_providers` 模式 |
| api-key-checker | `references/tier1-direct/api-key-checker/` | validate_key 模式(间接,Anthropic 部分无参考) |
| codemap.md | `PM思考工厂/keypilot/codemap.md` | 跨项目对照表 + 各参考项目结构说明 |

**关键发现**:
- cc-switch **没有** OpenAI direct API key 余额查询(走 Claude Code OAuth 路径,KeyPilot 不能用)
- cc-switch **没有** validate_key 命令(依赖"切换 provider 时自然发现 401"——KeyPilot 显式做,体验更好)
- api-key-checker **没有** Anthropic checker(只有 Gemini / OpenRouter / SiliconFlow / DeepSeek)

## Deferred(本次不锁)

以下 spec 留待后续 session,本次未达成共识或非 V0.1 阻塞:

- 托盘右键菜单结构(Stage 5 详细 spec)
- i18n / 字符串归属(V0.1 inline 中文 vs 抽 constants)
- 手动输入额度数据存哪(`quota_cache` + `QuotaSource::Manual` 还是新表)
- Key 显示遮罩规则(前 3 后 4 / 前 4 后 6 / 其他)
- 删除 Provider 确认弹窗
- 添加 Provider 时 `base_url` 缺省是否可编辑
- 托盘关主窗口 vs 退出行为
- Frontend 栈清理(MVP-范围.md "Svelte 或 React" 矛盾)
- Stage 1 config 一致性(tauri.conf.json `icon: []` / `frontendDist` 指向空 / 窗口无 label)
- `DESIGN.md` 污染(kaku.fun 文件,删?)
- GitHub 仓库名(`keypilot/keypilot` 拍板?)
- Azure Trusted Signing 申请责任(谁负责?)

## Status & 后续

✅ **本次 spec alignment 完成,Stage 1-4 实施可直接引用本 change 的 `spec.md` / `design.md` / `tasks.md`**。

Stage 1 实施本身仍在 in-progress(等待 5 个 Rust 源文件 + tauri.conf.json 不一致修复),不阻塞 spec 锁定。

下一次 session 优先级:
1. **修 Stage 1 阻塞项** — tauri.conf.json 窗口 label / frontendDist / icon,补 5 个 Rust 源文件
2. **Stage 2 实施** — 按 `tasks.md T2.1-T2.7` 推进
3. **上面"Deferred"列表** — 任意挑一个继续
