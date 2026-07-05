<h1 align="center">KeyPilot</h1>

<p align="center">
  <strong>AI 时代的本地凭证管理 + 额度查询</strong><br/>
  Windows 优先 · Rust + Tauri 2 · MIT
</p>

<p align="center">
  <a href="#快速开始"><img alt="快速开始" src="https://img.shields.io/badge/快速开始-000?style=flat-square&logo=readme&logoColor=white"/></a>
  <a href="LICENSE"><img alt="许可证" src="https://img.shields.io/github/license/YHlorra/keypilot?style=flat-square"/></a>
  <img alt="版本" src="https://img.shields.io/badge/version-0.2.1-000?style=flat-square"/>
  <img alt="平台" src="https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6?style=flat-square&logo=windows&logoColor=white"/>
  <img alt="技术栈" src="https://img.shields.io/badge/stack-Tauri%202%20%C2%B7%20Rust%20%C2%B7%20React%20%E2%9A%99%EF%B8%8F-000?style=flat-square"/>
</p>

<p align="center">
  <a href="README.md">English</a> · <a href="#许可">许可</a>
</p>

---

> [!NOTE]
> 本仓库是 KeyPilot V0.1 的**公开展示快照**,欢迎提交 Issue 与 Pull Request。

## 目录

- [快速开始](#快速开始)
- [简介](#简介)
- [主要功能](#主要功能)
- [接入 Provider](#接入-provider)
- [技术栈](#技术栈)
- [V0.1 范围](#v01-范围)
- [目录结构](#目录结构)
- [开发](#开发)
- [何时不该用](#何时不该用)
- [路线图](#路线图)
- [许可](#许可)

## 快速开始

### 环境依赖

| 工具 | 版本 |
|---|---|
| Rust (nightly) | 1.92 |
| Node.js | 22.22 |
| pnpm | 11.0.9 |
| Tauri CLI | 2.11.0 |
| WebView2 | Windows 11 自带 |
| MSVC Build Tools | `cargo` 链接器所需 |

### 从源码运行

```bash
git clone https://github.com/YHlorra/keypilot.git
cd keypilot

# 1. 安装前端依赖
cd webui && pnpm install && cd ..

# 2. 启动 dev (Tauri 热重载,弹出原生窗口)
cargo tauri dev
```

首次启动会在 `%APPDATA%\com.keypilot.app\` 下创建一个空的 SQLite DB。添加 Provider、粘贴 API Key、点 **Fetch Quota**,完事。

## 简介

KeyPilot 是面向 **AI 重度用户** 的本地凭证管理工具。它把所有 LLM Provider 的 API Key、AK/SK、连接串集中到一处,后台持续刷新额度,让你在按下回车之前就知道还剩多少。

V0.1 只做三件事:

1. **Provider 预设** —— 你实际用到的每个 LLM / Coding Plan Provider 都自带字段模板。
2. **实时额度** —— 你付费的东西(API 钱包、Coding Plan 5 小时 / 周窗口)后台自动更新,统一在托盘侧栏展示。
3. **Token 用量历史** —— 自动解析 OpenCode / Claude Code / Codex 的 session 日志,看清哪个 Agent 在什么时候烧了多少。

凭证加密、跨平台、云同步 **明确不在范围内** —— 见 [路线图](#路线图) 与 [何时不该用](#何时不该用)。

## 主要功能

- **21 个内置 Provider 预设**(V0.2.1)—— Anthropic、OpenAI、DeepSeek、GitHub Models、Volcengine、Kimi、GLM、MiniMax(CN/EN)、Mimo、OpenRouter、Groq、Mistral、SiliconFlow、Together、StepFun、Cohere、Perplexity、Fireworks AI、AI21 Labs。每个预设自带字段模板、docs_url 与额度接口。
- **多端点 catalog(V0.2.1)**—— 6 个双协议预设(Kimi / GLM / Volcengine / DeepSeek / MiniMax-CN / MiniMax-EN)共用同一 API key,暴露 OpenAI 兼容 + Anthropic 兼容双端点。点击卡片下方 "另有 N 个协议端点" 折叠区查看。
- **添加自定义 Provider(V0.2)**—— `+` → "+ 自定义" 对话框(shadcn/Radix/Tailwind)。选协议,填 URL + Key,「测试连接」通过 `provider.preflight` 直接打活端点验证后才落库。
- **7 个 Coding Plan 追踪**—— Kimi For Coding、GLM Coding、MiniMax Token Plan(CN+EN)、Volcengine Ark Coding Plan、ZenMux 5h + 周窗口。
- **5 协议 registry(V0.2)**—— typed match on `ProtocolId`(OpenAI / Anthropic / GitHub / Balance / DeepSeek)。加新协议 = `src-tauri/src/provider/protocols/` 下 1 文件 + `registry.rs` 加 1 行。加新预设 = 1 个 toml。零中心 switch。
- **三主题** —— Dark / Light / Follow System。Radix UI Colors 调色板,brutalist 排版,不用 Tailwind 默认色。
- **可见性三态** —— `visible → masked → revealed`。落盘始终是明文,UI 决定怎么显示。
- **Token 用量历史** —— 自动解析 OpenCode / Claude Code / Codex session 日志。热力图 + 趋势折线 + Agent 排行。
- **系统托盘常驻** —— 单一数据源,5 分钟 `staleTime`,Rust 后端事件实时推送。
- **Action Registry** —— 给重度用户的扩展钩子(自定义 fetcher、自动刷新、低额度告警)。

## 接入 Provider

所有 Provider 走同一套流程:选预设 → 填 `api_key`(以及预设要求的其它字段) → 点 **Fetch Quota**。

| 类别 | 例子 | 所需字段 |
|---|---|---|
| LLM API | Anthropic、OpenAI、DeepSeek、Kimi、GLM、MiniMax | `api_key` |
| LLM 聚合 | OpenRouter、ZenMux、Volcengine Ark | `api_key`(可选 `endpoint`) |
| Coding Plan | Kimi For Coding、GLM Coding、MiniMax Token Plan、Volcengine Coding Plan、ZenMux | `api_key`(自动识别 5h / 周窗口) |
| 工具 / 数据 | GitHub Models、Postgres | `api_key` / 连接串 |

> [!TIP]
> Provider 预设放在 `src-tauri/src/provider/`。新增 Provider = 一个文件(adapter)+ 预设注册表一行。无需改任何中央 switch。

Coding Plan 会自动拉取两个窗口,当较紧的窗口低于 20% 时高亮红色。

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面壳 | Tauri 2 |
| 后端 | Rust(edition 2021,nightly 1.92) |
| 前端 | React 18 + TypeScript |
| 构建 | Vite 6 |
| 状态 | TanStack Query v5 |
| UI | shadcn/ui(Radix Primitives)+ Tailwind + `@radix-ui/colors` |
| 数据 | SQLite(`rusqlite` bundled) |
| 托盘 / 文件监听 | `tauri::tray`、`notify-debouncer-full` |

## V0.2.1 范围

| 能力 | 状态 |
|---|---|
| Windows 10 / 11 | 支持 |
| 凭证存储 | 明文 + Windows ACL(无加密) |
| 实时额度 | 21 个 LLM 预设 + 7 个 Coding Plan(V0.2.1) |
| 多端点预设 | V0.2.1 — 6 个双协议预设共用 1 个 API key |
| Token 用量历史 | OpenCode / Claude Code / Codex 自动解析 |
| 跨平台 | V0.3+ |
| 主密码 / Argon2 | 推迟到 V0.2.2+ RFC |
| 自动刷新 / 低额度告警 | V0.2 |
| 真实 LLM 调用路由 | V0.3+(catalog 已铺好 `extras` 管道) |

## 目录结构

```
keypilot/
├── README.md / README.zh-CN.md   # 本文件
├── LICENSE                        # MIT
├── src-tauri/                     # Rust 后端(Tauri 2)
│   ├── Cargo.toml / tauri.conf.json / build.rs
│   └── src/
│       ├── main.rs / lib.rs / database.rs / store.rs / error.rs / types.rs
│       ├── provider/              # 24 个 Provider 适配器
│       ├── services/              # provider / category / quota / token_usage / pricing / auto_import
│       ├── commands/              # tauri IPC 命令
│       ├── actions/               # Action Registry(扩展钩子)
│       └── tray.rs                # 系统托盘
└── webui/                         # React 18 + TS + Vite 6
    ├── package.json / vite.config.ts
    └── src/
        ├── components/ (30+ 文件)
        ├── pages/ (UsagePage)
        ├── hooks/ (useUsage / useProviders / useQuota / ...)
        ├── lib/ (api / utils / action-registry / format)
        └── styles/ (globals.css)  # 设计令牌
```

## 开发

```bash
# 开发(热重载)
cargo tauri dev                  # 在仓库根目录执行

# 仅前端
cd webui && pnpm dev

# 类型检查
cd webui && pnpm tsc --noEmit

# 单元测试(Rust)
cargo test --manifest-path src-tauri/Cargo.toml

# 单元测试(WebUI)
cd webui && pnpm test

# E2E(Playwright,配置完成后)
cd webui && pnpm exec playwright test

# 打包发布
cargo tauri build
```

## 何时不该用

适合用 KeyPilot 的场景:

- 你同时用 3+ 个 AI Provider / Coding Plan,想一眼看到额度。
- 想看清 OpenCode / Claude Code / Codex 每天每个 Agent 烧了多少 token。
- 你跑 Windows 10/11,信任本地用户 ACL 边界。

**不要**用 KeyPilot 的场景:

- 需要跨平台(macOS / Linux)—— 最早 V0.3+。
- 需要静态加密(主密码、Argon2id、DPAPI、SQLCipher)—— V0.1 明确不做。
- 需要通用密码管理器(请用 1Password / Bitwarden / KeePass)。
- 需要云同步、多设备、团队共享 —— 不在范围内。
- 跑在共享 / 低信任 Windows 主机上,任何人都能读 `%APPDATA%`。

## 路线图

| 版本 | 状态 | 重点 |
|---|---|---|
| V0.1 | ✅ 已发布 | 5 预设, 手动额度, 仅 Windows, 明文凭证 |
| V0.2 | ✅ 已发布 | 18 预设经由 catalog;5 协议 registry (typed match);AddCustomProviderDialog + provider.preflight |
| **V0.2.1** | ✅ 已发布 | 多端点 catalog — 6 个双协议预设 (Kimi / GLM / Volcengine / DeepSeek / MiniMax-CN / MiniMax-EN) 合并 12 toml → 6 含 `[[extras]]`;+3 单协议预设 (Mimo / Fireworks / AI21) → 共 21;ProviderCard extras 折叠区;预设图标由 catalog 自动反查 |
| V0.2.2 | 计划中 | Encryption RFC (SQLCipher / 主密码 Argon2id / DPAPI);T3.9 高级 JSON 模式 |
| V0.3 | 计划中 | macOS / Linux 支持;真实 LLM 调用路由(catalog `extras`) |
| V1.0 | 计划中 | 稳定 API、签名发布、自动更新通道 |

## 许可

[MIT](LICENSE) &copy; 2026 KeyPilot Authors.
