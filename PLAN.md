# token-usage-b — Stage B: Business Logic (TokenUsageService)

> Worktree: `.claude/worktrees/token-usage-b`
> Branch: `token-usage-b`
> 上游: `main` (c729670)
> 前提: Stage A (token-usage-a) 合并后

---

## 1. 目标

实现 `TokenUsageService`：纯 Rust 业务逻辑，零 Tauri/IPC 依赖。消费 Stage A 的 Database + PricingService。

## 2. 先读这些文件

| 文件 | 为什么 |
|---|---|
| `src-tauri/src/services/provider.rs` | Service 层模式参考（spawn_blocking + Arc<Mutex> + 事务 + 错误映射） |
| `src-tauri/src/services/pricing.rs` | Stage A 产物，cost 计算逻辑 |
| `src-tauri/src/database.rs` | Stage A 产物，3 张新表 schema |
| `src-tauri/src/error.rs` | Stage A 扩展后的 AppError 变体 |
| `openspec/changes/token-usage-history/spec.md` | REQ-TOKEN-001 (数据模型) + REQ-TOKEN-002 (IPC 接口 = service 契约) |
| `openspec/changes/token-usage-history/design.md` | §2.2 Service 层方法签名 + 归约刷新逻辑 |
| `openspec/changes/token-usage-history/tasks.md` | T5.2 |

## 3. 完成目标

### 3.1 `src-tauri/src/services/token_usage.rs` (T5.2)

定义以下 struct + impl：

**Input/Output types:**
- `UsageRecordInput` — 单条记录输入 (provider, model, agent_type, usage_details, ...)
- `UsageRecord` — 完整记录输出 (含 id, occurred_at, cost_details, ...)
- `UsageFilter` — 查询过滤 (date_from?, date_to?, agent_type?, model?, status?)
- `UsageSummary` — 聚合结果 (total_tokens, total_cost_usd, total_requests, agent_pairs[], daily_series[])
- `UsageSummaryAgentPair` — agent pair 条形图条目
- `DailySeries` — 日粒度序列
- `ImportResult` — 导入结果 (imported, skipped, errors)

**6 个方法:**
- [ ] `record_usage(req: UsageRecordInput) -> Result<UsageRecord, AppError>` — insert + 同步 upsert rollups
- [ ] `list_records(filter: UsageFilter, page, per_page) -> Result<Vec<UsageRecord>, AppError>` — WHERE + LIMIT + OFFSET
- [ ] `get_summary(filter: UsageFilter) -> Result<UsageSummary, AppError>` — agent pair 条形图 + daily series
- [ ] `import_jsonl(content: &str, source_hint: Option<&str>) -> Result<ImportResult, AppError>` — 逐行解析, agent_type 推断, 去重
- [ ] `import_csv(content: &str) -> Result<ImportResult, AppError>` — 逐行解析, header mapping
- [ ] `refresh_daily_rollups(date: &str) -> Result<(), AppError>` — bulk refresh 归约表

**关键逻辑:**
- `record_usage` 后同步 upsert 到 `daily_agent_model_usage` + `daily_model_usage`
- 导入时按 `(agent_type, model, occurred_at, input_tokens, output_tokens)` 五元组去重
- 无定价 model → cost_details = null
- JSONL 解析: Claude Code 格式 (`usage.input_tokens`) + Codex 格式 (`usage.prompt_tokens`) 双支持

### 3.2 错误处理

使用 Stage A 已注册的 `AppError::TokenUsage(InvalidFormat, Duplicate, PricingNotFound)`。

## 4. 验收标准

```bash
cargo check --manifest-path src-tauri/Cargo.toml      # PASS
cargo test --manifest-path src-tauri/Cargo.toml       # PASS
```

单元测试覆盖: record_usage happy path + cost 计算 + import_jsonl dedup + import_csv parse + get_summary aggregation。

## 5. 约束

- 纯 Rust，不 import tauri（零 IPC 依赖）
- Service 持有 `Database` 引用（通过构造函数传入，不通过 State）
- 所有 fallible 函数返回 `Result<T, AppError>`
- 事务化多表写入 (`tx.commit()` 防半写)
