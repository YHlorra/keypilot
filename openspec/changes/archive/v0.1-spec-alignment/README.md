# v0.1-spec-alignment (Archived)

## Status
Superseded by `v0.1-general-credentials` on 2026-06-24. Archived on 2026-06-25.

## Why archived
The 11 V0.1 spec decisions in this change were carried forward into `v0.1-general-credentials`, which expanded scope to a general credentials vault (AI + DB + Dev). v0.1-spec-alignment is retained for historical reference only.

## Original scope
- ProviderKind 3 + Custom enum
- OpenAI quota algorithm (openai-balance port: subscription + 3-month usage iteration, cents to USD)
- DeepSeek quota algorithm (cc-switch port: GET /user/balance)
- Anthropic quota Unsupported
- validate_key three-state error (InvalidKey / Ambiguous / Network)
- Provider duplicates allowed
- 3 preset seed + `is_preset` flag
- schema v1 to v2 migration

## Implementation
Captured in `feature_list.json` stages 1-4 and applied 2026-06-24.
