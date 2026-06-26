# Token Usage History — Proposal

## Problem

KeyPilot 用户管理多个 AI 提供商的凭证 (OpenAI / Anthropic / DeepSeek / MiniMax / 等)。日常使用中，用户通过不同的 AI coding agent (Claude Code / Codex CLI / OpenCode / Cursor / 等) 调用这些凭证，产生 token 消耗。

当前痛点：
1. **无归因**：不知道哪个 agent + 哪个模型组合烧掉了最多的 token
2. **无历史**：quota 只显示当前快照，看不到趋势
3. **无细分**：看不到 input / output / cache_read / reasoning 的构成

用户想要的可视化效果：

```
【claudecode-claude oups 4.8】   ← agent pair, 4.8M tokens
【codex-chatgpt   5.6】          ← agent pair, 5.6M tokens  
【opencode-minimax M3】           ← agent pair, X tokens
```

## Proposed Solution

在 KeyPilot 内建一层 token usage history，核心能力：

| 能力 | 说明 |
|---|---|
| **Agent Pair 归因** | 按 `(agent_type, model)` 组合聚合，一眼看哪个组合最费 |
| **Token 类型细分** | input / output / cache_read / cache_write / reasoning 五类 |
| **时间序列** | 日 / 周 / 月粒度，看趋势 |
| **成本换算** | 基于 LiteLLM pricing DB，自动算 USD 成本 |
| **多数据源接入** | 手动导入 (JSONL/CSV) + 可选轻量 proxy 模式 |

## Scope

**V0.2 包含**：
- SQLite `token_usage_records` 表 + 归约表
- 5 个 IPC: `record_usage` / `list_usage_records` / `get_usage_summary` / `import_usage` / `get_pricing`
- 前端: Usage 页面 (热力图 + 折线图 + agent pair 条形图)
- 定价表: 内置精简版 LiteLLM pricing (Top 50 models)
- 导入: JSONL / CSV 双格式，支持 Claude Code / Codex / OpenCode 日志

**V0.2 不包含**：
- 实时 proxy 拦截 (V0.3)
- 告警 / 预算上限 (V0.3)
- 多用户/团队归因 (V0.3)
- 自动 re-pricing 历史 (V0.3)

## Why Now

- Stage 1-11 完成，核心凭证管理功能稳定
- 用户反馈 "不知道钱花在哪" 是下一个高频需求
- 技术栈成熟 (SQLite + TanStack Query + shadcn/ui charts)
- 设计参考成熟 (Langfuse / LiteLLM / Helicone  converged pattern)
