# KeyPilot

> AI 时代的本地凭证管理 + 额度查询
> Windows 优先 · Rust + Tauri 2 · MIT

## 简介

KeyPilot 是一个本地凭证管理员,集中存储 AI 服务凭证(API Key、AK/SK、连接串)并实时查询额度。
凭证加密不是 V0.1 的目标(依赖 Windows ACL 限制访问),跨平台也不是 V0.1 的目标(Win 优先)。

## 主要功能

- **24 个预设 Provider**:Anthropic / OpenAI / DeepSeek / GitHub / Volcengine / Kimi / GLM / MiniMax / ZenMux 等 LLM 与开发工具凭证模板
- **11 个 Coding Plan 额度查询**:Kimi For Coding / GLM Coding / MiniMax Token Plan / 火山方舟 Coding Plan / ZenMux 的 5 小时 + 周窗口
- **三主题**:Dark / Light / Follow System,Radix UI Colors 调色板
- **可见性三态复制**:visible → masked → revealed
- **Token 用量历史**:OpenCode / Claude Code / Codex session 自动解析,热力图 + 趋势折线 + Agent 排行
- **系统托盘常驻**:单数据源,5 分钟 staleTime,实时事件推送

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面壳 | Tauri 2 |
| 后端 | Rust |
| 前端 | React 18 + TypeScript |
| 构建 | Vite 5 |
| 状态 | TanStack Query v5 |
| UI | shadcn/ui(Radix Primitives)+ Tailwind |
| 数据 | SQLite(rusqlite bundled) |

## V0.1 范围

| 能力 | 状态 |
|---|---|
| Windows 10/11 | ✅ 支持 |
| 凭证存储 | 明文 + Windows ACL(无加密) |
| 实时额度查询 | ✅ 5 个 LLM + 6 个 Coding Plan |
| 跨平台 | ❌ V0.3+ |
| 主密码 / Argon2 | ❌ V0.2 评估 |
| 自动刷新 / 低额度告警 | ❌ V0.2 |
| 导入 / 导出 | ❌ V0.2 |

## 仓库说明

本仓库为 KeyPilot V0.1 公开展示版本,展示代码结构与实现。不接受外部协作与 PR。
内部开发文档(规划、变更提案、验证脚本)不在本仓库内。

## 许可

MIT — 详见 [LICENSE](LICENSE)
