# Third-Party Licenses

KeyPilot V0.1 bundles or depends on the following third-party assets and
projects. This file documents attribution and license obligations.

## Brand logos (provider preset icons)

Files: `webui/public/icons/providers/*.svg`
Source: simple-icons project (https://github.com/simple-icons/simple-icons)
License: CC0 1.0 Universal (public domain dedication)

The provider preset icons in `webui/public/icons/providers/` are unmodified
SVG vectors copied from the simple-icons project, which dedicates its
icon set to the public domain under CC0 1.0. The icons represent the
brand identity of each service; the brand names and trademarks remain
the property of their respective owners and are used nominatively here
solely to identify which credentials correspond to which service.

If you fork or redistribute KeyPilot, no attribution is legally required
for the icon SVGs themselves (CC0), but trademark courtesy still applies:
do not suggest endorsement by the trademark holders.

## Provider preset schema

The `base_url` / `api_key` / `visibility` shape was inspired by
`farion1231/cc-switch` (MIT). No code or assets were copied.

## cc-switch

- **Source**: https://github.com/JasonYoung04/cc-switch (commit hash to be filled)
- **License**: MIT (Copyright 2025 Jason Young)
- **Files in keypilot**: `src-tauri/src/provider/coding_plan/*.rs` (design pattern adapted, no source code copied)
- **License copy**: [cc-switch.LICENSE](docs/third-party/cc-switch.LICENSE)

The coding plan quota framework (subscription quota model + dispatcher +
provider-specific query implementations) in `src-tauri/src/provider/coding_plan/`
takes design inspiration from cc-switch's `services/coding_plan.rs` and
`services/subscription.rs`. The data model (`SubscriptionQuota`, `QuotaTier`,
`CredentialStatus`, `parse_minimax_tiers` style parsing helpers) follows the
same shape. KeyPilot's implementation is original; no Rust source was copied.
The MIT license text is reproduced verbatim in
[`docs/third-party/cc-switch.LICENSE`](docs/third-party/cc-switch.LICENSE)
to satisfy the upstream license's attribution clause.