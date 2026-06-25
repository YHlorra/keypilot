# KeyPilot V0.1

> AI 时代的钥匙管理员 — 本地凭证库 + 额度查询
> Windows 优先 · Rust + Tauri 2 · MIT 协议

## Features

- **5 Preset Providers**: OpenAI / DeepSeek / Anthropic / GitHub / PostgreSQL
- **3 Themes**: Dark / Light / Follow System (Radix UI Colors)
- **CopyButton 3-state**: visible → masked → revealed
- **Detail + Tray Quota**: Single quota_cache data source, 5-min staleTime
- **Plaintext Storage**: V0.1 不加密 (明文 SQLite + Windows ACL)

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
│       ├── lib.rs       # App setup + IPC handlers
│       ├── database.rs  # SQLite schema v3 + preset seed
│       ├── provider/    # 5 preset adapters
│       ├── commands/    # IPC command handlers
│       └── tray.rs      # System tray integration
├── webui/               # React frontend
│   └── src/
│       ├── components/  # UI components
│       ├── hooks/       # TanStack Query hooks
│       └── lib/api.ts   # Tauri IPC invoke wrappers
└── docs/
    ├── screenshots/    # App screenshots
    └── v0.1-acceptance.md  # Acceptance checklist
```

## License

MIT — see [LICENSE](LICENSE) (待添加).
