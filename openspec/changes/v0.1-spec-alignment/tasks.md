# V0.1 Spec Alignment — Tasks

> Stage 1-4 任务清单 + 验证清单。每项独立可勾选。
> 引用:`spec.md` REQ-XXX-NNN / `design.md` §N

---

## Stage 1: Database Schema

- [ ] **T1.1** `database.rs` 实现 `open()` 启动检查 `schema_version` 框架
  - 读: `SELECT value FROM meta WHERE key = 'schema_version'`
  - 缺省视为 v1
  - 引用:`design.md §5`
- [ ] **T1.2** v1 → v2 migration SQL
  ```sql
  ALTER TABLE providers ADD COLUMN is_preset INTEGER NOT NULL DEFAULT 0;
  UPDATE meta SET value = '2' WHERE key = 'schema_version';
  ```
  - 引用:`spec.md REQ-SCHEMA-001`
- [ ] **T1.3** `database.rs` 加 `set_meta(key, value)` / `get_meta(key) -> Option<String>` 方法
  - 给 preset_seeded flag 用
- [ ] **T1.4** `NewProviderRow` 加 `is_preset: i32` 字段(默认 0)
- [ ] **T1.5** `Database::insert_provider()` 写入 is_preset 字段
- [ ] **T1.6** `Database::list_providers()` SELECT 返回 is_preset

## Stage 2: Provider Adapters

- [ ] **T2.1** `provider/mod.rs`:`ProviderKind` enum 改为 `OpenAI / Anthropic / DeepSeek / Custom`(4 变体)
  - 移除:Gemini / Mistral / Moonshot / Zhipu 5 种
  - 引用:`spec.md REQ-PROV-001`
- [ ] **T2.2** `provider/adapter.rs`:`ValidateError` enum 完整定义
  ```rust
  enum ValidateError {
      InvalidKey,
      Ambiguous(String),  // Anthropic 400 case
      Network(String),
  }
  ```
  - 引用:`spec.md REQ-PROV-004`
- [ ] **T2.3** `provider/adapter.rs`:`ProviderAdapter` trait 加 `validate_key()` 方法(abstract)
- [ ] **T2.4** `provider/adapter.rs`:`QuotaError` enum 加 `Unsupported(String)` 变体
  - 给 Anthropic 用
- [ ] **T2.5** `provider/openai.rs`:`OpenAIAdapter` 实现 `validate_key()`(GET /v1/models)
  - 引用:`design.md §1` / `spec.md REQ-PROV-004`
- [ ] **T2.6** `provider/deepseek.rs`:`DeepSeekAdapter` 实现 `validate_key()`(GET /user/balance)
  - 引用:`design.md §2` / `spec.md REQ-PROV-004`
- [ ] **T2.7** `provider/anthropic.rs`:`AnthropicAdapter` 实现 `validate_key()`(POST /v1/messages,400 ambiguous)
  - 引用:`design.md §3` / `spec.md REQ-PROV-004`
- [ ] **T2.8** `provider/anthropic.rs`:`fetch_quota` 返回 `QuotaError::Unsupported`
  - 引用:`design.md §3` / `spec.md REQ-QUOTA-003`
- [ ] **T2.9** `services/provider.rs`:`init_default_providers()` 函数
  - 检查 `meta.preset_seeded == "1"` 早退
  - 3 条 preset INSERT
  - 写 `meta.preset_seeded = "1"`
  - 引用:`design.md §4` / `spec.md REQ-PROV-002`
- [ ] **T2.10** `services/provider.rs`:`add_provider()` 用 uuid v4 生成 id
  - 引用:`design.md §4`
- [ ] **T2.11** `lib.rs::run()` 调用 `init_default_providers()` 在 db open 之后、commands 注册之前
  - 引用:`design.md §4` (调用顺序)
- [ ] **T2.12** `Cargo.toml` 加 `uuid` 依赖(v4 + serde)

## Stage 3: UI

- [ ] **T3.1** `components/ProviderList.tsx`:preset 行前显示 "📌 官方" 徽章
  - 条件:`provider.is_preset === 1`
  - 引用:`design.md §4` / `spec.md REQ-PROV-002`
- [ ] **T3.2** `components/ProviderList.tsx`:`provider` 类型加 `is_preset: number` 字段
- [ ] **T3.3** `components/CopyButton.tsx`:**只复制明文 key / 明文 key + base_url 合并**
  - **不实现** format dropdown(SPEC 砍)
  - 引用:`spec.md REQ-COPY-001` / REMOVED `REQ-COPY-002`
- [ ] **T3.4** `lib/format.ts`:**整个文件删除或大幅简化**
  - 原计划:`ProviderKind → EnvVarSchema` 映射 + 4 种 shell format
  - 改为:无 / 或保留为 V0.2 备用
  - 引用:`spec.md REQ-COPY-002` REMOVED

## Stage 4: Quota Query

- [ ] **T4.1** `services/quota.rs`:3 个 command 框架
  ```rust
  pub async fn fetch_one(db: &Database, provider_id: i64) -> Result<QuotaSnapshot, AppError>
  pub async fn fetch_all(db: &Database) -> Result<Vec<QuotaSnapshot>, AppError>
  pub async fn get_cache(db: &Database, provider_id: i64) -> Result<Option<QuotaSnapshot>, AppError>
  ```
- [ ] **T4.2** `commands/quota.rs`:3 个 Tauri command 包装
  - `fetch_quota(provider_id)`
  - `fetch_all_quotas()`
  - `get_quota_cache(provider_id)`
- [ ] **T4.3** `provider/openai.rs`:`fetch_quota` 实现(subscription + usage 3-month iteration)
  - 引用:`design.md §1` / `spec.md REQ-QUOTA-001`
