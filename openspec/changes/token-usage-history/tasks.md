# Token Usage History — Tasks

## T5.1 数据层 (database.rs)

- [ ] 在 `migrate()` 添加 version bump (v3 → v4)
- [ ] 创建 `token_usage_records` 表 + 2 索引
- [ ] 创建 `daily_agent_model_usage` 归约表
- [ ] 创建 `daily_model_usage` 归约表
- [ ] `src-tauri/tests/ipc_e2e.rs` 新增 token_usage 测试 (T5.1.1 insert + query + rollup)

## T5.2 Service 层 (services/token_usage.rs)

- [ ] 定义 `UsageRecordInput`, `UsageRecord`, `UsageFilter`, `UsageSummary`, `UsageSummaryAgentPair`, `DailySeries`, `ImportResult` struct
- [ ] `record_usage()` — insert + upsert rollups (async transaction)
- [ ] `list_records()` — WHERE + LIMIT + OFFSET
- [ ] `get_summary()` — agent pair 条形图数据 + daily series
- [ ] `import_jsonl()` — 逐行解析,agent_type 推断,去重
- [ ] `import_csv()` — 逐行解析,header mapping
- [ ] 错误处理: `AppError::TokenUsage(InvalidFormat, Duplicate, PricingNotFound)`

## T5.3 定价表 (services/pricing.rs + data/pricing.json)

- [ ] 创建 `data/pricing.json` (Top 50 models, LiteLLM 源)
- [ ] `services/pricing.rs`: `PricingService::lookup(model) -> Option<&PricingEntry>`
- [ ] `cost_details` 计算: 5 token types × per-token rate → sum
- [ ] `pricing_version` = 内置 JSON 的 version 字段

## T5.4 IPC 层 (commands/token_usage.rs + lib.rs)

- [ ] `commands/token_usage.rs`: 5 个 handler
- [ ] `lib.rs` 注册: `record_usage`, `list_usage_records`, `get_usage_summary`, `import_usage`, `get_pricing`
- [ ] `src-tauri/tests/ipc_e2e.rs` 新增 5 个 IPC test

## T5.5 前端 — API 层

- [ ] `webui/src/types/api.ts` 新增 `UsageRecord`, `UsageSummary`, `ImportResult` 类型
- [ ] `webui/src/lib/api.ts` 新增 5 个 `invoke` 调用

## T5.6 前端 — Hooks

- [ ] `webui/src/hooks/useUsage.ts`:
  - `useUsageSummary(filter)` → UsageSummary
  - `useUsageRecords(filter)` → PaginatedResponse
  - `useImportUsage()` → mutation
  - `usePricing()` → Vec<PricingEntry>

## T5.7 前端 — 组件

- [ ] `webui/src/pages/UsagePage.tsx` — 主页面,tab 结构
- [ ] `webui/src/components/AgentPairChart.tsx` — 水平条形图
- [ ] `webui/src/components/UsageTimeSeries.tsx` — 折线图
- [ ] `webui/src/components/UsageHeatmap.tsx` — 小时 × agent grid
- [ ] `webui/src/components/UsageDetailPanel.tsx` — 点击 bar 后 detail
- [ ] `webui/src/components/ImportModal.tsx` — JSONL/CSV 导入 dialog

## T5.8 路由 + 集成

- [ ] `App.tsx` 新增 `/usage` route
- [ ] 侧边栏或 tab bar 加入 "用量历史" 入口

## 验收标准

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` PASS
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` PASS
- [ ] `cd webui && pnpm tsc --noEmit` PASS
- [ ] `pnpm build` PASS
- [ ] 导入 100 条 JSONL → `get_usage_summary` 返回正确 agent pair 聚合
- [ ] 删除 90 天前数据 → `SELECT COUNT(*)` 确认过期行已删
- [ ] 无定价 model → cost = null, 前端显示 "—"
