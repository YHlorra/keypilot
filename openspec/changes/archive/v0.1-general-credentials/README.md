# v0.1-general-credentials (Archived)

## Status
Spec applied 2026-06-24. Implementation complete across stages 1-9. Archived on 2026-06-25.

## Why archived
V0.1 general credentials vault is the project's foundation. All 5 preset seeds (OpenAI, DeepSeek, Anthropic, GitHub, PostgreSQL), visibility (visible / masked), 3 themes (Auto / Light / Dark), and shadcn/ui stack are in production. The spec is locked; future iterations build on this foundation.

## Original scope
- 5 SQLite tables: categories, providers, provider_fields, quota_cache, meta
- 5 preset seeds: OpenAI / DeepSeek / Anthropic / GitHub / PostgreSQL
- 12 IPC commands: list_providers / get_provider / add_provider / update_provider / delete_provider / list_categories / add_category / delete_category / test_connection / fetch_quota / get_theme / set_theme
- 3 themes (Auto / Light / Dark) with Radix UI Colors
- shadcn/ui + Radix Primitives + Tailwind utility stack
- Detail + Tray dual quota display (single quota_cache data source)
- 20 REQ total (11 ADDED + 5 MODIFIED + 4 REMOVED)

## Implementation
Captured in `feature_list.json` stages 1-9. Released as V0.1 (sign-off 2026-06-25 in stage-9).
