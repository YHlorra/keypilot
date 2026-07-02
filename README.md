# KeyPilot V0.1

> AI 时代的钥匙管理员 — 本地凭证库 + 额度查询
> Windows 优先 · Rust + Tauri 2 · MIT 协议

## Features

- **20+ Preset Providers** across LLM (Anthropic-compat + OpenAI-compat) and Dev Tools categories
- **7 Coding Plan Quota Support**: Kimi For Coding, GLM CN / EN, MiniMax CN / EN, 火山方舟 Coding/Agent Plan, ZenMux (5-hour + weekly tier windows)
- **3 Themes**: Dark / Light / Follow System (Radix UI Colors)
- **CopyButton 3-state**: visible → masked → revealed
- **Detail + Tray Quota**: Single quota_cache data source, 5-min staleTime
- **Plaintext Storage**: V0.1 不加密 (明文 SQLite + Windows ACL)
- **Token Usage History**: OpenCode / Claude Code / Codex sessions 自动解析,`agent_file_cursor` 增量追踪 + notify-debouncer-full 实时推送 `token_usage_tick` 事件
- **Usage Page**: 三周期 PeriodsSummary(today/month/all_time)+ 26 周热力图 + 趋势折线 + agent 配对排行

### Coding Plan Quota

For users on a **coding plan subscription** (Kimi For Coding / GLM Coding / MiniMax Token Plan / 火山方舟 Coding Plan / ZenMux), keypilot queries each provider's quota endpoint and shows tier windows (5-hour rolling + weekly) directly in the usage page.

Note for **火山方舟 (Volcengine)**: `api_key` field is single-string but HMAC-SHA256 requires AK + SK. Paste as two lines separated by a newline:
```
AKLTxxxxxxxxxxxxxxxx
SKltxxxxxxxxxxxxxxxx
```
The split happens automatically at fetch time.

Supported providers (5-hour + weekly tier windows):

- Kimi For Coding — `https://api.kimi.com/coding/v1/usages`
- GLM CN — `https://open.bigmodel.cn/api/monitor/usage/quota/limit`
- GLM EN — `https://api.z.ai/api/monitor/usage/quota/limit`
- MiniMax CN/EN — `https://www.minimaxi.com/v1/token_plan/remains`
- 火山方舟 — `https://open.volcengineapi.com` (HMAC-SHA256 signed)
- ZenMux — `base_url` (uses provider-configured host)

## Tech Stack

- **Backend**: Tauri 2 + Rust
- **Frontend**: React 18 + TypeScript + Vite 5
- **State**: TanStack Query v5
- **UI**: shadcn/ui (Radix Primitives) + Tailwind + Radix UI Colors
- **Database**: SQLite (rusqlite with bundled SQLite)

## Screenshots

| View | Description |
|------|-------------|
| ![Main Window - Dark Theme](docs/screenshots/main-dark.png) | Main window in Dark theme |
| ![Quota Display](docs/screenshots/quota.png) | Quota badge and display |
| ![Tray Hover Card](docs/screenshots/tray.png) | System tray hover card |
| ![Settings Modal](docs/screenshots/settings.png) | Settings modal with theme toggle |

## Installation

### Prerequisites

- Windows 10/11
- WebView2 Runtime (included in Windows 11; [download for Windows 10](https://developer.microsoft.com/en-us/microsoft-edge/webview2/))

### Download

Download the latest release from [GitHub Releases](https://github.com/keypilot/keypilot/releases):

- `KeyPilot_0.1.0_x64-setup.exe` — NSIS installer
- `KeyPilot_0.1.0_x64.msi` — MSI installer

### Build from Source

```bash
# Clone
git clone https://github.com/keypilot/keypilot.git
cd keypilot

# Install frontend dependencies
cd webui && pnpm install && cd ..

# Build release
pnpm tauri build
```

## Development

```bash
# Install frontend dependencies
cd webui && pnpm install && cd ..

# Run in development mode (hot reload)
pnpm tauri dev

# Type check (Rust)
cargo check --manifest-path src-tauri/Cargo.toml

# Type check (TypeScript)
cd webui && pnpm tsc --noEmit
```

## V0.1 Limitations

| Feature | Status | Notes |
|---------|--------|-------|
| Encryption | ❌ Not encrypted | Plaintext SQLite + Windows ACL; V0.2 RFC evaluates SQLCipher / master password / DPAPI |
| Cross-platform | ❌ Windows only | Mac/Linux deferred to V0.3+ |
| Auto-refresh quota | ❌ Manual | V0.2 |
| Low quota alerts | ❌ None | V0.2 |
| Import/Export | ❌ None | V0.2 |
| Sync | ❌ None | V0.2 |

## Architecture

```
keypilot-dev/
├── src-tauri/           # Rust backend (Tauri 2)
│   └── src/
│       ├── main.rs      # Entry point
│       ├── lib.rs       # App setup + IPC handlers + watcher spawn
│       ├── database.rs  # SQLite schema v6 (8 tables) + cursor CRUD
│       ├── provider/    # 5 preset adapters
│       ├── services/    # Token usage + agent parsers + incremental_import (file watcher)
│       ├── commands/    # IPC command handlers (incl. force_rescan_all)
│       └── tray.rs      # System tray integration
├── webui/               # React frontend
│   └── src/
│       ├── components/  # UI components (incl. UsageHeatmapCalendar, UsageKpiCards, UsageTimeSeries)
    │       ├── hooks/       # TanStack Query hooks + useUsageTick (real-time event listener)
    │       └── lib/api.ts   # Tauri IPC invoke wrappers
    └── docs/
    ├── screenshots/    # App screenshots
    ├── quota-token-reference.md  # Quota + token usage 架构 / schema / IPC 参考
    └── v0.1-acceptance.md  # Acceptance checklist
```

## Acknowledgements

- [token-monitor](https://github.com/Javis603/token-monitor) — token usage 解析逻辑(client 归一化、total_tokens 公式、时间戳处理)参考
- **[cc-switch](https://github.com/JasonYoung04/cc-switch)** — coding plan quota query architecture inspired by their per-provider dispatcher pattern (MIT, Copyright 2025 Jason Young). See [LICENSE](docs/third-party/cc-switch.LICENSE).

## License

MIT — see [LICENSE](LICENSE) (待添加).