- [ ] **T4.4** `provider/deepseek.rs`:`fetch_quota` 实现(GET /user/balance)
  - 引用:`design.md §2` / `spec.md REQ-QUOTA-002`
- [ ] **T4.5** `provider/anthropic.rs`:`fetch_quota` 已返回 Unsupported(Stage 2 写)
- [ ] **T4.6** `services/quota.rs`:每次 fetch_quota 后 upsert 到 `quota_cache` 表
- [ ] **T4.7** `services/quota.rs`:catch `QuotaError::Unsupported` → 标记 `source: Manual` 的 fallback 路径
  - Anthropic 收到 Unsupported,UI 走"手动输入"modal(Stage 7)
- [ ] **T4.8** `Cargo.toml` 加 `reqwest` + `serde_json` 依赖(Stage 2 也要)

---

## 验证(Stage DoD)

### V1 — 编译

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` 通过
- [ ] `cargo build --manifest-path src-tauri/Cargo.toml` 通过(release 也可,debug 默认)
- [ ] `cd webui && pnpm tsc --noEmit` 通过(Stage 3+)

### V2 — 硬约束(§3.1 / §3.2 / §3.3)

- [ ] `grep "api_key TEXT" src-tauri/src/database.rs` 有结果(明文确认)
- [ ] `grep "is_preset" src-tauri/src/database.rs` 有结果(新列)
- [ ] `grep -E "argon2|chacha20|ChaCha20Poly1305|aes-gcm|sodiumoxide|^age " src-tauri/Cargo.toml` 为空
- [ ] `grep -rn "fs::write|fs::create_dir_all" src-tauri/src/` 路径不在 `~/.claude/` / `~/.codex/` / `~/.config/opencode/` 下
- [ ] `grep "preset_seeded" src-tauri/src/services/provider.rs` 有结果(seed flag)
- [ ] `grep -rn "TODO\|FIXME\|todo!()\|unimplemented!()" src-tauri/src/` 无残留(§13.1 铁律)

### V3 — 行为

- [ ] 首次启动 SQLite 文件 `%APPDATA%\com.keypilot.app\keypilot.db` 存在
- [ ] `sqlite3 keypilot.db ".tables"` 返回 `providers` / `quota_cache` / `meta`
- [ ] `SELECT * FROM meta;` 返回 `schema_version = 2`,`preset_seeded = 1`
- [ ] `SELECT * FROM providers;` 返回 3 条 preset(OpenAI / Anthropic / DeepSeek),`is_preset=1`
- [ ] 删一条 preset(`DELETE FROM providers WHERE id='preset-openai'`)→ 重启应用 → 不重建(尊重用户意图)
- [ ] 添加同 kind custom provider → 数据库有 4 条(3 preset + 1 custom)
- [ ] `SELECT id, is_preset FROM providers WHERE kind='openai';` 至少返回 2 条(preset + custom),id 不同

### V4 — Quota

- [ ] OpenAI provider 配真 key → 触发 fetch_quota → QuotaSnapshot 有真实 total/used/remaining(USD)
- [ ] DeepSeek provider 配真 key → fetch_quota → QuotaSnapshot 有 remaining
- [ ] Anthropic provider 配真 key → fetch_quota → 收到 QuotaError::Unsupported,UI 走"手动输入"路径

### V5 — Validate

- [ ] OpenAI 配错 key → validate_key 返回 InvalidKey
- [ ] Anthropic 配错 key → validate_key 返回 InvalidKey
- [ ] OpenAI 配真 key → validate_key 返回 Ok

---

## 引用

| 文件 | 用于 |
|---|---|
| `references/tier2-tech/openai-balance/cmd/root.go` | OpenAI quota 算法 |
| `references/tier1-direct/cc-switch/src-tauri/src/services/balance.rs` | DeepSeek quota 算法 |
| `references/tier1-direct/cc-switch/src-tauri/src/database/dao/providers.rs` | Preset seed / duplicate 模式 |
| `references/tier1-direct/cc-switch/src-tauri/src/lib.rs` | 启动流程(参考 init_default_official_providers 调用时机) |
| `PM思考工厂/keypilot/codemap.md` | 跨项目对照表 + 参考项目结构 |

---

## 任务依赖图

```
T1.1 ──→ T1.2 ──→ T1.3 ──→ T1.4 ──→ T1.5 ──→ T1.6
                                         │
                                         ↓
T2.1 ──→ T2.2 ──→ T2.3 ──→ T2.4 ──┬──→ T2.5 ──→ T2.6 ──→ T2.7
                                    │            │
                                    │            ↓
                                    └──→ T2.8 ──→ T2.9 ──→ T2.10 ──→ T2.11
                                                                                │
                                                                                ↓
                                                                       T2.12 (Cargo.toml)
                                                                                │
                                                                                ↓
                                                                       T3.1 ──→ T3.2 ──→ T3.3 ──→ T3.4
                                                                                                          │
                                                                                                          ↓
                                                                                                         T4.1 ──→ T4.2
                                                                                                                    │
                                                                                                          ┌─────────┼─────────┐
                                                                                                          ↓         ↓         ↓
                                                                                                         T4.3     T4.4     T4.5
                                                                                                          │         │         │
                                                                                                          └─────────┼─────────┘
                                                                                                                    ↓
                                                                                                         T4.6 ──→ T4.7 ──→ T4.8
```

可并行 batch:
- T1.1-T1.6 串行
- T2.1-T2.4 串行(共享 trait/enum 定义)
- T2.5-T2.7 可并行(三个独立 adapter 文件)
- T3.1-T3.4 串行(依赖 schema)
- T4.3-T4.5 可并行(三 provider quota)
