<h1 align="center">KeyPilot</h1>

<p align="center">
  <strong>Local credential vault &amp; quota tracker for the AI era.</strong><br/>
  Windows-first · Rust + Tauri 2 · MIT
</p>

<p align="center">
  <a href="#quick-start"><img alt="Quick Start" src="https://img.shields.io/badge/quick-start-000?style=flat-square&logo=readme&logoColor=white"/></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/YHlorra/keypilot?style=flat-square"/></a>
  <img alt="Version" src="https://img.shields.io/badge/version-0.2.1-000?style=flat-square"/>
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6?style=flat-square&logo=windows&logoColor=white"/>
  <img alt="Stack" src="https://img.shields.io/badge/stack-Tauri%202%20%C2%B7%20Rust%20%C2%B7%20React%20%E2%9A%99%EF%B8%8F-000?style=flat-square"/>
</p>

<p align="center">
  <a href="README.zh-CN.md">简体中文</a> · <a href="#license">License</a>
</p>

---

> [!NOTE]
> This repository is a **public showcase snapshot** of KeyPilot V0.2.1. Issues and pull requests are welcome.

## Contents

- [Quick Start](#quick-start)
- [About](#about)
- [Features](#features)
- [Connecting to Providers](#connecting-to-providers)
- [Tech Stack](#tech-stack)
- [Scope (V0.2.1)](#scope-v021)
- [Project Structure](#project-structure)
- [Development](#development)
- [When (Not) to Use](#when-not-to-use)
- [Roadmap](#roadmap)
- [License](#license)

## Quick Start

### Prerequisites

| Tool | Version |
|---|---|
| Rust (nightly) | 1.92 |
| Node.js | 22.22 |
| pnpm | 11.0.9 |
| Tauri CLI | 2.11.0 |
| WebView2 | bundled with Windows 11 |
| MSVC Build Tools | required for `cargo` linker |

### Run from source

```bash
git clone https://github.com/YHlorra/keypilot.git
cd keypilot

# 1. install frontend deps
cd webui && pnpm install && cd ..

# 2. start dev (Tauri hot-reload, opens native window)
cargo tauri dev
```

The first launch seeds an empty SQLite DB at `%APPDATA%\com.keypilot.app\`. Add a provider, paste an API key, hit **Fetch Quota** — done.

## About

KeyPilot is a **local credential manager for AI power users**. It centralises API keys, AK/SK pairs and connection strings for every LLM provider you touch, and refreshes quota / balance in the background so you know what's left before you hit send.

V0.1 deliberately stops at three things:

1. **Provider presets** — every LLM / Coding-Plan provider you actually use has a built-in field schema.
2. **Live quota** — the things you pay for (API wallet, Coding Plan 5-hour / weekly windows) update in the background and surface in a single tray-side panel.
3. **Token usage history** — auto-parses sessions from OpenCode / Claude Code / Codex, so you can see which agent burned what, when.

Credential encryption, cross-platform support and cloud sync are **explicitly out of scope** — see [Roadmap](#roadmap) and [When (Not) to Use](#when-not-to-use).

## Features

- **21 built-in provider presets** (V0.2.1) — Anthropic, OpenAI, DeepSeek, GitHub Models, Volcengine, Kimi, GLM, MiniMax (CN/EN), Mimo, OpenRouter, Groq, Mistral, SiliconFlow, Together, StepFun, Cohere, Perplexity, Fireworks AI, AI21 Labs. Each preset ships its own field schema, docs_url and quota endpoint.
- **Multi-endpoint catalog (V0.2.1)** — 6 dual-protocol presets (Kimi / GLM / Volcengine / DeepSeek / MiniMax-CN / MiniMax-EN) expose both OpenAI-compat and Anthropic-compat endpoints under a single API key. Click "另有 N 个协议端点" on the card to inspect.
- **Add Custom Provider (V0.2)** — `+` → "+ 自定义" dialog (shadcn/Radix/Tailwind). Pick protocol, fill URL + Key, "测试连接" runs `provider.preflight` against the live endpoint before save.
- **7 Coding Plan trackers** — Kimi For Coding, GLM Coding, MiniMax Token Plan (CN+EN), Volcengine Ark Coding Plan, ZenMux 5h + weekly windows. Both 5-hour and weekly windows surfaced side-by-side.
- **5 protocol registry (V0.2)** — typed match on `ProtocolId` (OpenAI / Anthropic / GitHub / Balance / DeepSeek). Adding a new protocol = 1 file under `src-tauri/src/provider/protocols/` + 1 line in `registry.rs`. Adding a new preset = 1 toml file. No central switch.
- **3 themes** — Dark / Light / Follow System. Radix UI Colors palette, brutalist typography, no Tailwind defaults.
- **Visibility tri-state** — `visible → masked → revealed`. The disk never cares; the UI decides.
- **Token usage history** — auto-parses OpenCode / Claude Code / Codex session logs. Heatmap + trend line + per-agent leaderboard.
- **System tray resident** — single source of truth, 5-minute `staleTime`, real-time event push from Rust backend.
- **Action registry** — extension hooks for power users (custom fetchers, auto-refresh, low-quota alerts).

## Connecting to Providers

Every provider follows the same flow: pick a preset → fill in `api_key` (and any preset-specific fields) → **Fetch Quota**.

| Category | Examples | What you need |
|---|---|---|
| LLM API | Anthropic, OpenAI, DeepSeek, Kimi, GLM, MiniMax | `api_key` |
| LLM Aggregator | OpenRouter, ZenMux, Volcengine Ark | `api_key` (+ optional `endpoint`) |
| Coding Plan | Kimi For Coding, GLM Coding, MiniMax Token Plan, Volcengine Coding Plan, ZenMux | `api_key` (auto-discovers 5h / weekly windows) |
| Dev / Data | GitHub Models, Postgres | `api_key` / connection string |

> [!TIP]
> Provider presets live in `src-tauri/src/provider/`. Adding a new provider = one file (adapter) + one row in the preset registry. No central switch statement to touch.

For Coding Plans, KeyPilot auto-fetches both windows and shows the tighter one in red when it drops below 20%.

## Tech Stack

| Layer | Tech |
|---|---|
| Desktop shell | Tauri 2 |
| Backend | Rust (edition 2021, nightly 1.92) |
| Frontend | React 18 + TypeScript |
| Build | Vite 6 |
| State | TanStack Query v5 |
| UI | shadcn/ui (Radix Primitives) + Tailwind + `@radix-ui/colors` |
| DB | SQLite (`rusqlite` bundled) |
| Tray / FS watch | `tauri::tray`, `notify-debouncer-full` |

## Scope (V0.2.1)

| Capability | Status |
|---|---|
| Windows 10 / 11 | Supported |
| Credential storage | Plaintext + Windows ACL (no encryption) |
| Live quota | 21 LLM presets + 7 Coding Plans (V0.2.1) |
| Multi-endpoint per preset | V0.2.1 — 6 dual-protocol presets share 1 API key |
| Token usage history | OpenCode / Claude Code / Codex auto-parse |
| Cross-platform | Not in V0.3+ |
| Master password / Argon2 | Deferred to V0.2.2+ RFC |
| Auto-refresh / low-quota alerts | V0.2 |
| Real LLM call routing | V0.3+ (catalog already has `extras` plumbing) |
| Import / Export | V0.2 |

## Project Structure

```
keypilot/
├── README.md / README.zh-CN.md   # this file
├── LICENSE                        # MIT
├── src-tauri/                     # Rust backend (Tauri 2)
│   ├── Cargo.toml / tauri.conf.json / build.rs
│   └── src/
│       ├── main.rs / lib.rs / database.rs / store.rs / error.rs / types.rs
│       ├── provider/              # 24 provider adapters
│       ├── services/              # provider / category / quota / token_usage / pricing / auto_import
│       ├── commands/              # tauri IPC commands
│       ├── actions/               # Action Registry (extension hooks)
│       └── tray.rs                # system tray
└── webui/                         # React 18 + TS + Vite 6
    ├── package.json / vite.config.ts
    └── src/
        ├── components/ (30+ files)
        ├── pages/ (UsagePage)
        ├── hooks/ (useUsage / useProviders / useQuota / ...)
        ├── lib/ (api / utils / action-registry / format)
        └── styles/ (globals.css)  # design tokens
```

## Development

```bash
# dev (hot reload)
cargo tauri dev                  # run from project root

# frontend only
cd webui && pnpm dev

# typecheck
cd webui && pnpm tsc --noEmit

# unit tests (Rust)
cargo test --manifest-path src-tauri/Cargo.toml

# unit tests (WebUI)
cd webui && pnpm test

# e2e (Playwright, when configured)
cd webui && pnpm exec playwright test

# build release
cargo tauri build
```

## When (Not) to Use

Use KeyPilot when:

- You juggle 3+ AI providers / Coding Plans and want one place to see quota left.
- You want to track how much OpenCode / Claude Code / Codex sessions cost you per agent per day.
- You run Windows 10/11 and trust the local-user ACL boundary.

Do **not** use KeyPilot when:

- You need cross-platform (macOS / Linux) — V0.3+ at the earliest.
- You need at-rest encryption (master password, Argon2id, DPAPI, SQLCipher) — V0.1 explicitly skips this.
- You need a general-purpose password manager (1Password / Bitwarden / KeePass are the right tools).
- You need cloud sync, multi-device, or team sharing — out of scope.
- You run on a shared / low-trust Windows host where anyone can read `%APPDATA%` of your user.

## Roadmap

| Version | Status | Focus |
|---|---|---|
| V0.1 | ✅ Shipped | 5 presets, manual quota, Windows-only, plaintext credentials |
| V0.2 | ✅ Shipped | 18 presets via catalog; 5 protocol registry (typed match); AddCustomProviderDialog + provider.preflight |
| **V0.2.1** | ✅ Shipped | Multi-endpoint catalog — 6 dual-protocol presets (Kimi / GLM / Volcengine / DeepSeek / MiniMax-CN / MiniMax-EN) merge 12 toml → 6 with `[[extras]]`; +3 single-protocol presets (Mimo / Fireworks / AI21) → 21 total; ProviderCard extras foldout; preset icons auto-resolved from catalog |
| V0.2.2 | Planned | Encryption RFC (SQLCipher / master-password Argon2id / DPAPI); T3.9 advanced JSON mode |
| V0.3 | Planned | macOS / Linux support; real LLM call routing using catalog `extras` |
| V1.0 | Planned | Stable API, signed releases, auto-update channel |

## License

[MIT](LICENSE) &copy; 2026 KeyPilot Authors.
